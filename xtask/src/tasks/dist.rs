//! Production release compilation and binary bundling.

use anyhow::Result;
use clap::Args;

use crate::runner::{cmd, run_step};

#[derive(Args)]
pub struct DistArgs {
    /// Binary package name to build in release mode
    #[arg(long, default_value = "app")]
    pub package: String,
}

pub fn run(args: DistArgs) -> Result<()> {
    let dist_args = vec!["build", "--release", "-p", &args.package];
    run_step("cargo build --release", cmd("cargo", &dist_args))?;
    println!(
        "\nRelease artifact generated at: target/release/{}",
        args.package
    );
    Ok(())
}
