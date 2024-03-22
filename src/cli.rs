use std::{collections::BTreeMap, path::PathBuf};

use anstyle::{AnsiColor, Color, Style};
use anyhow::Result;
use chrono::prelude::*;
use clap::{builder::Styles, Args, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use serde::{Deserialize, Serialize};

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
    let gh_page_views = crate::github::get_page_views(&self.module).await?;
    println!("{:#?}", gh_page_views.summarize()?);

    let registry = crate::registry::get(&self.module).await?;
    self.write(registry.summarize()?)?;

    Ok(())
  }

  pub fn write(&self, data: BTreeMap<String, crate::registry::Summary>) -> Result<()> {
    let path = match self.path.as_deref() {
      Some(p) => p.to_path_buf(),
      None => PathBuf::from("data")
        .join("registry")
        .join(self.module.clone().to_lowercase()),
    };
    std::fs::create_dir_all(&path)?;

    let data = data.into_values().collect::<Vec<crate::registry::Summary>>();
    let utc: DateTime<Utc> = Utc::now();
    let file = path.join(format!("{}.json", utc.format("%Y-%m-%d")));
    let json = serde_json::to_string_pretty(&data)?;
    std::fs::write(file, json)?;

    Ok(())
  }
}
