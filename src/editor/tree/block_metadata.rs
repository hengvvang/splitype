//! Tree-metadata flags on a block — kind-trait snapshots and pending
//! refresh requests.

use super::block::Block;

pub(crate) const FLAG_QUOTE_CONTAINER: u8 = 1 << 0;
pub(crate) const FLAG_LIST_ITEM: u8 = 1 << 1;
pub(crate) const FLAG_FOOTNOTE_DEF: u8 = 1 << 2;
pub(crate) const FLAG_CALLOUT: u8 = 1 << 3;

impl Block {
    pub(crate) fn take_quote_reparse_requested(&mut self) -> bool {
        let requested = self.quote_reparse_requested;
        self.quote_reparse_requested = false;
        requested
    }

    pub(crate) fn take_numbered_list_restart_requested(&mut self) -> bool {
        let requested = self.numbered_list_restart_requested;
        self.numbered_list_restart_requested = false;
        requested
    }

    pub(crate) fn kind_metadata_flags(&self) -> u8 {
        let mut flags = 0u8;
        if self.kind().is_quote_container() {
            flags |= FLAG_QUOTE_CONTAINER;
        }
        if self.kind().is_list_item() {
            flags |= FLAG_LIST_ITEM;
        }
        if self.kind().is_footnote_definition() {
            flags |= FLAG_FOOTNOTE_DEF;
        }
        if self.kind().callout_kind().is_some() {
            flags |= FLAG_CALLOUT;
        }
        flags
    }

    /// Whether the kind-derived metadata (quote depth, anchors, ordinals)
    /// stored on this block matches its current kind traits.
    pub(crate) fn is_tree_metadata_current(&self) -> bool {
        self.tree_metadata_flags == self.kind_metadata_flags()
    }
}
