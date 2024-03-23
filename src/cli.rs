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
  /// Collect module data from the Terraform registry and GitHub repositories
  CollectData(Module),

  /// Generate graphs from the collected data
  Graph,
}

#[derive(Args, Debug, Deserialize, Serialize)]
pub struct Module {
  /// The module to collect download data for
  #[clap(short, long)]
  module: String,
}

impl Module {
  pub async fn collect(&self, data_path: PathBuf) -> Result<()> {
    // GitHub data
    crate::github::collect(&data_path, &self.module).await?;

    // Terraform registry data
    crate::registry::collect(&data_path, &self.module).await?;

    Ok(())
  }
}
