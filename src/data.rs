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
