use std::path::PathBuf;

use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Args, Debug, Deserialize, Serialize)]
pub struct Download {
  /// The module to collect download data for
  module: String,

  /// Path where the collected data outputs will be written to
  path: Option<PathBuf>,
}
