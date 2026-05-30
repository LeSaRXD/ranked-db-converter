use clap::Parser;
use std::path::PathBuf;

#[allow(clippy::unwrap_used)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	/// Path to the dump file
	pub path: Option<PathBuf>,

	/// Only parse matches after the following match id
	#[arg(short, long, default_value_t = 0)]
	pub after: u64,

	/// Only parse matches before the following match id
	#[arg(short, long, default_value_t = u64::MAX)]
	pub before: u64,
}
