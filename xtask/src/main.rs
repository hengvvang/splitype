//! Splitype Workspace Automation Task Runner (xtask)
//!
//! Provides a single, type-safe, cross-platform entry point for code quality,
//! testing, packaging, supply-chain auditing, and Git lifecycle hooks.

mod runner;
mod tasks;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "cargo xtask",
    about = "Splitype automation and engineering workflow runner",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Format code, verify compilation, and enforce strict clippy lints
    Check(tasks::check::CheckArgs),

    /// Run workspace test suites (nextest-aware with fallback to cargo test)
    Test(tasks::test::TestArgs),

    /// Audit dependencies for unused entries and check security advisories/licenses
    Audit(tasks::audit::AuditArgs),

    /// Execute the complete CI validation suite locally in strict mode
    Ci,

    /// Compile optimized release artifacts for distribution
    Dist(tasks::dist::DistArgs),

    /// Manage Git pre-commit hooks
    Hook(tasks::hook::HookArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check(args) => tasks::check::run(args),
        Commands::Test(args) => tasks::test::run(args),
        Commands::Audit(args) => tasks::audit::run(args),
        Commands::Ci => tasks::ci::run(),
        Commands::Dist(args) => tasks::dist::run(args),
        Commands::Hook(args) => tasks::hook::run(args),
    }
}
