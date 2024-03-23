use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use url::Url;

const GITHUB_TOKEN_ENV_VAR: &str = "TERRAFORM_MODULE_DATA";
const NO_ACCESS: [&str; 1] = ["s3-object"];

/// Page Views

#[derive(Debug, Serialize, Deserialize)]
struct PageViewsResponse {
  count: u64,
  uniques: u64,
  views: Vec<View>,
}

#[derive(Debug, Serialize, Deserialize)]
struct View {
  count: u64,
  timestamp: String,
  uniques: u64,
}

type PageViewSummary = BTreeMap<String, View>;

impl PageViewsResponse {
  /// Load the currently saved data from file
  fn get_current(&self, path: &PathBuf) -> Result<PageViewSummary> {
    match fs::read_to_string(path) {
      Ok(data) => {
        info!("Reading existing data from file: {}", path.display());
        let summary: PageViewSummary = serde_json::from_str(&data)?;
        Ok(summary)
      }
      Err(_) => {
        info!("No existing data found for {}, creating new summary", path.display());
        Ok(PageViewSummary::new())
      }
    }
  }

  fn summarize(self, path: &PathBuf) -> Result<PageViewSummary> {
    let mut summary = self.get_current(path)?;

    for v in self.views.into_iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();
      summary.insert(timestamp.date_naive().to_string(), v);
    }

    info!("{} summarized", path.display());

    Ok(summary)
  }

  fn write(self, path: &PathBuf) -> Result<()> {
    let filepath = path.join("views.json");
    let summary = self.summarize(&filepath)?;
    std::fs::create_dir_all(path)?;

    let json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(filepath, json)?;

    Ok(())
  }
}

async fn get_page_views(module: &str) -> Result<PageViewsResponse> {
  if NO_ACCESS.contains(&module) {
    bail!("No access to page views for {}", module);
  }

  let url = Url::parse(
    format!("https://api.github.com/repos/terraform-aws-modules/terraform-aws-{module}/traffic/views").as_str(),
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
    let response: PageViewsResponse = resp.json().await?;
    debug!("GET /traffic/views response: {:#?}", response);
    Ok(response)
  } else {
    error!("GET /traffic/views response: {:#?}", resp);
    bail!("Failed to get page views")
  }
}

/// Repository Clones

#[derive(Debug, Serialize, Deserialize)]
struct RepositoryCloneResponse {
  count: u64,
  uniques: u64,
  clones: Vec<Clone>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Clone {
  count: u64,
  timestamp: String,
  uniques: u64,
}

type RepositoryCloneSummary = BTreeMap<String, Clone>;

impl RepositoryCloneResponse {
  /// Load the currently saved data from file
  fn get_current(&self, path: &PathBuf) -> Result<RepositoryCloneSummary> {
    match fs::read_to_string(path) {
      Ok(data) => {
        info!("Reading existing data from file: {}", path.display());
        let summary: RepositoryCloneSummary = serde_json::from_str(&data)?;
        Ok(summary)
      }
      Err(_) => {
        info!("No existing data found for {}, creating new summary", path.display());
        Ok(RepositoryCloneSummary::new())
      }
    }
  }

  fn summarize(self, path: &PathBuf) -> Result<RepositoryCloneSummary> {
    let mut summary = self.get_current(path)?;

    for v in self.clones.into_iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();
      summary.insert(timestamp.date_naive().to_string(), v);
    }

    info!("{} summarized", path.display());

    Ok(summary)
  }

  fn write(self, path: &PathBuf) -> Result<()> {
    let filepath = path.join("clones.json");
    let summary = self.summarize(&filepath)?;
    std::fs::create_dir_all(path)?;

    let json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(filepath, json)?;

    Ok(())
  }
}

async fn get_repository_clones(module: &str) -> Result<RepositoryCloneResponse> {
  if NO_ACCESS.contains(&module) {
    bail!("No access to page views for {}", module);
  }

  let url = Url::parse(
    format!("https://api.github.com/repos/terraform-aws-modules/terraform-aws-{module}/traffic/clones").as_str(),
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
    let response: RepositoryCloneResponse = resp.json().await?;
    debug!("GET /traffic/clones response: {:#?}", response);
    Ok(response)
  } else {
    error!("GET /traffic/clones response: {:#?}", resp);
    bail!("Failed to get clones")
  }
}

/// Collect module traffic data from GitHub
pub async fn collect(path: &Path, module: &str) -> Result<()> {
  // GitHub repository data
  let gh_path = path.join("github").join(module.to_lowercase());

  let gh_views = get_page_views(module).await?;
  gh_views.write(&gh_path)?;
  let gh_clones = get_repository_clones(module).await?;
  gh_clones.write(&gh_path)?;

  Ok(())
}

/// Graph the data collected and insert into mdbook docs
pub(crate) fn graph(data_path: &Path) -> Result<()> {
  graph_clones(data_path)?;
  graph_page_views(data_path)?;

  Ok(())
}

fn graph_clones(data_path: &Path) -> Result<()> {
  let title = "Repository Clones";

  let data = graph_time_series(title, "data", "clones", data_path)?;
  let compute = graph_time_series(title, "compute", "clones", data_path)?;
  let serverless = graph_time_series(title, "serverless", "views", data_path)?;
  let network = graph_time_series(title, "network", "clones", data_path)?;
  let other = graph_time_series(title, "other", "clones", data_path)?;

  let tpl_path = PathBuf::from("src").join("templates").join("github-clones.tpl");
  let tpl = fs::read_to_string(tpl_path)?;

  let out_path = PathBuf::from("docs").join("github-clones.md");
  let rendered = tpl
    .replace("{{ data }}", &data)
    .replace("{{ compute }}", &compute)
    .replace("{{ serverless }}", &serverless)
    .replace("{{ network }}", &network)
    .replace("{{ other }}", &other);

  fs::write(out_path, rendered).map_err(Into::into)
}

fn graph_page_views(data_path: &Path) -> Result<()> {
  let title = "Repository Page Views";

  let data = graph_time_series(title, "data", "views", data_path)?;
  let compute = graph_time_series(title, "compute", "views", data_path)?;
  let serverless = graph_time_series(title, "serverless", "views", data_path)?;
  let network = graph_time_series(title, "network", "views", data_path)?;
  let other = graph_time_series(title, "other", "views", data_path)?;

  let tpl_path = PathBuf::from("src").join("templates").join("github-page-views.tpl");
  let tpl = fs::read_to_string(tpl_path)?;

  let out_path = PathBuf::from("docs").join("github-page-views.md");
  let rendered = tpl
    .replace("{{ data }}", &data)
    .replace("{{ compute }}", &compute)
    .replace("{{ serverless }}", &serverless)
    .replace("{{ network }}", &network)
    .replace("{{ other }}", &other);

  fs::write(out_path, rendered).map_err(Into::into)
}

fn graph_time_series(title: &str, category: &str, data_type: &str, data_path: &Path) -> Result<String> {
  let mut trace_data = Vec::new();

  for entry in fs::read_dir(data_path.join("github"))? {
    let entry = entry?;
    let dir_name = entry.path().file_stem().unwrap().to_str().unwrap().to_owned();
    let filepath = entry.path().join(format!("{data_type}.json"));

    // If directory is not in category, skip
    if !crate::CATEGORIES.get(category).unwrap().contains(&dir_name.as_str()) {
      continue;
    }

    let data = fs::read_to_string(filepath)?;
    let views: PageViewSummary = serde_json::from_str(&data)?;

    let mut x_data = vec![];
    let mut y_data = vec![];

    for (_, v) in views.iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();
      x_data.push(timestamp.date_naive());
      y_data.push(v.count.to_string());
    }

    trace_data.push(crate::graph::TraceData {
      name: dir_name,
      x_data,
      y_data,
    });
  }

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

  crate::graph::plot_time_series(&html_title, trace_data, titles)
}
