use crate::pane::SourceCodeState;
use editor_contracts::{PaneDescriptor, PaneKind, PaneView};
use gpui::SharedString;

/// Stable kind identifier of the Source Code pane plugin.
pub const PANE_KIND: &str = "splitype.pane.source_code";

/// Pane descriptor for Source Code mode.
#[derive(Clone, Debug, Default)]
pub struct SourceCodeDescriptor {}

impl SourceCodeDescriptor {
    pub fn new() -> Self {
        Self {}
    }
}

impl PaneDescriptor for SourceCodeDescriptor {
    fn kind(&self) -> PaneKind {
        PaneKind::from_static(PANE_KIND)
    }

    fn display_name(&self) -> SharedString {
        "Source Code".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(SourceCodeState::default())
    }
}
