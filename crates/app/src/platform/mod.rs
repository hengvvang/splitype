//! Platform-specific adapters — macOS file associations and other OS
//! glue. The CLI-tool installation wizard lives in splitype_installer.

#[cfg(any(target_os = "macos", test))]
pub mod file_url;

