//! Local CI pipeline runner ensuring complete verification parity before pushing to remote.

use anyhow::Result;

use crate::tasks::{audit, check, test};

/// Runs all verification steps sequentially under strict mode.
/// Guarantees that passing locally ensures green GitHub Actions builds.
pub fn run() -> Result<()> {
    println!(">>> Starting Local CI Replication (Strict Mode) <<<\n");

    check::run(check::CheckArgs {
        fix: false,
        package: None,
    })?;

    test::run(test::TestArgs {
        package: None,
        filters: Vec::new(),
    })?;

    audit::run(audit::AuditArgs { strict: true })?;

    println!("\n✓ All local CI checks passed successfully. Safe to push!");
    Ok(())
}
