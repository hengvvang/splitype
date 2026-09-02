//! Shell-level action vocabulary shared with panel plugins.
//!
//! These actions are the public vocabulary that plugins dispatch to the
//! window shell (layout mutations, path opens) plus the generic editing
//! actions (copy/cut/paste/dismiss) that every text surface consumes.
//! They live in the platform contracts because both directions —
//! plugin -> shell and shell -> plugin — pass them across the boundary.

use gpui::*;
use schemars::JsonSchema;
use serde::Deserialize;
use splitter::tree::{NodeId, SplitAxis};

actions!(splitype, [Copy, Cut, Paste, SelectAll, DismissTransientUi,]);

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

/// Open a path in the active panel area (explorer row clicks).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct OpenPath {
    /// Absolute path of the file to open.
    pub path: String,
    /// True for double-click (permanent tab + focus); false for
    /// single click (transient preview tab).
    pub persistent: bool,
}

/// Open a path in a freshly split panel area (explorer Ctrl/Cmd+double-click).
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct OpenPathInSplit {
    /// Absolute path of the file to open.
    pub path: String,
}

/// A worktree path was renamed or moved. Panels with open documents on the
/// old path should re-point them to the new path.
#[derive(Clone, Debug, PartialEq, Deserialize, JsonSchema, gpui::Action)]
#[action(namespace = splitype)]
#[serde(deny_unknown_fields)]
pub struct UpdateOpenTabPaths {
    /// Old absolute path.
    pub from: String,
    /// New absolute path.
    pub to: String,
}
