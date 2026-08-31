use std::path::{Path, PathBuf};
use std::sync::Arc;
use gpui::{App, AppContext, Entity};
use editor_core::{Editor, EditorSession};
use editor_model::{PaneDescriptor, PaneRegistry};
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
        let mut registry = PaneRegistry::global().lock().unwrap();
        registry.register(Arc::new(editor_wysiwyg::WysiwygDescriptor::new()));
        registry.register(Arc::new(editor_source_code::SourceCodeDescriptor::new()));
        registry.register(Arc::new(editor_preview::PreviewDescriptor::new()));
    }

    /// Builds and creates the Editor entity within GPUI App context.
    pub fn build(self, cx: &mut App) -> Entity<Editor> {
        let panel_id = self.panel_id.unwrap_or(PanelId(workspace::DEFAULT_EDITOR_PANEL_ID));
        if let Some(session) = self.session {
            cx.new(|cx| Editor::with_session(panel_id, session, cx))
        } else {
            cx.new(|cx| Editor::new(self.initial_text, self.file_path, cx))
        }
    }

    /// Builds and creates the Editor entity within GPUI Context<T>.
    pub fn build_in<T: 'static>(self, cx: &mut gpui::Context<T>) -> Entity<Editor> {
        let panel_id = self.panel_id.unwrap_or(PanelId(workspace::DEFAULT_EDITOR_PANEL_ID));
        if let Some(session) = self.session {
            cx.new(|cx| Editor::with_session(panel_id, session, cx))
        } else {
            cx.new(|cx| Editor::new(self.initial_text, self.file_path, cx))
        }
    }
}
