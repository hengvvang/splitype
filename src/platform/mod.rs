//! Platform-specific adapters — app identity, macOS file associations, and
//! macOS CLI tool installation.

pub(crate) mod app_identity;

pub(crate) mod cli_tool;

#[cfg(any(target_os = "macos", test))]
pub(crate) mod file_url;

