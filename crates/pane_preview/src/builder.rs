use crate::state::PreviewState;
use editor_contracts::{PaneDescriptor, PaneKind, PaneView};
use gpui::SharedString;

/// Stable kind identifier of the Preview pane plugin.
pub const PANE_KIND: &str = "splitype.pane.preview";

/// Pane descriptor for Preview mode.
#[derive(Clone, Debug, Default)]
pub struct PreviewDescriptor {}

impl PreviewDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PaneDescriptor for PreviewDescriptor {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(PANE_KIND)
    }

    fn display_name(&self) -> SharedString {
        "Preview".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(PreviewState::default())
    }
}
