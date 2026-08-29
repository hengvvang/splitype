//! Platform-specific adapters — app identity, macOS file associations, and
//! macOS CLI tool installation.

pub mod app_identity;

pub mod cli_tool;

#[cfg(any(target_os = "macos", test))]
pub mod file_url;
