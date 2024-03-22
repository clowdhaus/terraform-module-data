use std::env;

use anyhow::{bail, Result};
use itertools::Itertools;
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

impl PageViewsResponse {
  pub fn summarize(&self) -> Result<Vec<View>> {
    let views = self
      .views
      .iter()
      .map(|v| {
        let timestamp = chrono::DateTime::parse_from_rfc3339(&v.timestamp).unwrap();

        View {
          count: v.count,
          timestamp: timestamp.date_naive().to_string(),
          uniques: v.uniques,
        }
      })
      .sorted_by(|a, b| a.timestamp.cmp(&b.timestamp))
      .collect();

    Ok(views)
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
