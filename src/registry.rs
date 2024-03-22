use std::{
  collections::BTreeMap,
  path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
