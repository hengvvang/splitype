//! WYSIWYG pane shell: the [`PaneView`] contract adapter around the
//! [`WysiwygDocumentController`] entity.

use gpui::{App, AppContext, Entity};

use crate::pane::controller::WysiwygDocumentController;

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPane {
    pub controller: Option<Entity<WysiwygDocumentController>>,
    /// The most recent document snapshot, seeding a lazily created controller.
    pub latest_snapshot: Option<editor_contracts::DocumentSnapshot>,
}

impl WysiwygPane {
    pub(crate) fn ensure_controller(&mut self, cx: &mut App) -> Entity<WysiwygDocumentController> {
        if let Some(controller) = &self.controller {
            return controller.clone();
        }
        let document = self
            .latest_snapshot
            .clone()
            .unwrap_or_else(editor_contracts::DocumentSnapshot::empty);
        let controller = cx.new(|cx| WysiwygDocumentController::new(&document, cx));
        self.controller = Some(controller.clone());
        controller
    }
}
