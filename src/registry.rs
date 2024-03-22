use std::{collections::BTreeMap, path::PathBuf};

use anyhow::Result;
use chrono::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
  pub data: Data,
  pub included: Vec<Included>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Data {
  #[serde(rename = "type")]
  pub dtype: String,
  pub id: String,
  pub attributes: Attributes,
  pub relationships: serde_json::Value,
  pub links: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Attributes {
  pub downloads: u64,
  pub full_name: String,
  pub name: String,
  pub namespace: String,
  pub owner_name: String,
  pub provider_logo_url: String,
  pub provider_name: String,
  pub source: String,
  pub verified: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Included {
  #[serde(rename = "type")]
  pub itype: String,
  pub id: String,
  pub attributes: IncludedAttributes,
  pub links: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct IncludedAttributes {
  pub created_at: String,
  pub description: String,
  pub downloads: u64,
  pub published_at: String,
  pub source: String,
  pub updated_at: String,
  pub version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Summary {
  pub downloads: u64,
  pub major_version: String,
  pub created_at: String,
}

impl Response {
  pub fn summarize(&self) -> Result<BTreeMap<String, Summary>> {
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

  pub fn write(&self, path: PathBuf, data: BTreeMap<String, Summary>) -> Result<()> {
    std::fs::create_dir_all(&path)?;

    let data = data.into_values().collect::<Vec<Summary>>();
    let utc: DateTime<Utc> = Utc::now();
    let file = path.join(format!("{}.json", utc.format("%Y-%m-%d")));
    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(file, json)?;

    Ok(())
  }
}

pub async fn get(module: &str) -> Result<Response> {
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
