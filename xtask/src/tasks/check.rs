//! Code formatting, compilation checks, and Clippy lint enforcement.

use anyhow::Result;
use clap::Args;

use crate::runner::{cmd, run_step};

#[derive(Args)]
pub struct CheckArgs {
    /// Automatically apply code formatting and safe clippy suggestions in-place
    #[arg(long)]
    pub fix: bool,

    /// Restrict checks to a specific package rather than the entire workspace
    #[arg(short, long)]
    pub package: Option<String>,
}

pub fn run(args: CheckArgs) -> Result<()> {
    if args.fix {
        run_step("cargo fmt (in-place)", cmd("cargo", &["fmt", "--all"]))?;

        let mut clippy_args = vec!["clippy", "--fix", "--allow-dirty", "--allow-staged"];
        if let Some(pkg) = &args.package {
            clippy_args.extend(["-p", pkg]);
        } else {
            clippy_args.extend(["--workspace", "--all-targets"]);
        }
        return run_step("cargo clippy --fix", cmd("cargo", &clippy_args));
    }

    // Standard verification mode: fails if any formatting or lint deviations exist
    run_step(
        "cargo fmt --check",
        cmd("cargo", &["fmt", "--all", "--", "--check"]),
    )?;

    let mut check_args = vec!["check"];
    if let Some(pkg) = &args.package {
        check_args.extend(["-p", pkg]);
    } else {
        check_args.extend(["--workspace", "--all-targets"]);
    }
    run_step("cargo check (all targets)", cmd("cargo", &check_args))?;

    let mut clippy_args = vec!["clippy"];
    if let Some(pkg) = &args.package {
        clippy_args.extend(["-p", pkg]);
    } else {
        clippy_args.extend(["--workspace", "--all-targets"]);
    }
    clippy_args.extend(["--", "-D", "warnings"]);
    run_step("cargo clippy (-D warnings)", cmd("cargo", &clippy_args))?;

    Ok(())
}
