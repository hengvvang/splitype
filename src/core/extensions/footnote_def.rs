//! Footnote definition binding, registry, and validation helpers.

use std::collections::HashMap;

use gpui::EntityId;
use uuid::Uuid;


/// Location of the first resolved inline reference for one footnote id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteReferenceLocation {
    pub entity_id: EntityId,
    pub occurrence_index: usize,
}

/// Definition block and first-reference metadata for one footnote id.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteDefinitionBinding {
    pub ordinal: Option<usize>,
    pub definition_entity_id: EntityId,
    pub first_reference: Option<FootnoteReferenceLocation>,
}

/// Document-wide registry that binds references to definition blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootnoteMap {
    pub bindings: HashMap<String, FootnoteDefinitionBinding>,
    pub block_occurrences: HashMap<Uuid, Vec<FootnoteResolvedOccurrence>>,
}

impl FootnoteMap {
    pub fn binding(&self, id: &str) -> Option<&FootnoteDefinitionBinding> {
        self.bindings.get(id)
    }

    pub fn ordinal(&self, id: &str) -> Option<usize> {
        self.binding(id).and_then(|binding| binding.ordinal)
    }

    pub fn occurrences_for_block(
        &self,
        block_id: Uuid,
    ) -> Option<&[FootnoteResolvedOccurrence]> {
        self.block_occurrences.get(&block_id).map(Vec::as_slice)
    }
}

/// Resolved occurrence stored per block for inline rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FootnoteResolvedOccurrence {
    pub id: String,
    pub ordinal: Option<usize>,
    pub occurrence_index: usize,
}

pub fn is_valid_footnote_id(id: &str) -> bool {
    !id.is_empty()
        && !id
            .chars()
            .any(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '^' | '[' | ']'))
}

pub fn parse_footnote_definition_head(line: &str) -> Option<(String, String)> {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return None;
    }

    let rest = &trimmed_end[leading_spaces..];
    let after_open = rest.strip_prefix("[^")?;
    let label_end = after_open.find("]:")?;
    let id = after_open[..label_end].to_string();
    if !is_valid_footnote_id(&id) {
        return None;
    }

    let remainder = after_open[label_end + 2..]
        .strip_prefix(' ')
        .unwrap_or(&after_open[label_end + 2..])
        .to_string();
    Some((id, remainder))
}

#[cfg(test)]
mod tests {
    use super::{is_valid_footnote_id, parse_footnote_definition_head};

    #[test]
    fn validates_footnote_ids() {
        assert!(is_valid_footnote_id("long-note"));
        assert!(!is_valid_footnote_id("bad id"));
    }

    #[test]
    fn parses_definition_head() {
        assert_eq!(
            parse_footnote_definition_head("[^ref-1]: body"),
            Some(("ref-1".to_string(), "body".to_string()))
        );
    }
}
