//! Footnote definition binding, registry, and validation helpers.

use std::collections::HashMap;

/// Location of the first resolved inline reference for one footnote id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteReferenceLocation {
    pub entity_id: gpui::EntityId,
    pub occurrence_index: usize,
}

/// Definition block and first-reference metadata for one footnote id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteDefinitionBinding {
    pub definition_entity_id: gpui::EntityId,
    pub first_reference: Option<FootnoteReferenceLocation>,
}

/// Document-wide registry that binds references to definition blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteMap {
    pub bindings: HashMap<String, FootnoteDefinitionBinding>,
    pub block_occurrences: HashMap<crate::parse::BlockId, Vec<FootnoteResolvedOccurrence>>,
}

impl FootnoteMap {
    pub fn binding(&self, id: &str) -> Option<&FootnoteDefinitionBinding> {
        self.bindings.get(id)
    }

    pub fn occurrences_for_block(
        &self,
        block_id: crate::parse::BlockId,
    ) -> Option<&[FootnoteResolvedOccurrence]> {
        self.block_occurrences.get(&block_id).map(Vec::as_slice)
    }
}

/// Resolved occurrence stored per block for inline rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FootnoteResolvedOccurrence {
    pub id: String,
    pub occurrence_index: usize,
}

