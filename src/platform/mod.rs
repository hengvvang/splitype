//! Platform-specific adapters — custom titlebar chrome, app identity, macOS file associations.

pub(crate) mod app_identity;

#[cfg(any(target_os = "macos", test))]
pub(crate) mod file_url;

