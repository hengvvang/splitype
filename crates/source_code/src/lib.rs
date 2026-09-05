//! source_code — high-performance, modular source code editor plugin.
//!
//! Inspired by Zed's architecture with autonomous text buffers, layered
//! display maps (Tab/Fold/Wrap), multi-cursor selection collection,
//! syntax highlighting, indent guides, and virtualized viewport rendering.

pub mod assets;
pub mod brackets;
pub mod builder;
pub mod display_map;
pub mod editor;
pub mod gutter;
pub mod indent_guides;
pub mod outline;
pub mod pane;
pub mod search;
pub mod selection;
pub mod settings;

pub use builder::*;
pub use rope::Rope;
pub use settings::SourceCodeSettings;

pub const MANIFEST_TOML: &str = include_str!("../manifest.toml");

#[cfg(test)]
mod tests;
