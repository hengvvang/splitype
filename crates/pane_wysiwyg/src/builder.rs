use crate::plugin::WysiwygPaneState;
use core_contracts::{PaneDescriptor, PaneKind, PaneView};
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

    pub fn builder() -> WysiwygBuilder {
        WysiwygBuilder::new()
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
