use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use thiserror::Error;

use crate::document::DocumentSnapshot;
use crate::pane::{PaneDescriptor, PaneKind, PaneView};

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PaneRegistryError {
    #[error("pane kind '{0}' is already registered")]
    DuplicateKind(PaneKind),
    #[error("pane registry lock is poisoned")]
    Poisoned,
}

/// Takeover delegation target when a pane is intercepted based on document conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaneTakeover {
    /// The pane kind to delegate rendering and input to.
    pub target_kind: PaneKind,
    /// Whether the delegated pane should operate in read-only mode.
    pub read_only: bool,
}

pub type TakeoverPredicate = Arc<dyn Fn(&DocumentSnapshot) -> bool + Send + Sync>;

#[derive(Clone)]
struct TakeoverRule {
    target_kind: PaneKind,
    read_only: bool,
    predicate: TakeoverPredicate,
}

#[derive(Default)]
pub struct PaneRegistry {
    descriptors: HashMap<PaneKind, Arc<dyn PaneDescriptor>>,
    takeover_rules: HashMap<PaneKind, Vec<TakeoverRule>>,
    order: Vec<PaneKind>,
    default_kind: Option<PaneKind>,
}

impl PaneRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn global() -> &'static Mutex<Self> {
        static REGISTRY: LazyLock<Mutex<PaneRegistry>> =
            LazyLock::new(|| Mutex::new(PaneRegistry::new()));
        &REGISTRY
    }

    pub fn register(
        &mut self,
        descriptor: Arc<dyn PaneDescriptor>,
        is_default: bool,
    ) -> Result<(), PaneRegistryError> {
        // Query plugin metadata before mutating the registry. A descriptor callback
        // must never observe a half-completed registration.
        let kind = descriptor.kind();
        if self.descriptors.contains_key(&kind) {
            return Err(PaneRegistryError::DuplicateKind(kind));
        }

        self.order.push(kind.clone());
        if is_default || self.default_kind.is_none() {
            self.default_kind = Some(kind.clone());
        }
        self.descriptors.insert(kind, descriptor);
        Ok(())
    }

    pub fn register_global(
        descriptor: Arc<dyn PaneDescriptor>,
        is_default: bool,
    ) -> Result<(), PaneRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .register(descriptor, is_default)
    }

    pub fn get(&self, kind: PaneKind) -> Option<Arc<dyn PaneDescriptor>> {
        self.descriptors.get(&kind).cloned()
    }

    pub fn registered(
        kind: PaneKind,
    ) -> Result<Option<Arc<dyn PaneDescriptor>>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .get(kind))
    }

    pub fn create_registered(
        kind: PaneKind,
    ) -> Result<Option<Box<dyn PaneView>>, PaneRegistryError> {
        // Clone the descriptor while locked, then execute third-party factory code
        // only after the guard has been released. This permits safe re-entry.
        let descriptor = Self::registered(kind)?;
        Ok(descriptor.map(|descriptor| descriptor.create_pane()))
    }

    pub fn default_kind(&self) -> Option<PaneKind> {
        self.default_kind
            .clone()
            .or_else(|| self.order.first().cloned())
    }

    pub fn registered_default_kind() -> Result<Option<PaneKind>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .default_kind())
    }

    pub fn all_descriptors(&self) -> Vec<Arc<dyn PaneDescriptor>> {
        self.order
            .iter()
            .filter_map(|kind| self.descriptors.get(kind).cloned())
            .collect()
    }

    pub fn registered_descriptors() -> Result<Vec<Arc<dyn PaneDescriptor>>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .all_descriptors())
    }

    /// Registers a takeover rule intercepting `source_kind` when `predicate(doc)` is true.
    pub fn register_takeover(
        &mut self,
        source_kind: PaneKind,
        target_kind: PaneKind,
        read_only: bool,
        predicate: Arc<dyn Fn(&DocumentSnapshot) -> bool + Send + Sync>,
    ) {
        self.takeover_rules
            .entry(source_kind)
            .or_default()
            .push(TakeoverRule {
                target_kind,
                read_only,
                predicate,
            });
    }

    /// Registers a takeover rule in the process-global pane registry.
    pub fn register_takeover_global(
        source_kind: PaneKind,
        target_kind: PaneKind,
        read_only: bool,
        predicate: Arc<dyn Fn(&DocumentSnapshot) -> bool + Send + Sync>,
    ) -> Result<(), PaneRegistryError> {
        Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .register_takeover(source_kind, target_kind, read_only, predicate);
        Ok(())
    }

    /// Resolves whether `kind` is taken over by another pane for the given document snapshot.
    pub fn resolve_takeover(
        &self,
        kind: &PaneKind,
        doc: &DocumentSnapshot,
    ) -> Option<PaneTakeover> {
        if let Some(rules) = self.takeover_rules.get(kind) {
            for rule in rules {
                if (rule.predicate)(doc) {
                    return Some(PaneTakeover {
                        target_kind: rule.target_kind.clone(),
                        read_only: rule.read_only,
                    });
                }
            }
        }
        None
    }

    /// Resolves whether `kind` is taken over using the process-global registry.
    pub fn resolve_takeover_global(
        kind: &PaneKind,
        doc: &DocumentSnapshot,
    ) -> Result<Option<PaneTakeover>, PaneRegistryError> {
        Ok(Self::global()
            .lock()
            .map_err(|_| PaneRegistryError::Poisoned)?
            .resolve_takeover(kind, doc))
    }
}

#[cfg(test)]
mod tests {
    use gpui::SharedString;

    use super::*;

    struct TestDescriptor(PaneKind);

    impl PaneDescriptor for TestDescriptor {
        fn kind(&self) -> PaneKind {
            self.0.clone()
        }

        fn display_name(&self) -> SharedString {
            "Test".into()
        }

        fn create_pane(&self) -> Box<dyn PaneView> {
            panic!("factory is not needed by this test")
        }
    }

    #[test]
    fn duplicate_kinds_are_rejected_without_changing_order() {
        let mut registry = PaneRegistry::new();
        let kind = PaneKind::new("test.pane");
        registry
            .register(Arc::new(TestDescriptor(kind.clone())), true)
            .unwrap();

        assert_eq!(
            registry.register(Arc::new(TestDescriptor(kind.clone())), false),
            Err(PaneRegistryError::DuplicateKind(kind.clone()))
        );
        assert_eq!(registry.default_kind(), Some(kind));
        assert_eq!(registry.all_descriptors().len(), 1);
    }

    #[test]
    fn takeover_rules_intercept_matching_documents() {
        let mut registry = PaneRegistry::new();
        let wysiwyg = PaneKind::new("splitype.pane.wysiwyg");
        let source_code = PaneKind::new("splitype.pane.source_code");

        registry.register_takeover(
            wysiwyg.clone(),
            source_code.clone(),
            false,
            Arc::new(|doc| !doc.is_markdown()),
        );

        let md_doc = DocumentSnapshot::empty().with_path(std::path::PathBuf::from("test.md"));
        let rs_doc = DocumentSnapshot::empty().with_path(std::path::PathBuf::from("test.rs"));

        assert_eq!(registry.resolve_takeover(&wysiwyg, &md_doc), None);
        assert_eq!(
            registry.resolve_takeover(&wysiwyg, &rs_doc),
            Some(PaneTakeover {
                target_kind: source_code,
                read_only: false,
            })
        );
    }
}
