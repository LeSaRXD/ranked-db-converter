use clap::Parser;
use std::path::PathBuf;

#[allow(clippy::unwrap_used)]
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
	/// Path to the dump file
	pub path: Option<PathBuf>,

	/// Only parse matches after the following match id
	#[arg(short, long)]
	pub after: Option<u64>,

	/// Only parse matches before the following match id
	#[arg(short, long)]
	pub before: Option<u64>,
}
impl Cli {
	pub fn after(&self) -> u64 {
		self.after.unwrap_or(0)
	}

	pub fn before(&self) -> u64 {
		self.after.unwrap_or(u64::MAX)
	}
}
