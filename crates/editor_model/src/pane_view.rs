//! The universal plugin contract for editor panes.

use std::any::Any;

use gpui::{AnyElement, App, FocusHandle, Window};

use crate::{EditorDocument, PaneKindId, PaneRenderContext};

/// The plugin contract implemented by every editor pane kind.
pub trait PaneView: Any + Send + Sync + 'static {
    /// Which pane kind this state belongs to.
    fn kind(&self) -> PaneKindId;

    /// Pure markdown source of the active tab, as this mode sees it.
    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String;

    /// Render the pane's inner body element.
    fn render(
        &mut self,
        ctx: &PaneRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;

    /// Focus handle owned by this pane (if any).
    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// Hook called when the shared document text has changed.
    fn on_document_changed(&mut self, _new_text: &str, _cx: &mut App) {}

    /// Type-erased access for downcasting to the concrete mode state.
    fn as_any(&self) -> &dyn Any;

    /// Type-erased mutable access for downcasting to the concrete mode state.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
