//! The window shell — the OS window's root entity.
//!
//! Owns the mapping from layout areas to content entities (`AreaContent`)
//! and delegates the full window rendering to its primary editor, which
//! owns the window-level panel state and all overlays.

use std::collections::HashMap;

use gpui::*;

use crate::editor::controller::Editor;
use crate::splitter::NodeId;

/// The content of one area in the outer layout tree.
pub enum AreaContent {
    /// An editor with its own tab list and inner panel layout.
    Editor(Entity<Editor>),
}

/// The OS window's root entity: content areas + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id.
    pub(crate) areas: HashMap<NodeId, AreaContent>,
}

impl Shell {
    /// The window's primary (first) editor area content, if any.
    pub(crate) fn primary_editor(&self) -> Option<&Entity<Editor>> {
        self.areas.values().find_map(|area| match area {
            AreaContent::Editor(editor) => Some(editor),
        })
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // The shell delegates the full window rendering to its primary
        // editor for now; the titlebar, split tree, and overlays migrate
        // here incrementally.
        match self.primary_editor() {
            Some(editor) => editor.clone().into_any_element(),
            None => div().into_any_element(),
        }
    }
}
