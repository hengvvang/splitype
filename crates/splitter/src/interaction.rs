//! Centralized layout interaction manager.
//!
//! Owns all transient UI interaction states (drag sessions, dropdowns, border
//! menus, and maximize states), cleanly decoupled from the pure layout topology.

use crate::gesture::{BorderMenuState, CornerDragSession, SplitterDragSession};
use crate::id::LeafId;

/// Manages all transient interaction state for a split layout.
///
/// Keeps the layout tree pure: containers only store persistent identity and payload,
/// while active gestures, open menus, and full-screen maximization live here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutInteraction {
    /// Leaf currently maximized (filling the entire root), if any.
    maximized_leaf: Option<LeafId>,
    /// Leaf whose header dropdown is currently open (at most one open at a time).
    open_dropdown_leaf: Option<LeafId>,
    /// Active corner-drag gesture session, if any.
    active_corner_drag: Option<CornerDragSession>,
    /// Active splitter-bar drag session (resizing divider), if any.
    active_splitter_drag: Option<SplitterDragSession>,
    /// Active right-click border divider menu, if any.
    active_border_menu: Option<BorderMenuState>,
}

impl LayoutInteraction {
    /// Creates a fresh interaction manager with all sessions inactive.
    pub const fn new() -> Self {
        Self {
            maximized_leaf: None,
            open_dropdown_leaf: None,
            active_corner_drag: None,
            active_splitter_drag: None,
            active_border_menu: None,
        }
    }

    // ------------------------------------------------------------------
    // Maximization
    // ------------------------------------------------------------------

    /// Returns `true` if any leaf in the layout is currently maximized.
    #[inline]
    pub const fn is_maximized(&self) -> bool {
        self.maximized_leaf.is_some()
    }

    /// Returns `true` if `leaf` is currently maximized.
    #[inline]
    pub fn is_leaf_maximized<I: Into<LeafId>>(&self, leaf: I) -> bool {
        self.maximized_leaf == Some(leaf.into())
    }

    /// Returns the ID of the currently maximized leaf, if any.
    #[inline]
    pub const fn maximized_leaf(&self) -> Option<LeafId> {
        self.maximized_leaf
    }

    /// Explicitly sets the maximized leaf.
    pub fn set_maximized(&mut self, leaf: Option<LeafId>) {
        self.maximized_leaf = leaf;
    }

    /// Toggles the maximized state of `leaf`. If `leaf` was already maximized,
    /// restores the tiled layout. If another leaf was maximized, switches to `leaf`.
    pub fn toggle_maximize<I: Into<LeafId>>(&mut self, leaf: I) {
        let leaf = leaf.into();
        if self.maximized_leaf == Some(leaf) {
            self.maximized_leaf = None;
        } else {
            self.maximized_leaf = Some(leaf);
        }
    }

    /// Clears any active maximization.
    #[inline]
    pub fn clear_maximize(&mut self) {
        self.maximized_leaf = None;
    }

    // ------------------------------------------------------------------
    // Dropdown menus
    // ------------------------------------------------------------------

    /// Returns `true` if the dropdown for `leaf` is currently open.
    #[inline]
    pub fn is_dropdown_open<I: Into<LeafId>>(&self, leaf: I) -> bool {
        self.open_dropdown_leaf == Some(leaf.into())
    }

    /// Returns the ID of the leaf with an open dropdown, if any.
    #[inline]
    pub const fn open_dropdown_leaf(&self) -> Option<LeafId> {
        self.open_dropdown_leaf
    }

    /// Returns `true` if any dropdown in this layout is open.
    #[inline]
    pub const fn has_any_dropdown_open(&self) -> bool {
        self.open_dropdown_leaf.is_some()
    }

    /// Toggles the dropdown menu for `leaf`. Ensures at most one dropdown is open.
    pub fn toggle_dropdown<I: Into<LeafId>>(&mut self, leaf: I) {
        let leaf = leaf.into();
        if self.open_dropdown_leaf == Some(leaf) {
            self.open_dropdown_leaf = None;
        } else {
            self.open_dropdown_leaf = Some(leaf);
        }
    }

    /// Closes all dropdowns. Returns `true` if a dropdown was actually closed.
    pub fn clear_dropdowns(&mut self) -> bool {
        let was_open = self.open_dropdown_leaf.is_some();
        self.open_dropdown_leaf = None;
        was_open
    }

    // ------------------------------------------------------------------
    // Corner drag gesture
    // ------------------------------------------------------------------

    /// Returns `true` if a corner-drag gesture is in progress.
    #[inline]
    pub const fn is_corner_dragging(&self) -> bool {
        self.active_corner_drag.is_some()
    }

    /// Reference to the active corner-drag session, if any.
    #[inline]
    pub const fn active_corner_drag(&self) -> Option<&CornerDragSession> {
        self.active_corner_drag.as_ref()
    }

    /// Mutable reference to the active corner-drag session, if any.
    #[inline]
    pub fn active_corner_drag_mut(&mut self) -> Option<&mut CornerDragSession> {
        self.active_corner_drag.as_mut()
    }

    /// Starts a corner-drag gesture session.
    pub fn start_corner_drag(&mut self, session: CornerDragSession) {
        self.active_corner_drag = Some(session);
    }

    /// Finishes the corner-drag gesture, returning the final session data.
    pub fn finish_corner_drag(&mut self) -> Option<CornerDragSession> {
        self.active_corner_drag.take()
    }

    /// Cancels the active corner-drag gesture without applying any mutation.
    pub fn cancel_corner_drag(&mut self) -> bool {
        self.active_corner_drag.take().is_some()
    }

    // ------------------------------------------------------------------
    // Splitter-bar drag gesture
    // ------------------------------------------------------------------

    /// Returns `true` if a splitter-divider drag is currently in progress.
    #[inline]
    pub const fn is_splitter_dragging(&self) -> bool {
        self.active_splitter_drag.is_some()
    }

    /// Reference to the active splitter drag session, if any.
    #[inline]
    pub const fn active_splitter_drag(&self) -> Option<&SplitterDragSession> {
        self.active_splitter_drag.as_ref()
    }

    /// Mutable reference to the active splitter drag session, if any.
    #[inline]
    pub fn active_splitter_drag_mut(&mut self) -> Option<&mut SplitterDragSession> {
        self.active_splitter_drag.as_mut()
    }

    /// Starts a splitter-bar drag session.
    pub fn start_splitter_drag(&mut self, session: SplitterDragSession) {
        self.active_splitter_drag = Some(session);
    }

    /// Finishes the splitter drag session.
    pub fn finish_splitter_drag(&mut self) -> Option<SplitterDragSession> {
        self.active_splitter_drag.take()
    }

    /// Cancels the splitter drag session, returning the cancelled session so
    /// the caller can restore the original split ratio if desired.
    pub fn cancel_splitter_drag(&mut self) -> Option<SplitterDragSession> {
        self.active_splitter_drag.take()
    }

    // ------------------------------------------------------------------
    // Border context menu
    // ------------------------------------------------------------------

    /// Reference to the active border context menu state, if any.
    #[inline]
    pub const fn active_border_menu(&self) -> Option<&BorderMenuState> {
        self.active_border_menu.as_ref()
    }

    /// Opens the border context menu.
    pub fn open_border_menu(&mut self, menu: BorderMenuState) {
        self.active_border_menu = Some(menu);
    }

    /// Clears the border context menu. Returns `true` if a menu was open.
    pub fn clear_border_menu(&mut self) -> bool {
        let was_open = self.active_border_menu.is_some();
        self.active_border_menu = None;
        was_open
    }

    // ------------------------------------------------------------------
    // Utilities
    // ------------------------------------------------------------------

    /// Clears all transient context menus and overlays (dropdowns, border menus).
    pub fn clear_transient_overlays(&mut self) {
        self.open_dropdown_leaf = None;
        self.active_border_menu = None;
    }

    /// Handles cleanup when a leaf is retired or removed from the layout.
    pub fn on_leaf_removed<I: Into<LeafId>>(&mut self, removed: I) {
        let removed = removed.into();
        if self.maximized_leaf == Some(removed) {
            self.maximized_leaf = None;
        }
        if self.open_dropdown_leaf == Some(removed) {
            self.open_dropdown_leaf = None;
        }
        if let Some(session) = &self.active_corner_drag {
            if session.target_id == removed {
                self.active_corner_drag = None;
            }
        }
    }
}
