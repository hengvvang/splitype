//! macOS CLI tool installation and uninstallation.
//!
//! Installs a `splitype` symlink into `/usr/local/bin` pointing at the
//! running `.app` bundle via AppleScript with administrator privileges.
//! Non-macOS targets provide stubs that report unavailability.
//!
//! This module holds only the platform operations; the user-facing
//! prompts and localization live in `crate::app::cli::install`.

#[cfg(target_os = "macos")]
use std::process::{Command, Output};

/// Returns `true` only if the symlink exists **and** resolves (directly or via
/// one level of canonicalization) to the currently running executable.
#[cfg(target_os = "macos")]
pub fn is_cli_symlink_current_app() -> bool {
    let link = std::path::Path::new("/usr/local/bin/splitype");
    let Ok(target) = std::fs::read_link(link) else {
        return false; // does not exist or not a symlink
    };
    let resolved = if target.is_absolute() {
        // Canonicalize the target itself (may fail if dangling).
        std::fs::canonicalize(&target).unwrap_or(target)
    } else {
        // Relative — resolve from symlink's parent directory.
        link.parent()
            .unwrap_or(std::path::Path::new("/"))
            .join(&target)
            .canonicalize()
            .unwrap_or(target)
    };
    match std::env::current_exe() {
        Ok(exe) => resolved == exe,
        Err(_) => false,
    }
}

/// Escape a string for use inside an AppleScript string literal.
///
/// Pure text transformation, so it compiles on every platform; the
/// osascript runner below is macOS-only.
pub fn applescript_string_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

/// Run an AppleScript command with administrator privileges and return the
/// raw process output (stderr included).
#[cfg(target_os = "macos")]
pub fn run_osascript(script: &str) -> std::io::Result<Output> {
    Command::new("osascript").arg("-e").arg(script).output()
}

