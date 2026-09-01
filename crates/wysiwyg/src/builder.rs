use crate::pane::state::WysiwygPaneState;
use editor_contracts::{PaneDescriptor, PaneKind, PaneView};
use gpui::SharedString;

/// Stable kind identifier of the WYSIWYG pane plugin.
pub const PANE_KIND: &str = "splitype.pane.wysiwyg";

/// Pane descriptor for WYSIWYG mode.
#[derive(Clone, Debug, Default)]
pub struct WysiwygDescriptor {}

impl WysiwygDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PaneDescriptor for WysiwygDescriptor {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(PANE_KIND)
    }

    fn display_name(&self) -> SharedString {
        "WYSIWYG".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(WysiwygPaneState::default())
    }
}
