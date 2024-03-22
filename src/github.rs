use std::{collections::BTreeMap, env, fs, path::PathBuf};

use anyhow::{bail, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

const GITHUB_TOKEN_ENV_VAR: &str = "TERRAFORM_MODULE_DATA";

#[derive(Debug, Serialize, Deserialize)]
pub struct PageViewsResponse {
  pub count: u64,
  pub uniques: u64,
  pub views: Vec<View>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct View {
  pub count: u64,
  pub timestamp: String,
  pub uniques: u64,
}

pub type PageViewSummary = BTreeMap<String, View>;

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

  pub fn write(self, module: &str, path: PathBuf) -> Result<()> {
    let filepath = path.join(format!("{module}.json"));
    let summary = self.summarize(&filepath)?;
    std::fs::create_dir_all(&path)?;

    let json = serde_json::to_string_pretty(&summary)?;
    std::fs::write(filepath, json)?;

    Ok(())
  }
}

pub async fn get_page_views(module: &str) -> Result<PageViewsResponse> {
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
