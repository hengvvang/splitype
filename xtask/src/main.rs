//! `cargo xtask` — Engineering and workflow automation tool for Splitype.

use clap::{Parser, Subcommand};
use std::process::{Command, ExitStatus};

#[derive(Parser)]
#[command(name = "xtask", about = "Splitype development workflow tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all workspace checks (format, check, clippy)
    Check,
    /// Run all tests across the workspace
    Test,
    /// Audit unused dependencies (cargo machete)
    Machete,
    /// Audit dependencies for advisories/licensing (cargo deny)
    Deny,
    /// Build the release binary
    BuildRelease,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check => {
            println!("🔍 Checking formatting...");
            run_cmd("cargo", &["fmt", "--all", "--", "--check"])?;

            println!("🦀 Running cargo check across workspace...");
            run_cmd("cargo", &["check", "--workspace", "--all-targets"])?;

            println!("✨ Running clippy across workspace...");
            run_cmd(
                "cargo",
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?;

            println!("✅ All workspace checks passed!");
        }
        Commands::Test => {
            println!("🧪 Running tests across workspace...");
            run_cmd("cargo", &["test", "--workspace"])?;
            println!("✅ All tests passed!");
        }
        Commands::Machete => {
            println!("🔎 Auditing unused dependencies...");
            run_cmd("cargo", &["machete"])?;
            println!("✅ No unused dependencies!");
        }
        Commands::Deny => {
            println!("🛡️  Auditing dependencies (advisories, licenses, bans)...");
            run_cmd("cargo", &["deny", "--workspace", "check"])?;
            println!("✅ Dependency audit passed!");
        }
        Commands::BuildRelease => {
            println!("📦 Building release package...");
            run_cmd("cargo", &["build", "--release", "-p", "app"])?;
            println!("✅ Release build completed!");
        }
    }

    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> anyhow::Result<ExitStatus> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        anyhow::bail!("Command failed: {} {}", program, args.join(" "));
    }
    Ok(status)
}
