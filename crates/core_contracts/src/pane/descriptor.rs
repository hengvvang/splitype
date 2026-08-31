use gpui::SharedString;
use crate::pane::{PaneKind, PaneView};

pub trait PaneDescriptor: Send + Sync + 'static {
    fn kind(&self) -> PaneKind;
    fn display_name(&self) -> SharedString;
    fn icon_path(&self) -> Option<SharedString> {
        None
    }
    fn create_pane(&self) -> Box<dyn PaneView>;
}
