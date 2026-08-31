//! Panel descriptor trait for registering panel plugins.

use std::sync::Arc;
use gpui::{App, SharedString};
use crate::layout::PanelId;
use crate::panel::{PanelHost, PanelKind, PanelView};

/// Factory descriptor for a Panel plugin.
pub trait PanelDescriptor: Send + Sync + 'static {
    /// The unique kind identifier.
    fn kind(&self) -> PanelKind;

    /// Human-readable display name.
    fn display_name(&self) -> SharedString;

    /// Icon path for this panel kind.
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Instantiates a new PanelView for a given PanelId.
    fn create_panel(
        &self,
        panel_id: PanelId,
        host: Option<Arc<dyn PanelHost>>,
        cx: &mut App,
    ) -> Box<dyn PanelView>;
}
