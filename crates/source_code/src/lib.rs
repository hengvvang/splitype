//! source_code — high-performance, modular source code editor plugin.
//!
//! Inspired by Zed's architecture with autonomous text buffers, layered
//! display maps (Tab/Fold/Wrap), multi-cursor selection collection,
//! syntax highlighting, indent guides, and virtualized viewport rendering.

pub mod buffer;
pub mod builder;
pub mod display_map;
pub mod element;
pub mod gutter;
pub mod input;
pub mod outline;
pub mod search;
pub mod selection;
pub mod settings;
pub mod state;
pub mod syntax;

pub use builder::*;
pub use settings::SourceCodeSettings;

#[cfg(test)]
mod tests;
