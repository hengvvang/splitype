//! Mouse event handling (click, drag, scroll, hover).
//!
//! # Current state
//!
//! Mouse events are dispatched inline from the [`Editor`]'s
//! GPUI event-emitter impl in [`crate::ui::input::keyboard`]
//! and the render tree in [`crate::editor::render`].  The relevant
//! methods are:
//!
//! | Event                        | Handler                                                |
//! |------------------------------|--------------------------------------------------------|
//! | `MouseDownEvent` on editor   | [`Editor::on_editor_mouse_down`] (keyboard.rs)         |
//! | `ScrollWheelEvent` on editor | [`Editor::on_editor_scroll_wheel`] (keyboard.rs)       |
//! | `Hover` on editor            | [`Editor::on_editor_hover`] (keyboard.rs)              |
//! | Scrollbar drag               | [`Editor::start_scrollbar_drag`] (keyboard.rs)         |
//! |                              | [`Editor::update_scrollbar_drag`] (keyboard.rs)        |
//! |                              | [`Editor::end_scrollbar_drag`] (keyboard.rs)           |
//!
//! These are thin wrappers that toggle UI state
//! (scrollbar visibility, menu dismissal, table-axis preview
//! clearing) and therefore belong in the UI layer.
//!
//! # Extraction plan
//!
//! 1. Move the scrollbar-drag session (`ScrollbarDragSession`) and the
//!    three drag methods to this module as pure functions that take the
//!    `ScrollHandle` by reference.
//! 2. Hoist the menu-bar / table-axis clearing logic that runs on
//!    mouse-down so it can be called from both the editor chrome AND
//!    the block-level handlers.
//! 3. Extract the scroll-wheel → scrollbar-visibility bump into a
//!    reusable helper consumed by both the editor and the scrollbar
//!    component.
//!
//! Until Phase 1 lands, mouse dispatch stays in the legacy engine
//! modules to avoid churn in the event-propagation paths.
