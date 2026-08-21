//! Atomic document deltas and reversible editing transactions.
//!
//! Provides the data structures for fine-grained undo/redo: [`DocDelta`]
//! records atomic mutations to the block tree, while [`Transaction`] groups
//! related deltas with selection snapshots and provides an $O(1)$ [`Transaction::invert`]
//! operation to generate the exact reverse action.

use std::time::Instant;

use crate::editor::block_protocol::UndoCaptureKind;
use crate::editor::controller::UndoSelectionSnapshot;
use crate::model::block::table::TableData;
use crate::model::inline::text::BlockText;
use crate::model::parse::{BlockData, BlockId};

/// Atomic mutation applied to the document block tree.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum DocDelta {
    /// Insert a new block at `index`.
    InsertBlock {
        index: usize,
        block: BlockData,
    },
    /// Remove an existing block at `index`.
    RemoveBlock {
        index: usize,
        old_block: BlockData,
    },
    /// Replace the block at `index` with new data (e.g. heading level change).
    ReplaceBlock {
        index: usize,
        old_block: BlockData,
        new_block: BlockData,
    },
    /// Update the text of a single block by its stable ID.
    UpdateBlockText {
        block_id: BlockId,
        old_text: BlockText,
        new_text: BlockText,
    },
    /// Update the native table structure and contents of a table block.
    UpdateTable {
        block_id: BlockId,
        old_table: TableData,
        new_table: TableData,
    },
    /// Reorder a block from `from_index` to `to_index`.
    MoveBlock {
        from_index: usize,
        to_index: usize,
    },
}

/// A grouped sequence of atomic deltas representing one user-level action.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct Transaction {
    /// Atomic operations composing this transaction, in execution order.
    pub(crate) ops: Vec<DocDelta>,
    /// Selection snapshot before this transaction was applied.
    pub(crate) selection_before: UndoSelectionSnapshot,
    /// Selection snapshot after this transaction was applied.
    pub(crate) selection_after: UndoSelectionSnapshot,
    /// Creation timestamp for coalesce window calculation.
    pub(crate) timestamp: Instant,
    /// Capture categorization (coalescible typing vs structural edits).
    pub(crate) kind: UndoCaptureKind,
}

#[allow(dead_code)]
impl Transaction {
    /// Creates a new transaction with the given operations and selection snapshots.
    pub(crate) fn new(
        ops: Vec<DocDelta>,
        selection_before: UndoSelectionSnapshot,
        selection_after: UndoSelectionSnapshot,
        kind: UndoCaptureKind,
    ) -> Self {
        Self {
            ops,
            selection_before,
            selection_after,
            timestamp: Instant::now(),
            kind,
        }
    }

    /// Creates an empty transaction.
    pub(crate) fn empty() -> Self {
        Self {
            ops: Vec::new(),
            selection_before: UndoSelectionSnapshot::default(),
            selection_after: UndoSelectionSnapshot::default(),
            timestamp: Instant::now(),
            kind: UndoCaptureKind::NonCoalescible,
        }
    }

    /// Returns true if the transaction contains no operations.
    pub(crate) fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Single block text update helper.
    pub(crate) fn text_update(
        block_id: BlockId,
        old_text: BlockText,
        new_text: BlockText,
        selection_before: UndoSelectionSnapshot,
        selection_after: UndoSelectionSnapshot,
        kind: UndoCaptureKind,
    ) -> Self {
        Self::new(
            vec![DocDelta::UpdateBlockText {
                block_id,
                old_text,
                new_text,
            }],
            selection_before,
            selection_after,
            kind,
        )
    }

    /// Inverts this transaction, producing an exact reverse transaction.
    ///
    /// Applying the inverted transaction will restore the document and
    /// selection state to exactly what it was prior to this transaction.
    pub(crate) fn invert(self) -> Self {
        let mut inverted_ops = Vec::with_capacity(self.ops.len());
        for op in self.ops.into_iter().rev() {
            inverted_ops.push(match op {
                DocDelta::InsertBlock { index, block } => {
                    DocDelta::RemoveBlock {
                        index,
                        old_block: block,
                    }
                }
                DocDelta::RemoveBlock { index, old_block } => {
                    DocDelta::InsertBlock {
                        index,
                        block: old_block,
                    }
                }
                DocDelta::ReplaceBlock {
                    index,
                    old_block,
                    new_block,
                } => DocDelta::ReplaceBlock {
                    index,
                    old_block: new_block,
                    new_block: old_block,
                },
                DocDelta::UpdateBlockText {
                    block_id,
                    old_text,
                    new_text,
                } => DocDelta::UpdateBlockText {
                    block_id,
                    old_text: new_text,
                    new_text: old_text,
                },
                DocDelta::UpdateTable {
                    block_id,
                    old_table,
                    new_table,
                } => DocDelta::UpdateTable {
                    block_id,
                    old_table: new_table,
                    new_table: old_table,
                },
                DocDelta::MoveBlock {
                    from_index,
                    to_index,
                } => DocDelta::MoveBlock {
                    from_index: to_index,
                    to_index: from_index,
                },
            });
        }

        Self {
            ops: inverted_ops,
            selection_before: self.selection_after,
            selection_after: self.selection_before,
            timestamp: Instant::now(),
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

        let tx = Transaction::text_update(
            block_id,
            t1.clone(),
            t2.clone(),
            UndoSelectionSnapshot::default(),
            UndoSelectionSnapshot::default(),
            UndoCaptureKind::CoalescibleText,
        );

        let inverted = tx.clone().invert();
        assert_eq!(inverted.ops.len(), 1);
        match &inverted.ops[0] {
            DocDelta::UpdateBlockText {
                block_id: bid,
                old_text,
                new_text,
            } => {
                assert_eq!(*bid, block_id);
                assert_eq!(*old_text, t2);
                assert_eq!(*new_text, t1);
            }
            _ => panic!("unexpected op"),
        }

        let restored = inverted.invert();
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
