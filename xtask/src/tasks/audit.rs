//! Dependency and supply chain hygiene (unused dependencies and vulnerability/license audits).

use anyhow::{Result, bail};
use clap::Args;

use crate::runner::{cmd, is_runnable, run_step};

#[derive(Args)]
pub struct AuditArgs {
    /// Strict mode: fail immediately if audit tools are missing (mandatory in CI)
    #[arg(long, default_value_t = false)]
    pub strict: bool,
}

pub fn run(args: AuditArgs) -> Result<()> {
    // Audit unused dependencies across the workspace using cargo-machete
    if is_runnable("cargo-machete") {
        run_step("cargo machete", cmd("cargo", &["machete"]))?;
    } else if args.strict {
        bail!(
            "Tool 'cargo-machete' is required in strict mode (`cargo install cargo-machete --locked`)"
        );
    } else {
        eprintln!("[warn] cargo-machete not installed, skipping unused dependency check");
    }

    // Verify vulnerability advisories, bans, and license compliance using cargo-deny
    if is_runnable("cargo-deny") {
        run_step(
            "cargo deny",
            cmd("cargo", &["deny", "--workspace", "check"]),
        )?;
    } else if args.strict {
        bail!("Tool 'cargo-deny' is required in strict mode (`cargo install cargo-deny --locked`)");
    } else {
        eprintln!("[warn] cargo-deny not installed, skipping security/license audit");
    }

    Ok(())
}
