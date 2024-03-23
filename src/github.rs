use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

const GITHUB_TOKEN_ENV_VAR: &str = "TERRAFORM_MODULE_DATA";

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
        let summary: PageViewSummary = serde_json::from_str(&data)?;
        Ok(summary)
      }
      Err(_) => Ok(PageViewSummary::new()),
    }
  }

  fn summarize(self, path: &PathBuf) -> Result<PageViewSummary> {
    let mut summary = self.get_current(path)?;

    for v in self.views.into_iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();
      summary.insert(timestamp.date_naive().to_string(), v);
    }

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

  let response: PageViewsResponse = resp.json().await?;
  Ok(response)
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
        let summary: RepositoryCloneSummary = serde_json::from_str(&data)?;
        Ok(summary)
      }
      Err(_) => Ok(RepositoryCloneSummary::new()),
    }
  }

  fn summarize(self, path: &PathBuf) -> Result<RepositoryCloneSummary> {
    let mut summary = self.get_current(path)?;

    for v in self.clones.into_iter() {
      let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();
      summary.insert(timestamp.date_naive().to_string(), v);
    }

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

  let response: RepositoryCloneResponse = resp.json().await?;
  Ok(response)
}

pub async fn collect(path: &Path, module: &str) -> Result<()> {
  // GitHub repository data
  let gh_path = path.join("github").join(module.to_lowercase());
  let gh_views = get_page_views(module).await?;
  gh_views.write(&gh_path)?;
  let gh_clones = get_repository_clones(module).await?;
  gh_clones.write(&gh_path)?;

  Ok(())
}

pub(crate) fn graph(data_path: &Path) -> Result<()> {
  let views = graph_time_series("Repository Page Views", "views", data_path)?;
  let clones = graph_time_series("Repository Clones", "clones", data_path)?;

  let tpl_path = PathBuf::from("src").join("templates").join("github.tpl");
  let tpl = fs::read_to_string(tpl_path)?;

  let out_path = PathBuf::from("docs").join("github.md");
  let rendered = tpl.replace("{{ views }}", &views).replace("{{ clones }}", &clones);
  fs::write(out_path, rendered)?;

  Ok(())
}

fn graph_time_series(title: &str, data_type: &str, data_path: &Path) -> Result<String> {
  let mut trace_data = Vec::new();

  for entry in fs::read_dir(data_path.join("github"))? {
    let entry = entry?;
    let dir_name = entry.path().file_stem().unwrap().to_str().unwrap().to_owned();
    let filepath = entry.path().join(format!("{data_type}.json"));

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

  // Ugly way of title casing a string
  let mut data_type = data_type.to_string();
  let y_title = data_type.remove(0).to_uppercase().to_string() + &data_type;

  let title = title.to_string();
  let html_title = title
    .split_whitespace()
    .map(|x| x.to_string().to_lowercase())
    .collect::<Vec<String>>()
    .join("_");

  let titles = crate::graph::Titles {
    title: title.to_string(),
    x_title: "Date".to_string(),
    y_title,
  };

  crate::graph::plot_time_series(&html_title, trace_data, titles)
}
