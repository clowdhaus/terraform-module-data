use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
};

use anyhow::{bail, Result};
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
      let major_version = ver.next().ok_or_else(|| anyhow::anyhow!("Invalid version format"))?.to_string();
      let minor_version = ver.next().ok_or_else(|| anyhow::anyhow!("Invalid version format"))?;
      let patch_version = ver.next().ok_or_else(|| anyhow::anyhow!("Invalid version format"))?;
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

  if !resp.status().is_success() {
    bail!("Registry API returned {}", resp.status());
  }
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

fn collect_trace_data(data_path: &Path) -> Result<ModuleData> {
  let mut data = ModuleData::new();

  for entry in fs::read_dir(data_path.join("registry"))? {
    let mod_path = entry?.path();
    let module = mod_path
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", mod_path))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", mod_path))?
      .to_owned();

    let traces = get_module_data_traces(&mod_path)?;
    data.insert(module, traces);
  }

  Ok(data)
}

fn get_module_data_traces(mod_path: &Path) -> Result<Vec<crate::graph::TraceData>> {
  let module_name = mod_path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("");

  let mut aggregate: BTreeMap<String, (Vec<NaiveDate>, Vec<u64>)> = BTreeMap::new();

  for fentry in fs::read_dir(mod_path)? {
    let file_path = fentry?.path();
    let file_name = file_path
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", file_path))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", file_path))?;
    let file_data = fs::read_to_string(&file_path)?;
    let summary = serde_json::from_str::<Vec<Summary>>(&file_data)?;

    let timestamp = NaiveDate::parse_from_str(file_name, "%Y-%m-%d")?;

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
    // EKS module versions before v16 used a fundamentally different architecture
    // and are not meaningful for download trend comparison
    if module_name == "eks" && version.parse::<i32>().unwrap_or(0) < 16 {
      tracing::debug!("Skipping {mod_path:#?} version {version} (pre-v16 EKS)");
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_summarize_basic() {
    let response = Response {
      data: Data {
        dtype: "modules".to_string(),
        id: "test-id".to_string(),
        attributes: Attributes {
          downloads: 1000,
          full_name: "terraform-aws-modules/vpc/aws".to_string(),
          name: "vpc".to_string(),
          namespace: "terraform-aws-modules".to_string(),
          owner_name: "terraform-aws-modules".to_string(),
          provider_logo_url: "".to_string(),
          provider_name: "aws".to_string(),
          source: "".to_string(),
          verified: true,
        },
        relationships: serde_json::json!({}),
        links: serde_json::json!({}),
      },
      included: vec![
        Included {
          itype: "module-versions".to_string(),
          id: "1".to_string(),
          attributes: IncludedAttributes {
            created_at: "2023-01-01T00:00:00Z".to_string(),
            description: "v1.0.0".to_string(),
            downloads: 500,
            published_at: "2023-01-01T00:00:00Z".to_string(),
            source: "".to_string(),
            updated_at: "2023-01-01T00:00:00Z".to_string(),
            version: "1.0.0".to_string(),
          },
          links: serde_json::json!({}),
        },
        Included {
          itype: "module-versions".to_string(),
          id: "2".to_string(),
          attributes: IncludedAttributes {
            created_at: "2023-06-01T00:00:00Z".to_string(),
            description: "v1.1.0".to_string(),
            downloads: 300,
            published_at: "2023-06-01T00:00:00Z".to_string(),
            source: "".to_string(),
            updated_at: "2023-06-01T00:00:00Z".to_string(),
            version: "1.1.0".to_string(),
          },
          links: serde_json::json!({}),
        },
        Included {
          itype: "module-versions".to_string(),
          id: "3".to_string(),
          attributes: IncludedAttributes {
            created_at: "2024-01-01T00:00:00Z".to_string(),
            description: "v2.0.0".to_string(),
            downloads: 200,
            published_at: "2024-01-01T00:00:00Z".to_string(),
            source: "".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
            version: "2.0.0".to_string(),
          },
          links: serde_json::json!({}),
        },
      ],
    };

    let summary = response.summarize().unwrap();
    // v1 should aggregate downloads: 500 + 300 = 800
    assert_eq!(summary.get("01").unwrap().downloads, 800);
    assert_eq!(summary.get("01").unwrap().created_at, "2023-01-01");
    // v2 should have 200 downloads
    assert_eq!(summary.get("02").unwrap().downloads, 200);
    assert_eq!(summary.get("02").unwrap().created_at, "2024-01-01");
  }

  #[test]
  fn test_summarize_skips_v0() {
    let response = Response {
      data: Data {
        dtype: "modules".to_string(),
        id: "test-id".to_string(),
        attributes: Attributes {
          downloads: 100,
          full_name: "test/mod/aws".to_string(),
          name: "mod".to_string(),
          namespace: "test".to_string(),
          owner_name: "test".to_string(),
          provider_logo_url: "".to_string(),
          provider_name: "aws".to_string(),
          source: "".to_string(),
          verified: false,
        },
        relationships: serde_json::json!({}),
        links: serde_json::json!({}),
      },
      included: vec![Included {
        itype: "module-versions".to_string(),
        id: "1".to_string(),
        attributes: IncludedAttributes {
          created_at: "2023-01-01T00:00:00Z".to_string(),
          description: "v0.1.0".to_string(),
          downloads: 50,
          published_at: "2023-01-01T00:00:00Z".to_string(),
          source: "".to_string(),
          updated_at: "2023-01-01T00:00:00Z".to_string(),
          version: "0.1.0".to_string(),
        },
        links: serde_json::json!({}),
      }],
    };

    let summary = response.summarize().unwrap();
    assert!(summary.is_empty(), "v0 versions should be skipped");
  }

  #[test]
  fn test_summarize_invalid_version() {
    let response = Response {
      data: Data {
        dtype: "modules".to_string(),
        id: "test-id".to_string(),
        attributes: Attributes {
          downloads: 0,
          full_name: "test/mod/aws".to_string(),
          name: "mod".to_string(),
          namespace: "test".to_string(),
          owner_name: "test".to_string(),
          provider_logo_url: "".to_string(),
          provider_name: "aws".to_string(),
          source: "".to_string(),
          verified: false,
        },
        relationships: serde_json::json!({}),
        links: serde_json::json!({}),
      },
      included: vec![Included {
        itype: "module-versions".to_string(),
        id: "1".to_string(),
        attributes: IncludedAttributes {
          created_at: "2023-01-01T00:00:00Z".to_string(),
          description: "bad version".to_string(),
          downloads: 10,
          published_at: "2023-01-01T00:00:00Z".to_string(),
          source: "".to_string(),
          updated_at: "2023-01-01T00:00:00Z".to_string(),
          version: "invalid".to_string(),
        },
        links: serde_json::json!({}),
      }],
    };

    let result = response.summarize();
    assert!(result.is_err(), "Invalid version should return error");
  }
}
