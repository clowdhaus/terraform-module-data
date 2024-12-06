use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
struct Response {
  data: Data,
  included: Vec<Included>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Data {
  #[serde(rename = "type")]
  dtype: String,
  id: String,
  attributes: Attributes,
  relationships: serde_json::Value,
  links: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Attributes {
  downloads: u64,
  full_name: String,
  name: String,
  namespace: String,
  owner_name: String,
  provider_logo_url: String,
  provider_name: String,
  source: String,
  verified: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct Included {
  #[serde(rename = "type")]
  itype: String,
  id: String,
  attributes: IncludedAttributes,
  links: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
struct IncludedAttributes {
  created_at: String,
  description: String,
  downloads: u64,
  published_at: String,
  source: String,
  updated_at: String,
  version: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Summary {
  downloads: u64,
  major_version: String,
  created_at: String,
}

impl Response {
  fn summarize(&self) -> Result<BTreeMap<String, Summary>> {
    let mut summary: BTreeMap<String, Summary> = BTreeMap::new();
    for i in self.included.iter() {
      let mut ver = i.attributes.version.split('.');
      let major_version = ver.next().unwrap().to_string();
      let minor_version = ver.next().unwrap();
      let patch_version = ver.next().unwrap();
      let key = format!("{:02}", major_version.parse::<u64>().unwrap_or(0));

      if major_version == "0" {
        continue;
      }

      let record = summary.entry(key).or_insert(Summary {
        downloads: 0,
        major_version,
        created_at: "".to_string(),
      });

      record.downloads += i.attributes.downloads;

      if minor_version == "0" && patch_version == "0" {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&i.attributes.created_at)?;
        record.created_at = timestamp.date_naive().to_string();
      }
    }

    Ok(summary)
  }

  fn write(&self, path: PathBuf, data: BTreeMap<String, Summary>) -> Result<()> {
    std::fs::create_dir_all(&path)?;

    let data = data.into_values().collect::<Vec<Summary>>();
    let utc: DateTime<Utc> = Utc::now();
    let file = path.join(format!("{}.json", utc.format("%Y-%m-%d")));
    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(file, json)?;

    Ok(())
  }
}

async fn get(module: &str) -> Result<Response> {
  let url = Url::parse_with_params(
    format!("https://registry.terraform.io/v2/modules/terraform-aws-modules/{module}/aws").as_str(),
    &[("include", "module-versions")],
  )?;

  let resp = Client::builder()
    .user_agent("Module Download Data")
    .build()?
    .get(url)
    .send()
    .await?;

  let response: Response = resp.json().await?;
  Ok(response)
}

pub async fn collect(path: &Path, module: &str) -> Result<()> {
  // Terraform registry data
  let registry = get(module).await?;
  let registry_path = path.join("registry").join(module.to_lowercase());
  registry.write(registry_path, registry.summarize()?)?;

  Ok(())
}

type Module = String;
type ModuleData = BTreeMap<Module, Vec<crate::graph::TraceData>>;
type ModuleMajorVersion = String;
type DownloadDates = Vec<NaiveDate>;
type DownloadCounts = Vec<u64>;

fn collect_trace_data(data_path: &Path) -> Result<ModuleData> {
  let mut data = ModuleData::new();

  for entry in fs::read_dir(data_path.join("registry"))? {
    let mod_path = entry?.path();
    let module = mod_path.file_stem().unwrap().to_str().unwrap().to_owned();

    let traces = get_module_data_traces(&mod_path)?;
    data.insert(module, traces);
  }

  Ok(data)
}

fn get_module_data_traces(mod_path: &Path) -> Result<Vec<crate::graph::TraceData>> {
  let mut aggregate: BTreeMap<ModuleMajorVersion, (DownloadDates, DownloadCounts)> = BTreeMap::new();

  for fentry in fs::read_dir(mod_path)? {
    let file_path = fentry?.path();
    let file_name = file_path.file_stem().unwrap().to_str().unwrap();
    let file_data = fs::read_to_string(&file_path)?;
    let summary = serde_json::from_str::<Vec<Summary>>(&file_data)?;

    let timestamp = NaiveDate::parse_from_str(&file_name.replace(".json", ""), "%Y-%m-%d")?;

    for sum in summary.iter() {
      let version = &sum.major_version;
      let downloads = sum.downloads;

      aggregate
        .entry(version.to_string())
        .and_modify(|(dates, counts)| {
          dates.push(timestamp);
          counts.push(downloads);
        })
        .or_insert((vec![timestamp], vec![downloads]));
    }
  }

  let mut traces = Vec::new();
  for (version, (dates, downloads)) in aggregate.into_iter() {
    if mod_path.ends_with("eks") && version.parse::<i32>()? < 16 {
      tracing::error!("Skipping {mod_path:#?} version {version}");
      // Not interested in EKS versions older than 16
      continue;
    }
    traces.push(crate::graph::TraceData {
      name: format!("v{version}.0"),
      x_data: dates,
      y_data: downloads.iter().map(|d| d.to_string()).collect(),
    });
  }

  Ok(traces)
}

fn graph_downloads(timestamp: &str, data_path: &Path) -> Result<()> {
  let title = "Terraform Registry Downloads";
  let tdata = crate::registry::collect_trace_data(data_path)?;
  let mut body = String::new();

  for (module, traces) in tdata.into_iter() {
    body.push_str(&format!("## {module}\n\n"));

    let html_title = format!("{module} - {title}");
    let titles = crate::graph::Titles {
      title: html_title.clone(),
      x_title: "Date".to_string(),
      y_title: "Total Downloads".to_string(),
    };

    info!("Plotting {} time series", html_title);

    let plot = crate::graph::plot_time_series(&html_title, traces, titles, plotly::common::Mode::Markers)?;

    body.push_str(plot.as_str());
    body.push_str("\n\n");
  }

  let tpl_path = PathBuf::from("src").join("templates").join("registry-downloads.tpl");
  let tpl = fs::read_to_string(tpl_path)?;
  let out_path = PathBuf::from("docs").join("registry-downloads.md");

  let rendered = tpl.replace("{{ body }}", &body).replace("{{ date }}", timestamp);
  fs::write(out_path, rendered).map_err(Into::into)
}

/// Graph the data collected and insert into mdbook docs
pub(crate) fn graph(data_path: &Path) -> Result<()> {
  let timestamp = chrono::Local::now().to_utc().format("%Y-%m-%d %H:%M:%S").to_string();

  graph_downloads(&timestamp, data_path)?;

  Ok(())
}
