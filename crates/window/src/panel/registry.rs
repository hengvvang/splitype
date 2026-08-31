//! Global panel plugin registry.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use gpui::App;
use crate::layout::PanelId;
use crate::panel::{PanelDescriptor, PanelHost, PanelKind, PanelView};

/// Thread-safe registry holding all available PanelDescriptors.
#[derive(Default)]
pub struct PanelRegistry {
    descriptors: HashMap<PanelKind, Arc<dyn PanelDescriptor>>,
    order: Vec<PanelKind>,
    default_kind: Option<PanelKind>,
    primary_kind: Option<PanelKind>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<PanelRegistry>> =
            LazyLock::new(|| Mutex::new(PanelRegistry::new()));
        &REGISTRY
    }

    pub fn register(&mut self, descriptor: Arc<dyn PanelDescriptor>, is_primary: bool) {
        let kind = descriptor.kind();
        if !self.descriptors.contains_key(&kind) {
            self.order.push(kind);
        }
        if is_primary || self.primary_kind.is_none() {
            self.primary_kind = Some(kind);
        }
        if self.default_kind.is_none() {
            self.default_kind = Some(kind);
        }
        self.descriptors.insert(kind, descriptor);
    }

    pub fn get(&self, kind: PanelKind) -> Option<Arc<dyn PanelDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn create_panel(
        &self,
        kind: PanelKind,
        panel_id: PanelId,
        host: Option<Arc<dyn PanelHost>>,
        cx: &mut App,
    ) -> Option<Box<dyn PanelView>> {
        self.descriptors.get(&kind).map(|d| d.create_panel(panel_id, host, cx))
    }

    pub fn default_kind(&self) -> Option<PanelKind> {
        self.default_kind.or_else(|| self.order.first().copied())
    }

    pub fn primary_kind(&self) -> Option<PanelKind> {
        self.primary_kind.or_else(|| self.default_kind())
    }

    pub fn all_kinds(&self) -> &[PanelKind] {
        &self.order
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PanelDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }
}
