use crate::state::PreviewState;
use core_contracts::{PaneDescriptor, PaneKind, PaneView};
use gpui::SharedString;

/// Pane descriptor for Preview mode.
#[derive(Clone, Debug, Default)]
pub struct PreviewDescriptor {}

impl PreviewDescriptor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn builder() -> PreviewBuilder {
        PreviewBuilder::new()
    }
}

impl PaneDescriptor for PreviewDescriptor {
    fn kind(&self) -> PaneKind {
        PaneKind::new("preview")
    }

    fn display_name(&self) -> SharedString {
        "Preview".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(PreviewState::default())
    }
}

/// Fluent builder for Preview mode.
#[derive(Clone, Debug, Default)]
pub struct PreviewBuilder {
    descriptor: PreviewDescriptor,
}

impl PreviewBuilder {
    pub fn new() -> Self {
        Self {
            descriptor: PreviewDescriptor::new(),
        }
    }

    pub fn build(self) -> PreviewDescriptor {
        self.descriptor
    }
}
