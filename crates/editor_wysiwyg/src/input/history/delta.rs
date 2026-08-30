//! Atomic document deltas and reversible editing transactions.
//!
//! Provides the data structures for fine-grained undo/redo: [`DocDelta`]
//! records atomic mutations to the block tree, while [`Transaction`] groups
//! related deltas with selection snapshots and provides an $O(1)$ [`Transaction::invert`]
//! operation to generate the exact reverse action.

use crate::document::protocol::UndoCaptureKind;
use crate::state::UndoSelectionSnapshot;
use crate::markdown::inline::text::BlockText;
use crate::markdown::parse::{BlockData, BlockId};

/// Atomic mutation applied to the document block tree.
#[derive(Clone, Debug)]
pub enum DocDelta {
    /// Update the text of a single block by its stable ID.
    UpdateBlockText {
        block_id: BlockId,
        old_text: BlockText,
        new_text: BlockText,
    },
    /// Splicing root blocks at an index: remove `deleted` and insert `inserted`.
    SpliceRoots {
        index: usize,
        deleted: Vec<BlockData>,
        inserted: Vec<BlockData>,
    },
}

/// A grouped sequence of atomic deltas representing one user-level action.
#[derive(Clone, Debug)]
pub struct Transaction {
    /// Atomic operations composing this transaction, in execution order.
    pub ops: Vec<DocDelta>,
    /// Selection snapshot before this transaction was applied.
    pub selection_before: UndoSelectionSnapshot,
    /// Selection snapshot after this transaction was applied.
    pub selection_after: UndoSelectionSnapshot,
    /// Capture categorization (coalescible typing vs structural edits).
    pub kind: UndoCaptureKind,
}

impl Transaction {
    /// Creates an empty transaction.
    pub fn empty() -> Self {
        Self {
            ops: Vec::new(),
            selection_before: UndoSelectionSnapshot::default(),
            selection_after: UndoSelectionSnapshot::default(),
            kind: UndoCaptureKind::NonCoalescible,
        }
    }

    /// Returns true if the transaction contains no operations.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Inverts this transaction, producing an exact reverse transaction.
    ///
    /// Applying the inverted transaction will restore the document and
    /// selection state to exactly what it was prior to this transaction.
    pub fn invert(self) -> Self {
        let mut inverted_ops = Vec::with_capacity(self.ops.len());
        for op in self.ops.into_iter().rev() {
            inverted_ops.push(match op {
                DocDelta::UpdateBlockText {
                    block_id,
                    old_text,
                    new_text,
                } => DocDelta::UpdateBlockText {
                    block_id,
                    old_text: new_text,
                    new_text: old_text,
                },
                DocDelta::SpliceRoots {
                    index,
                    deleted,
                    inserted,
                } => DocDelta::SpliceRoots {
                    index,
                    deleted: inserted,
                    inserted: deleted,
                },
            });
        }

        Self {
            ops: inverted_ops,
            selection_before: self.selection_after,
            selection_after: self.selection_before,
            kind: self.kind,
        }
    }
}

