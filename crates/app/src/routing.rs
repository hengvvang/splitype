//! Composition-root service wiring: maps panel kinds to the roles their
//! plugins implement.
//!
//! The shell talks to panels only through generic role traits
//! ([`DocumentPanel`]) or through plugin-exported hook functions. Casting a
//! `dyn PanelView` to a concrete role requires knowing the concrete view
//! type, which only the implementing plugin has — so each plugin exports its
//! adapter functions and the composition root registers them here by kind.
//! Nothing outside this crate and `plugins.rs` ever imports a concrete panel
//! view type.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use editor_contracts::DocumentPanel;
use gpui::{App, Window};
use platform_contracts::{PanelKind, PanelView};

/// Adapter casting a panel view to its document role. Exported by the
/// document plugin (only it knows its concrete view type) and registered by
/// the composition root for its kind.
#[derive(Clone, Copy)]
pub(crate) struct DocumentRouting {
    pub(crate) as_document: fn(&dyn PanelView) -> Option<&dyn DocumentPanel>,
    pub(crate) as_document_mut: fn(&mut dyn PanelView) -> Option<&mut dyn DocumentPanel>,
}

#[derive(Default)]
struct DocumentRoutingTable {
    by_kind: HashMap<PanelKind, DocumentRouting>,
    primary_kind: Option<PanelKind>,
}

impl DocumentRoutingTable {
    fn register(&mut self, kind: PanelKind, routing: DocumentRouting, is_primary: bool) {
        if is_primary || self.primary_kind.is_none() {
            self.primary_kind = Some(kind.clone());
        }
        self.by_kind.insert(kind, routing);
    }

    fn routing(&self, kind: &PanelKind) -> Option<DocumentRouting> {
        self.by_kind.get(kind).copied()
    }
}

fn document_table() -> &'static Mutex<DocumentRoutingTable> {
    static TABLE: LazyLock<Mutex<DocumentRoutingTable>> =
        LazyLock::new(|| Mutex::new(DocumentRoutingTable::default()));
    &TABLE
}

/// Registers the document-role adapter for `kind`. `is_primary` marks the
/// kind the default window layout should prefer.
pub(crate) fn register_document_routing(
    kind: PanelKind,
    routing: DocumentRouting,
    is_primary: bool,
) {
    document_table()
        .lock()
        .expect("document routing table lock poisoned")
        .register(kind, routing, is_primary);
}

/// The document-role adapter for `kind`, if its plugin registered one.
pub(crate) fn document_routing(kind: &PanelKind) -> Option<DocumentRouting> {
    document_table()
        .lock()
        .expect("document routing table lock poisoned")
        .routing(kind)
}

/// Whether panels of `kind` route documents.
pub(crate) fn is_document_kind(kind: &PanelKind) -> bool {
    document_routing(kind).is_some()
}

/// The preferred document panel kind for the default window layout.
pub(crate) fn primary_document_kind() -> Option<PanelKind> {
    document_table()
        .lock()
        .expect("document routing table lock poisoned")
        .primary_kind
        .clone()
}

/// Hooks the shell uses to push document context and explorer commands into
/// panel views of the explorer plugin's kind. Exported by the explorer and
/// registered by the composition root.
#[derive(Clone)]
pub(crate) struct ExplorerHooks {
    pub(crate) kind: PanelKind,
    pub(crate) set_active_document_path: fn(&mut dyn PanelView, Option<PathBuf>, &mut App),
    pub(crate) on_document_path_changed: fn(&mut dyn PanelView, &mut App),
    pub(crate) toggle_tree: fn(&mut dyn PanelView, &mut Window, &mut App),
    pub(crate) close_folder_scope: fn(&mut dyn PanelView, &mut App),
}

fn explorer_hooks_slot() -> &'static LazyLock<Mutex<Option<ExplorerHooks>>> {
    static HOOKS: LazyLock<Mutex<Option<ExplorerHooks>>> = LazyLock::new(|| Mutex::new(None));
    &HOOKS
}

/// Registers the explorer plugin's hooks. Called exactly once at startup.
pub(crate) fn register_explorer_hooks(hooks: ExplorerHooks) {
    let mut slot = explorer_hooks_slot()
        .lock()
        .expect("explorer hooks lock poisoned");
    assert!(
        slot.is_none(),
        "explorer hooks must be registered exactly once"
    );
    *slot = Some(hooks);
}

/// The registered explorer hooks, if the explorer plugin is present.
pub(crate) fn explorer_hooks() -> Option<ExplorerHooks> {
    explorer_hooks_slot()
        .lock()
        .expect("explorer hooks lock poisoned")
        .clone()
}

/// The explorer plugin's panel kind, if it is present.
pub(crate) fn explorer_kind() -> Option<PanelKind> {
    explorer_hooks().map(|hooks| hooks.kind)
}
