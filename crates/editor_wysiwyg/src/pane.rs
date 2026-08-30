use gpui::{App, IntoElement};

use crate::document::Document;
use crate::state::{FocusState, ReferenceRegistries, SelectionState, TableGrids};
use editor_model::{EditorDocument, PaneKindId, PaneRenderContext, PaneView};

/// View state specific to a WYSIWYG editor pane.
#[derive(Default)]
pub struct WysiwygPaneState {
    pub focus: FocusState,
    pub selection: SelectionState,
    pub document: Option<Document>,
    pub tables: TableGrids,
    pub references: ReferenceRegistries,
    pub text_stale: bool,
}

impl PaneView for WysiwygPaneState {
    fn kind(&self) -> PaneKindId {
        PaneKindId::WYSIWYG
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        if let Some(document) = &self.document {
            if self.text_stale {
                return document.serialize_markdown(cx);
            }
        }
        doc.serialize_markdown(cx)
    }

    fn render(
        &mut self,
        _ctx: &PaneRenderContext,
        _window: &mut gpui::Window,
        _cx: &mut App,
    ) -> gpui::AnyElement {
        gpui::div().into_any_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
