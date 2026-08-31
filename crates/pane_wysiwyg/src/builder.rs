use gpui::SharedString;
use core_contracts::{PaneDescriptor, PaneKind, PaneView};
use crate::plugin::WysiwygPaneState;

/// Pane descriptor for WYSIWYG mode.
#[derive(Clone, Debug, Default)]
pub struct WysiwygDescriptor {}

impl WysiwygDescriptor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn builder() -> WysiwygBuilder {
        WysiwygBuilder::new()
    }
}

impl PaneDescriptor for WysiwygDescriptor {
    fn kind(&self) -> PaneKind {
        PaneKind::new("wysiwyg")
    }

    fn display_name(&self) -> SharedString {
        "WYSIWYG".into()
    }

    fn create_pane(&self) -> Box<dyn PaneView> {
        Box::new(WysiwygPaneState::default())
    }
}

/// Fluent builder for WYSIWYG mode.
#[derive(Clone, Debug, Default)]
pub struct WysiwygBuilder {
    descriptor: WysiwygDescriptor,
}

impl WysiwygBuilder {
    pub fn new() -> Self {
        Self {
            descriptor: WysiwygDescriptor::new(),
        }
    }

    pub fn build(self) -> WysiwygDescriptor {
        self.descriptor
    }
}


