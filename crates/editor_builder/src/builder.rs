use std::path::{Path, PathBuf};
use std::sync::Arc;
use gpui::{App, AppContext, Entity, SharedString};
use editor_core::{Editor, EditorSession};
use editor_model::{PaneDescriptor, PaneKindId, PaneRegistry, PaneView};
use workspace::PanelId;

/// Fluent builder for assembling and instantiating Editor entities.
pub struct EditorBuilder {
    initial_text: String,
    file_path: Option<PathBuf>,
    session: Option<EditorSession>,
    panel_id: Option<PanelId>,
}

impl Default for EditorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuilder {
    /// Creates a new editor builder with empty content.
    pub fn new() -> Self {
        Self {
            initial_text: String::new(),
            file_path: None,
            session: None,
            panel_id: None,
        }
    }

    /// Sets the initial raw markdown text.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.initial_text = text.into();
        self
    }

    /// Sets the initial file path.
    pub fn with_file_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Sets an existing EditorSession.
    pub fn with_session(mut self, session: EditorSession) -> Self {
        self.session = Some(session);
        self
    }

    /// Sets the panel id.
    pub fn with_panel_id(mut self, panel_id: PanelId) -> Self {
        self.panel_id = Some(panel_id);
        self
    }

    /// Loads text from a file path.
    pub fn from_file_path(mut self, path: &Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        self.initial_text = text;
        self.file_path = Some(path.to_path_buf());
        Ok(self)
    }

    /// Registers a pane descriptor into the global registry.
    pub fn register_pane<D: PaneDescriptor>(self, descriptor: D) -> Self {
        let mut registry = PaneRegistry::global().lock().unwrap();
        registry.register(Arc::new(descriptor));
        self
    }

    /// Registers all built-in default pane descriptors (WYSIWYG, Source Code, Preview).
    pub fn register_default_panes(self) -> Self {
        Self::register_defaults();
        self
    }

    /// Registers built-in pane descriptors statically.
    pub fn register_defaults() {
        struct WysiwygDescriptor;
        impl PaneDescriptor for WysiwygDescriptor {
            fn kind(&self) -> PaneKindId {
                PaneKindId::WYSIWYG
            }
            fn display_name(&self) -> SharedString {
                "WYSIWYG".into()
            }
            fn create_pane(&self) -> Box<dyn PaneView> {
                Box::new(editor_wysiwyg::WysiwygPaneState::default())
            }
        }

        struct SourceCodeDescriptor;
        impl PaneDescriptor for SourceCodeDescriptor {
            fn kind(&self) -> PaneKindId {
                PaneKindId::SOURCE_CODE
            }
            fn display_name(&self) -> SharedString {
                "Source Code".into()
            }
            fn create_pane(&self) -> Box<dyn PaneView> {
                Box::new(editor_source_code::SourceCodeState::default())
            }
        }

        struct PreviewDescriptor;
        impl PaneDescriptor for PreviewDescriptor {
            fn kind(&self) -> PaneKindId {
                PaneKindId::PREVIEW
            }
            fn display_name(&self) -> SharedString {
                "Preview".into()
            }
            fn create_pane(&self) -> Box<dyn PaneView> {
                Box::new(editor_preview::PreviewState::default())
            }
        }

        let mut registry = PaneRegistry::global().lock().unwrap();
        registry.register(Arc::new(WysiwygDescriptor));
        registry.register(Arc::new(SourceCodeDescriptor));
        registry.register(Arc::new(PreviewDescriptor));
    }

    /// Builds and creates the Editor entity within GPUI.
    pub fn build(self, cx: &mut App) -> Entity<Editor> {
        let panel_id = self.panel_id.unwrap_or(PanelId(workspace::DEFAULT_EDITOR_PANEL_ID));
        if let Some(session) = self.session {
            cx.new(|cx| Editor::with_session(panel_id, session, cx))
        } else {
            cx.new(|cx| Editor::new(self.initial_text, self.file_path, cx))
        }
    }
}
