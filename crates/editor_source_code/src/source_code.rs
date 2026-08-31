//! editor_source_code — high-performance, modular source code editor plugin.
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
pub mod state;
pub mod syntax;

pub use builder::*;

pub use buffer::{Anchor, Bias, BufferPoint, LineMap};
pub use display_map::{DisplayPoint, DisplaySnapshot, FoldMap, FoldRange, TabMap, WrapMap};
pub use element::{EditorElement, SourceCodePrepaintState};
pub use gutter::GutterLayout;
pub use input::{handle_key_down, handle_mouse_down, handle_mouse_move, handle_mouse_up, hit_test};
pub use selection::{Selection, SelectionsCollection};
pub use state::SourceCodeState;
pub use syntax::{
    CodeHighlightResult, CodeHighlightSpan, CodeLanguageKey, find_matching_bracket,
    highlight_code_block, prewarm_code_highlight_registry,
};

#[cfg(test)]
mod tests;
