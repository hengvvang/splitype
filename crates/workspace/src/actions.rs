//! Window-level layout actions.
//!
//! Panels (explorer, settings, editor chrome) dispatch these actions from
//! their topbar controls; the shell (`crates/app`) handles them against
//! its window layout tree. This keeps the panel crates free of any shell
//! dependency.

use schemars::JsonSchema;
use serde::Deserialize;
use splitter::tree::{NodeId, SplitAxis};

/// Toggle the panel-kind dropdown of the given window panel.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct ToggleKindDropdown {
    /// The window panel (split-tree leaf) whose kind dropdown toggles.
    pub panel: NodeId,
}

/// Split the given window panel into two same-kind panels.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct SplitPanel {
    /// The window panel (split-tree leaf) to split.
    pub panel: NodeId,
    /// The split direction.
    pub axis: SplitAxis,
}

/// Toggle the given window panel between maximized and restored.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct TogglePanelMaximized {
    /// The window panel (split-tree leaf) to maximize or restore.
    pub panel: NodeId,
}

/// Close the given window panel.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct ClosePanel {
    /// The window panel (split-tree leaf) to close.
    pub panel: NodeId,
}
