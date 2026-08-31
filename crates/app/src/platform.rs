//! Platform-specific adapters — macOS file associations and other OS glue.

#[cfg(any(target_os = "macos", test))]
pub mod file_url;
