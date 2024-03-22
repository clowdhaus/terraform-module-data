use std::env;

use anyhow::Result;
use clap::Parser;
use terraform_module_data::cli as tmd;
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

  let cur_exe = env::current_exe()?;
  let _cur_dir = cur_exe.parent().unwrap().parent().unwrap().parent().unwrap();

  match &cli.command {
    tmd::Commands::CollectData(download) => download.get().await,
  }
}
