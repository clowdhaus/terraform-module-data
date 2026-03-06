use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use url::Url;

const GITHUB_TOKEN_ENV_VAR: &str = "TERRAFORM_MODULE_DATA";
const NO_ACCESS: [&str; 1] = ["s3-object"];

/// A single traffic entry (used for both page views and clones)
#[derive(Debug, Serialize, Deserialize)]
struct TrafficEntry {
  count: u64,
  timestamp: String,
  uniques: u64,
}

type TrafficSummary = BTreeMap<String, TrafficEntry>;

/// Load the currently saved traffic data from file
fn get_current_traffic(path: &Path) -> Result<TrafficSummary> {
  match fs::read_to_string(path) {
    Ok(data) => {
      info!("Reading existing data from file: {}", path.display());
      let summary: TrafficSummary = serde_json::from_str(&data)?;
      Ok(summary)
    }
    Err(_) => {
      info!("No existing data found for {}, creating new summary", path.display());
      Ok(TrafficSummary::new())
    }
  }
}

/// Merge new traffic entries into the existing summary
fn summarize_traffic(entries: Vec<TrafficEntry>, path: &Path) -> Result<TrafficSummary> {
  let mut summary = get_current_traffic(path)?;

  for v in entries {
    let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).context("Failed to parse timestamp")?;
    summary.insert(timestamp.date_naive().to_string(), v);
  }

  info!("{} summarized", path.display());

  Ok(summary)
}

/// Write traffic entries to a JSON file, merging with existing data
fn write_traffic(entries: Vec<TrafficEntry>, dir: &Path, filename: &str) -> Result<()> {
  let filepath = dir.join(filename);
  let summary = summarize_traffic(entries, &filepath)?;
  std::fs::create_dir_all(dir)?;

  let json = serde_json::to_string_pretty(&summary)?;
  std::fs::write(filepath, json)?;

  Ok(())
}

/// Fetch traffic data (views or clones) from the GitHub API
async fn get_traffic(module: &str, traffic_type: &str) -> Result<Vec<TrafficEntry>> {
  if NO_ACCESS.contains(&module) {
    bail!("No access to {traffic_type} data for {module}");
  }

  let url = Url::parse(
    format!("https://api.github.com/repos/terraform-aws-modules/terraform-aws-{module}/traffic/{traffic_type}").as_str(),
  )?;

  let token = match env::var(GITHUB_TOKEN_ENV_VAR) {
    Ok(v) => v,
    Err(e) => bail!("${GITHUB_TOKEN_ENV_VAR} is not set ({})", e),
  };

  let resp = Client::builder()
    .user_agent("Module Download Data")
    .build()?
    .get(url)
    .header("Accept", "application/vnd.github+json")
    .header("Authorization", format!("Bearer {token}"))
    .header("X-GitHub-Api-Version", "2022-11-28")
    .send()
    .await?;

  if resp.status().is_success() {
    let value: serde_json::Value = resp.json().await?;
    let entries_key = if traffic_type == "views" { "views" } else { "clones" };
    let entries: Vec<TrafficEntry> = serde_json::from_value(
      value
        .get(entries_key)
        .ok_or_else(|| anyhow::anyhow!("Missing '{entries_key}' field in response"))?
        .clone(),
    )?;
    debug!("GET /traffic/{traffic_type} response: {entries:#?}");
    Ok(entries)
  } else {
    error!("GET /traffic/{traffic_type} response: {resp:#?}");
    bail!("Failed to get {traffic_type} data")
  }
}

/// Collect module traffic data from GitHub
pub async fn collect(path: &Path, module: &str) -> Result<()> {
  let gh_path = path.join("github").join(module.to_lowercase());

  let views = get_traffic(module, "views").await?;
  write_traffic(views, &gh_path, "views.json")?;

  let clones = get_traffic(module, "clones").await?;
  write_traffic(clones, &gh_path, "clones.json")?;

  Ok(())
}

/// Graph the data collected and insert into mdbook docs
pub(crate) fn graph(data_path: &Path) -> Result<()> {
  let timestamp = chrono::Local::now().to_utc().format("%Y-%m-%d %H:%M:%S").to_string();

  graph_traffic(&timestamp, data_path, "Repository Clones", "clones", "github-clones.tpl", "github-clones.md")?;
  graph_traffic(&timestamp, data_path, "Repository Page Views", "views", "github-page-views.tpl", "github-page-views.md")?;

  Ok(())
}

fn graph_traffic(timestamp: &str, data_path: &Path, title: &str, data_type: &str, template: &str, output: &str) -> Result<()> {
  let all = create_time_series_graph(title, None, data_type, data_path)?;
  let data = create_time_series_graph(title, Some(crate::DATA), data_type, data_path)?;
  let compute = create_time_series_graph(title, Some(crate::COMPUTE), data_type, data_path)?;
  let serverless = create_time_series_graph(title, Some(crate::SERVERLESS), data_type, data_path)?;
  let network = create_time_series_graph(title, Some(crate::NETWORKING), data_type, data_path)?;
  let other = create_time_series_graph(title, Some(crate::OTHER), data_type, data_path)?;

  let tpl_path = PathBuf::from("src").join("templates").join(template);
  let tpl = fs::read_to_string(tpl_path)?;

  let out_path = PathBuf::from("docs").join(output);
  let rendered = tpl
    .replace("{{ date }}", timestamp)
    .replace("{{ all }}", &all)
    .replace("{{ data }}", &data)
    .replace("{{ compute }}", &compute)
    .replace("{{ serverless }}", &serverless)
    .replace("{{ network }}", &network)
    .replace("{{ other }}", &other);

  fs::write(out_path, rendered).map_err(Into::into)
}

fn create_time_series_graph(title: &str, category: Option<&str>, data_type: &str, data_path: &Path) -> Result<String> {
  let mut trace_data = Vec::new();

  for entry in fs::read_dir(data_path.join("github"))? {
    let entry = entry?;
    let dir_name = entry
      .path()
      .file_stem()
      .ok_or_else(|| anyhow::anyhow!("Missing file stem for path: {:?}", entry.path()))?
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("Non-UTF8 file stem for path: {:?}", entry.path()))?
      .to_owned();
    let filepath = entry.path().join(format!("{data_type}.json"));

    // If directory is not in category, skip
    // If no category provided, return all
    if let Some(c) = category {
      if !crate::CATEGORIES.get(c).ok_or_else(|| anyhow::anyhow!("Unknown category: {c}"))?.contains(&dir_name.as_str()) {
        continue;
      }
    }

    let data = fs::read_to_string(filepath)?;
    let summary: TrafficSummary = serde_json::from_str(&data)?;

    let mut x_data = vec![];
    let mut y_data = vec![];

    for (_, v) in summary.iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).context("Failed to parse timestamp")?;
      x_data.push(timestamp.date_naive());
      y_data.push(v.count.to_string());
    }

    trace_data.push(crate::graph::TraceData {
      name: dir_name,
      x_data,
      y_data,
    });
  }

  let category = category.unwrap_or("all");
  let title = format!("{} - {}", crate::titlecase(category.to_string())?, title);
  let html_title = title
    .split_whitespace()
    .map(|x| x.to_string().to_lowercase())
    .collect::<Vec<String>>()
    .join("_");

  let titles = crate::graph::Titles {
    title: title.to_string(),
    x_title: "Date".to_string(),
    y_title: crate::titlecase(data_type.to_string())?,
  };

  info!("Plotting {} time series", title);

  crate::graph::plot_time_series(&html_title, trace_data, titles, plotly::common::Mode::Lines)
}
