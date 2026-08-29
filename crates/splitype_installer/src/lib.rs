//! splitype_installer — software lifecycle management for Splitype:
//! installing the CLI tool and checking for updates.
//!
//! - [`cli_tool`] — the macOS platform operations (symlink into
//!   `/usr/local/bin` via AppleScript).
//! - [`install`] — the user-facing installation wizard (gpui windows,
//!   i18n strings; macOS-only, stubs elsewhere).
//! - [`update_checker`] — online update checks against the public
//!   Cargo manifest (GitHub with a Gitee timeout fallback).
//!
//! The app composition root wires these into its menus ("Install CLI
//! Tool", "Check for Updates").

pub mod cli_tool;
pub mod install;
pub mod update_checker;

pub use install::{install_cli_tool, uninstall_cli_tool};
