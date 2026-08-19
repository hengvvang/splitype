//! Tree-metadata flags on a block — kind-trait snapshots and pending
//! refresh requests.

use super::block::Block;

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
            flags |= 1;
        }
        if self.kind().is_list_item() {
            flags |= 2;
        }
        if self.kind().is_footnote_definition() {
            flags |= 4;
        }
        if self.kind().callout_kind().is_some() {
            flags |= 8;
        }
        flags
    }

    /// Whether the kind-derived metadata (quote depth, anchors, ordinals)
    /// stored on this block matches its current kind traits.
    pub(crate) fn is_tree_metadata_current(&self) -> bool {
        self.tree_metadata_flags == self.kind_metadata_flags()
    }
}
