use crate::panels::{PanelId, WindowPanelKind};

/// Fluent builder for the Workspace shell.
pub struct WorkspaceBuilder {
    default_panel_id: PanelId,
    default_panel_kind: WindowPanelKind,
    dock_ratio: f32,
}

impl Default for WorkspaceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceBuilder {
    pub fn new() -> Self {
        Self {
            default_panel_id: PanelId(crate::DEFAULT_EDITOR_PANEL_ID),
            default_panel_kind: WindowPanelKind::Editor,
            dock_ratio: 0.2,
        }
    }

    pub fn with_default_panel(mut self, id: PanelId, kind: WindowPanelKind) -> Self {
        self.default_panel_id = id;
        self.default_panel_kind = kind;
        self
    }

    pub fn with_dock_ratio(mut self, ratio: f32) -> Self {
        self.dock_ratio = ratio;
        self
    }

    pub fn default_panel_id(&self) -> PanelId {
        self.default_panel_id
    }

    pub fn default_panel_kind(&self) -> WindowPanelKind {
        self.default_panel_kind
    }

    pub fn dock_ratio(&self) -> f32 {
        self.dock_ratio
    }
}
