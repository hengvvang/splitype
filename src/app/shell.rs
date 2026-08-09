//! The window shell — the OS window's root entity.
//!
//! Owns the window-level panel state (outer layout tree, explorer, outline,
//! settings) and the mapping from layout areas to content entities
//! (`AreaContent`). Renders the titlebar, split containers, and overlays;
//! content entities (`Editor`) render their own area frame and reach back
//! into the shell through a weak handle.

use std::collections::HashMap;

use gpui::*;

use crate::editor::controller::Editor;
use crate::layout::AreaId;

/// Weak handle to the window shell, held by content entities to reach
/// layout operations without creating a reference cycle.
pub type WeakShell = WeakEntity<Shell>;

/// The content of one area in the outer layout tree.
pub enum AreaContent {
    /// An editor with its own tab list and inner panel layout.
    Editor(Entity<Editor>),
}

/// The OS window's root entity: content areas + window lifecycle.
pub struct Shell {
    /// Content entity per outer area id.
    pub(crate) areas: HashMap<AreaId, AreaContent>,
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
