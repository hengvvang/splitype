//! Dynamic Git pre-commit hook installer and remover.

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::fs;

use crate::runner::resolve_git_hooks_dir;

#[derive(Args)]
pub struct HookArgs {
    #[command(subcommand)]
    pub action: HookAction,
}

#[derive(Subcommand)]
pub enum HookAction {
    /// Install pre-commit hook into the active Git repository
    Install,
    /// Uninstall pre-commit hook
    Uninstall,
}

const PRE_COMMIT_SCRIPT: &str = r#"#!/usr/bin/env sh
# Installed by `cargo xtask hook install`
echo "==> [pre-commit] Executing workspace quality gate..."
cargo xtask check
if [ $? -ne 0 ]; then
    echo "❌ [pre-commit] Quality gate failed. Please resolve issues before committing."
    exit 1
fi
"#;

pub fn run(args: HookArgs) -> Result<()> {
    let hooks_dir = resolve_git_hooks_dir()?;
    let hook_file = hooks_dir.join("pre-commit");

    match args.action {
        HookAction::Install => {
            fs::create_dir_all(&hooks_dir).context("Failed to create Git hooks directory")?;
            fs::write(&hook_file, PRE_COMMIT_SCRIPT)
                .context("Failed to write pre-commit script")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&hook_file)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&hook_file, perms)?;
            }

            println!("✓ Pre-commit hook installed to: {}", hook_file.display());
        }
        HookAction::Uninstall => {
            if hook_file.exists() {
                fs::remove_file(&hook_file)?;
                println!("✓ Pre-commit hook uninstalled.");
            } else {
                println!("No pre-commit hook was found to uninstall.");
            }
        }
    }

    Ok(())
}
