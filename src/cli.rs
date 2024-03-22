use std::path::PathBuf;

use anstyle::{AnsiColor, Color, Style};
use anyhow::Result;
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
}

impl Download {
  pub async fn get(&self) -> Result<()> {
    let data_path = PathBuf::from("data");

    // GitHub repository data
    let gh_path = data_path.join("github").join(self.module.clone().to_lowercase());
    let gh_views = crate::github::get_page_views(&self.module).await?;
    gh_views.write(&gh_path)?;
    let gh_clones = crate::github::get_repository_clones(&self.module).await?;
    gh_clones.write(&gh_path)?;

    // Terraform registry data
    let registry = crate::registry::get(&self.module).await?;
    let registry_path = data_path.join("registry").join(self.module.clone().to_lowercase());

    registry.write(registry_path, registry.summarize()?)?;

    Ok(())
  }
}
