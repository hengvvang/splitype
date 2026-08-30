//! editor_builder — Assembly and registration facade for Splitype editors.
//!
//! Provides the fluent [`EditorBuilder`] pattern to register built-in and
//! custom pane descriptors, configure layouts, load file sources, and
//! instantiate `Editor` entities.

pub mod builder;

pub use builder::EditorBuilder;
pub use editor_model::{PaneDescriptor, PaneKindId, PaneRegistry, PaneView};
