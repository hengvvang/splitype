//! Test execution orchestrator with automatic nextest acceleration and filter forwarding.

use anyhow::Result;
use clap::Args;

use crate::runner::{cmd, is_runnable, run_step};

#[derive(Args)]
pub struct TestArgs {
    /// Restrict test execution to a specific package
    #[arg(short, long)]
    pub package: Option<String>,

    /// Filter expressions forwarded directly to the underlying test runner
    #[arg(last = true)]
    pub filters: Vec<String>,
}

pub fn run(args: TestArgs) -> Result<()> {
    // Automatically use nextest for parallel execution when available; fallback gracefully
    let has_nextest = is_runnable("cargo-nextest");
    let mut test_args = if has_nextest {
        println!("==> Detected cargo-nextest: parallel execution enabled.");
        vec!["nextest", "run"]
    } else {
        println!("==> cargo-nextest not found: falling back to standard cargo test.");
        vec!["test"]
    };

    if let Some(pkg) = &args.package {
        test_args.extend(["-p", pkg]);
    } else {
        test_args.push("--workspace");
    }

    for filter in &args.filters {
        test_args.push(filter);
    }

    run_step("cargo test runner", cmd("cargo", &test_args))
}
