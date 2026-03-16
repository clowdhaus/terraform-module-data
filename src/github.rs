use std::{collections::BTreeMap, env, fs, path::Path};

use anyhow::{Context, Result, bail};
use chrono::Datelike;
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
    format!("https://api.github.com/repos/terraform-aws-modules/terraform-aws-{module}/traffic/{traffic_type}")
      .as_str(),
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

/// Output JSON data for the Astro site
pub(crate) fn graph(data_path: &Path, output_path: &Path) -> Result<()> {
  let timestamp = chrono::Local::now().to_utc().format("%Y-%m-%d %H:%M:%S").to_string();

  write_traffic_json(
    &timestamp,
    data_path,
    output_path,
    "Repository Clones",
    "clones",
    "github-clones.json",
  )?;
  write_traffic_json(
    &timestamp,
    data_path,
    output_path,
    "Repository Page Views",
    "views",
    "github-views.json",
  )?;

  Ok(())
}

fn write_traffic_json(
  timestamp: &str,
  data_path: &Path,
  output_path: &Path,
  title: &str,
  data_type: &str,
  filename: &str,
) -> Result<()> {
  let category_names: &[(&str, Option<&str>)] = &[
    ("All", None),
    ("Compute", Some(crate::COMPUTE)),
    ("Serverless", Some(crate::SERVERLESS)),
    ("Data", Some(crate::DATA)),
    ("Networking", Some(crate::NETWORKING)),
    ("Other", Some(crate::OTHER)),
  ];

  let mut sections = Vec::new();
  for &(section_title, category) in category_names {
    let datasets = collect_traffic_datasets(category, data_type, data_path)?;
    sections.push(crate::graph::ChartSection {
      title: section_title.to_string(),
      datasets,
    });
  }

  let page = crate::graph::ChartPage {
    title: title.to_string(),
    updated_at: timestamp.to_string(),
    sections,
  };

  info!("Writing {filename}");
  crate::graph::write_chart_page(output_path, filename, &page)
}

fn collect_traffic_datasets(
  category: Option<&str>,
  data_type: &str,
  data_path: &Path,
) -> Result<Vec<crate::graph::ChartDataset>> {
  let mut datasets = Vec::new();

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

    // If directory is not in category, skip; if no category provided, return all
    if let Some(c) = category
      && !crate::CATEGORIES
        .get(c)
        .ok_or_else(|| anyhow::anyhow!("Unknown category: {c}"))?
        .contains(&dir_name.as_str())
    {
      continue;
    }

    let data = fs::read_to_string(filepath)?;
    let summary: TrafficSummary = serde_json::from_str(&data)?;

    // Aggregate daily data into monthly buckets (sum counts per month)
    let mut monthly: BTreeMap<chrono::NaiveDate, u64> = BTreeMap::new();
    for (_, v) in summary.iter() {
      let ts = chrono::DateTime::parse_from_rfc3339(&v.timestamp).context("Failed to parse timestamp")?;
      let date = ts.date_naive();
      let month_start = chrono::NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .ok_or_else(|| anyhow::anyhow!("Invalid date: {date}"))?;
      *monthly.entry(month_start).or_insert(0) += v.count;
    }

    let dates: Vec<chrono::NaiveDate> = monthly.keys().copied().collect();
    let values: Vec<u64> = monthly.values().copied().collect();
    let (dates, values) = crate::graph::filter_incomplete_month(dates, values);

    let data_points = dates
      .iter()
      .zip(values.iter())
      .map(|(d, v)| crate::graph::DataPoint {
        x: d.to_string(),
        y: *v,
      })
      .collect();

    datasets.push(crate::graph::ChartDataset {
      label: dir_name,
      data: data_points,
    });
  }

  // Sort datasets by label for consistent output
  datasets.sort_by(|a, b| a.label.cmp(&b.label));

  Ok(datasets)
}
