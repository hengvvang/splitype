use gpui::App;

use crate::document::Document;
use crate::state::{FocusState, ReferenceRegistries, SelectionState, TableGrids};
use editor_model::{EditorDocument, EditorPaneKind, Pane};

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

impl Pane for WysiwygPaneState {
    fn kind(&self) -> EditorPaneKind {
        EditorPaneKind::Wysiwyg
    }

    fn document_source(&self, doc: &dyn EditorDocument, cx: &App) -> String {
        doc.serialize_markdown(cx)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
