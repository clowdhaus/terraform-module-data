use std::path::PathBuf;

use anstyle::{AnsiColor, Color, Style};
use anyhow::Result;
use clap::{builder::Styles, Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

/// Styles for CLI
fn get_styles() -> Styles {
  Styles::styled()
    .header(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue))),
    )
    .literal(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Cyan))))
    .usage(
      Style::new()
        .bold()
        .underline()
        .fg_color(Some(Color::Ansi(AnsiColor::Blue))),
    )
    .placeholder(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Magenta))))
}

#[derive(Debug, Parser)]
#[command(author, about, version)]
#[command(author, version, about)]
#[command(propagate_version = true)]
#[command(styles=get_styles())]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,

  #[clap(flatten)]
  pub verbose: Verbosity,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
  /// Get module downloads from Terraform registry
  GetDownloads(Download),
}

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
    // println!("{:#?}", response);
    println!("{:#?}", response.summarize()?);

    Ok(())
  }
}
