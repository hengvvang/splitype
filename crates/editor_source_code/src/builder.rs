use gpui::SharedString;
use editor_model::{PaneDescriptor, PaneKindId, PaneView};
use crate::state::SourceCodeState;

/// Pane descriptor for Source Code mode.
#[derive(Clone, Debug, Default)]
pub struct SourceCodeDescriptor {}

impl SourceCodeDescriptor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn builder() -> SourceCodeBuilder {
        SourceCodeBuilder::new()
    }
}

impl PaneDescriptor for SourceCodeDescriptor {
    fn kind(&self) -> PaneKindId {
        PaneKindId::SOURCE_CODE
    }

    fn display_name(&self) -> SharedString {
        "Source Code".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(SourceCodeState::default())
    }
}

/// Fluent builder for Source Code mode.
#[derive(Clone, Debug, Default)]
pub struct SourceCodeBuilder {
    descriptor: SourceCodeDescriptor,
}

impl SourceCodeBuilder {
    pub fn new() -> Self {
        Self {
            descriptor: SourceCodeDescriptor::new(),
        }
    }

    pub fn build(self) -> SourceCodeDescriptor {
        self.descriptor
    }
}
