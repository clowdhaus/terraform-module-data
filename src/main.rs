use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use terraform_module_data::{cli as tmd, graph};
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = tmd::Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  // Root directory where data is stored
  let data_path = PathBuf::from("data");

  // Directory where generated graphs are stored
  let assets_path = PathBuf::from("assets");

  match &cli.command {
    tmd::Commands::CollectData(download) => download.collect(data_path).await,
    tmd::Commands::Graph => graph::graph(&data_path, &assets_path),
  }
}
