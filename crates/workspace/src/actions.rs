//! Window-level layout actions and generic editing actions.
//!
//! Panels (explorer, settings, editor chrome) dispatch these actions from
//! their topbar controls; the shell (`crates/app`) handles them against
//! its window layout tree. This keeps the panel crates free of any shell
//! dependency.

use gpui::*;
use schemars::JsonSchema;
use serde::Deserialize;
use splitter::tree::{NodeId, SplitAxis};

// Generic editing actions shared by every editing surface (editor panes,
// explorer filename editor). Defined here so panels never depend on the
// editor family.
actions!(
    splitype,
    [
        Copy,
        Cut,
        Paste,
        DismissTransientUi,
    ]
);

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

/// Open a path in the active editor panel (explorer row clicks).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct OpenInEditor {
    /// Absolute path of the file to open.
    pub path: String,
    /// True for double-click (permanent tab + focus editor); false for
    /// single click (transient preview tab).
    pub persistent: bool,
}

/// Open a path in a freshly split editor area (explorer Ctrl/Cmd+double-click).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct OpenInSplit {
    /// Absolute path of the file to open.
    pub path: String,
}
