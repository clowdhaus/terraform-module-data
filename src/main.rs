use std::env;

use anyhow::Result;
use clap::Parser;
use module_download_data::cli as mdd;
use tracing_log::AsTrace;
use tracing_subscriber::FmtSubscriber;

#[cfg(not(tarpaulin_include))]
#[tokio::main]
async fn main() -> Result<()> {
  let cli = mdd::Cli::parse();
  let subscriber = FmtSubscriber::builder()
    .with_max_level(cli.verbose.log_level_filter().as_trace())
    .without_time()
    .finish();
  tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

  let cur_exe = env::current_exe()?;
  let _cur_dir = cur_exe.parent().unwrap().parent().unwrap().parent().unwrap();

  match &cli.command {
    mdd::Commands::GetDownloads(download) => download.get().await,
  }
}
