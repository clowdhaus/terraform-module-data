use anyhow::Result;
use itertools::Itertools;
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
  pub fn summarize(&self) -> Result<Vec<Summary>> {
    let summary = self
      .included
      .iter()
      .map(|i| Summary {
        downloads: i.attributes.downloads,
        version: i.attributes.version.clone(),
        major_version: i.attributes.version.split('.').next().unwrap().to_string(),
      })
      .group_by(|i| i.major_version.clone())
      .into_iter()
      .map(|(k, v)| {
        let total_downloads: u64 = v.map(|i| i.downloads).sum();
        Summary {
          downloads: total_downloads,
          version: k.clone(),
          major_version: k,
        }
      })
      .collect::<Vec<Summary>>();

    Ok(summary)
  }
}
