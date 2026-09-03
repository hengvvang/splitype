//! Low-level process spawning primitives, exit-code retention, and environment detection.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

/// Constructs a new `Command` configured with a binary and argument slice.
pub fn cmd(program: &str, args: &[&str]) -> Command {
    let mut c = Command::new(program);
    c.args(args);
    c
}

/// Executes a child process synchronously with standard I/O inheritance.
///
/// Captures execution duration and ensures non-zero exit codes bubble up
/// immediately with contextual diagnostic messages.
pub fn run_step(step_name: &str, mut command: Command) -> Result<()> {
    println!("\n==> [{step_name}]");
    let start = Instant::now();

    let status = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("Failed to spawn command: {command:?}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        bail!(
            "Step '{step_name}' failed with exit code {code} (elapsed: {:.2?})",
            start.elapsed()
        );
    }

    println!("    completed in {:.2?}", start.elapsed());
    Ok(())
}

/// Checks whether an external executable exists in PATH and is functional.
/// Executes `--version` silently; exits with false if the binary is missing or broken.
pub fn is_runnable(binary: &str) -> bool {
    Command::new(binary)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolves the actual `.git` hooks directory using Git itself.
/// Correctly handles custom `core.hooksPath` configurations and git worktrees.
pub fn resolve_git_hooks_dir() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--git-path", "hooks"])
        .output()
        .context("Failed to run `git rev-parse --git-path hooks`")?;

    if !output.status.success() {
        bail!("Failed to locate Git hooks directory. Are you inside a Git repository?");
    }

    let path_str = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(PathBuf::from(path_str))
}
