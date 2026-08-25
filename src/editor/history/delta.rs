//! Atomic document deltas and reversible editing transactions.
//!
//! Provides the data structures for fine-grained undo/redo: [`DocDelta`]
//! records atomic mutations to the block tree, while [`Transaction`] groups
//! related deltas with selection snapshots and provides an $O(1)$ [`Transaction::invert`]
//! operation to generate the exact reverse action.

use crate::editor::document::protocol::UndoCaptureKind;
use crate::editor::engine::controller::UndoSelectionSnapshot;
use crate::model::inline::text::BlockText;
use crate::model::parse::{BlockData, BlockId};

/// Atomic mutation applied to the document block tree.
#[derive(Clone, Debug)]
pub(crate) enum DocDelta {
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
pub(crate) struct Transaction {
    /// Atomic operations composing this transaction, in execution order.
    pub(crate) ops: Vec<DocDelta>,
    /// Selection snapshot before this transaction was applied.
    pub(crate) selection_before: UndoSelectionSnapshot,
    /// Selection snapshot after this transaction was applied.
    pub(crate) selection_after: UndoSelectionSnapshot,
    /// Capture categorization (coalescible typing vs structural edits).
    pub(crate) kind: UndoCaptureKind,
}

impl Transaction {
    /// Creates an empty transaction.
    pub(crate) fn empty() -> Self {
        Self {
            ops: Vec::new(),
            selection_before: UndoSelectionSnapshot::default(),
            selection_after: UndoSelectionSnapshot::default(),
            kind: UndoCaptureKind::NonCoalescible,
        }
    }

    /// Returns true if the transaction contains no operations.
    pub(crate) fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Inverts this transaction, producing an exact reverse transaction.
    ///
    /// Applying the inverted transaction will restore the document and
    /// selection state to exactly what it was prior to this transaction.
    pub(crate) fn invert(self) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_inversion_roundtrip() {
        let block_id = BlockId::new();
        let t1 = BlockText::plain("Hello");
        let t2 = BlockText::plain("Hello World");

        let tx = Transaction {
            ops: vec![
                DocDelta::UpdateBlockText {
                    block_id,
                    old_text: t1.clone(),
                    new_text: t2.clone(),
                },
                DocDelta::SpliceRoots {
                    index: 2,
                    deleted: vec![BlockData::paragraph("Old")],
                    inserted: vec![BlockData::paragraph("New")],
                },
            ],
            selection_before: UndoSelectionSnapshot::default(),
            selection_after: UndoSelectionSnapshot::default(),
            kind: UndoCaptureKind::CoalescibleText,
        };

        let inverted = tx.clone().invert();
        assert_eq!(inverted.ops.len(), 2);
        match &inverted.ops[0] {
            DocDelta::SpliceRoots {
                index,
                deleted,
                inserted,
            } => {
                assert_eq!(*index, 2);
                assert_eq!(deleted[0].text.plain_text(), "New");
                assert_eq!(inserted[0].text.plain_text(), "Old");
            }
            _ => panic!("unexpected op"),
        }

        let restored = inverted.invert();
        assert_eq!(restored.ops.len(), 2);
        match &restored.ops[0] {
            DocDelta::UpdateBlockText {
                block_id: bid,
                old_text,
                new_text,
            } => {
                assert_eq!(*bid, block_id);
                assert_eq!(*old_text, t1);
                assert_eq!(*new_text, t2);
            }
            _ => panic!("unexpected op"),
        }
    }
}
