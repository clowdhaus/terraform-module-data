pub mod data;

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Args, Debug, Deserialize, Serialize)]
pub struct Download {
  /// The module to collect download data for
  #[clap(short, long)]
  module: String,

  /// Path where the collected data outputs will be written to
  #[clap(short, long)]
  path: Option<PathBuf>,
}

impl Download {
  pub async fn get(&self) -> Result<()> {
    let url = Url::parse_with_params(
      format!(
        "https://registry.terraform.io/v2/modules/terraform-aws-modules/{}/aws",
        self.module
      )
      .as_str(),
      &[("include", "module-versions")],
    )?;

    let resp = Client::builder()
      .user_agent("Module Download Data")
      .build()?
      .get(url)
      .send()
      .await?;

    let response: crate::data::Response = resp.json().await?;
    println!("{:#?}", response);

    Ok(())
  }
}
