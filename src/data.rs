use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
  pub version: String,
  pub major_version: String,
}

impl Response {
  pub fn summarize(&self) -> Result<BTreeMap<String, Summary>> {
    let mut summary: BTreeMap<String, Summary> = BTreeMap::new();
    for i in self.included.iter() {
      let major_version = i.attributes.version.split('.').next().unwrap().to_string();
      let key = format!("{:02}", major_version.parse::<u64>().unwrap_or(0));

      let record = summary.entry(key).or_insert(Summary {
        downloads: 0,
        version: major_version.clone(),
        major_version,
      });

      record.downloads += i.attributes.downloads;
    }

    Ok(summary)
  }
}
