//! Workspace-level universal panel plugin SPI (Service Provider Interface).
//!
//! Enables modular and decoupled registration of window-level panels (Editor,
//! Explorer, Settings, and custom third-party panels) mirroring the internal
//! pane plugin architecture.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use gpui::{AnyElement, App, FocusHandle, SharedString, Window};
use theme::Theme;
use config::language::I18nStrings;
use crate::panels::PanelId;

/// Strongly-typed, extensible identifier for a workspace-level panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PanelKindId(pub &'static str);

impl PanelKindId {
    pub const EDITOR: Self = Self("splitype.editor");
    pub const EXPLORER: Self = Self("splitype.explorer");
    pub const SETTINGS: Self = Self("splitype.settings");

    #[inline]
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for PanelKindId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Rendering context passed to a [`PanelView`] on every render frame.
pub struct PanelRenderContext<'a> {
    pub panel_id: PanelId,
    pub leaf_count: usize,
    pub is_maximized: bool,
    pub theme: &'a Theme,
    pub strings: &'a I18nStrings,
}

/// Universal trait contract that any top-level window panel must implement.
pub trait PanelView: 'static {
    /// The unique identifier of this panel's kind.
    fn kind(&self) -> PanelKindId;

    /// The display name shown in tabs, topbars or dropdown menus.
    fn display_name(&self) -> SharedString;

    /// The icon asset path for this panel (if any).
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Renders the complete panel UI inside the workspace tile container.
    fn render(
        &mut self,
        ctx: &PanelRenderContext,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement;

    /// The FocusHandle owned by this panel for keyboard navigation.
    fn focus_handle(&self, _cx: &App) -> Option<FocusHandle> {
        None
    }

    /// Upcast to Any for downcasting to specific panel types when needed.
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Factory descriptor for a Panel plugin.
pub trait PanelDescriptor: Send + Sync + 'static {
    /// The unique kind identifier.
    fn kind(&self) -> PanelKindId;

    /// Human-readable display name.
    fn display_name(&self) -> SharedString;

    /// Icon path for this panel kind.
    fn icon(&self) -> Option<&'static str> {
        None
    }

    /// Instantiates a new PanelView for a given PanelId.
    fn create_panel(&self, panel_id: PanelId, cx: &mut App) -> Box<dyn PanelView>;
}

/// Thread-safe registry holding all available PanelDescriptors.
#[derive(Default)]
pub struct PanelRegistry {
    descriptors: HashMap<PanelKindId, Arc<dyn PanelDescriptor>>,
    order: Vec<PanelKindId>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Mutex<Self> {
        static REGISTRY: OnceLock<Mutex<PanelRegistry>> = OnceLock::new();
        REGISTRY.get_or_init(|| Mutex::new(PanelRegistry::new()))
    }

    pub fn register(&mut self, descriptor: Arc<dyn PanelDescriptor>) {
        let kind = descriptor.kind();
        if !self.descriptors.contains_key(&kind) {
            self.order.push(kind);
        }
        self.descriptors.insert(kind, descriptor);
    }

    pub fn get(&self, kind: PanelKindId) -> Option<Arc<dyn PanelDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn create_panel(
        &self,
        kind: PanelKindId,
        panel_id: PanelId,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        self.descriptors.get(&kind).map(|d| d.create_panel(panel_id, cx))
    }

    pub fn all_kinds(&self) -> &[PanelKindId] {
        &self.order
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PanelDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }
}
