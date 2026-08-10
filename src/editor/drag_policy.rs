//! Window-level drag policy — the `DefaultDragPolicy` instantiated with
//! the editor's own operations.
//!
//! The splitter engine defines the policy interface and the window-level
//! defaults ([`DefaultDragPolicy`]); this module wires the two
//! host-dependent steps to the editor: Shift drags clone the whole window
//! into a new one, plain drags seed the fresh sibling leaf with a deep
//! copy of the source editor's content.

use gpui::{App, WeakEntity};

use crate::app::window_area::WindowAreaKind;
use crate::editor::controller::Editor;
use splitype_splitter::policy::{ClonedContainer, DefaultDragPolicy};
use splitype_splitter::state::SplitterContainer;
use splitype_splitter::types::NodeId;

pub(crate) struct WindowDragPolicy {
    pub(crate) policy: DefaultDragPolicy<
        Box<dyn FnMut(ClonedContainer<WindowAreaKind>, &mut App)>,
        Box<dyn FnMut(&mut SplitterContainer<WindowAreaKind>, NodeId, NodeId, &mut App)>,
    >,
}

impl WindowDragPolicy {
    pub(crate) fn new(editor: WeakEntity<Editor>) -> Self {
        let open_editor = editor.clone();
        let seed_editor = editor.clone();
        Self {
            policy: DefaultDragPolicy {
                // Shift drag: clone the whole container into a new window.
                open_clone_window: Box::new(move |cloned, cx| {
                    let _ = open_editor.update(cx, |ed, cx| {
                        ed.clone_window_into_new_window(cloned, cx);
                    });
                }),
                // Plain drag split: deep-copy the source editor's content
                // (inner panel layout + tab list) into the fresh sibling
                // leaf.
                seed_split_content: Box::new(move |container, src, dst, cx| {
                    let _ = seed_editor.update(cx, |ed, cx| {
                        ed.seed_split_content(container, src, dst, cx);
                    });
                }),
            },
        }
    }
}
