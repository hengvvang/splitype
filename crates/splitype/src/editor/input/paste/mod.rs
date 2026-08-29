//! Classification helpers for rendered-mode external paste.
//!
//! File import keeps CommonMark paragraph semantics, but paste should match
//! the user's visual expectation more closely. Plain physical lines become
//! separate blocks, while structural Markdown and block/risky HTML are left to
//! the document block builder.

pub mod drop;
pub mod image;
pub mod quote;

use gpui::Context;
use gpui::Entity;

use crate::editor::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;
use markdown::inline::text::BlockText;
use markdown::parse::{BlockData, BlockKind};

impl Editor {
    /// Build one paragraph block per non-empty physical line, with a single
    /// empty block for an all-empty paste (so pasting blank text still
    /// produces an editable paragraph).
    pub(crate) fn build_plain_paste_blocks_from_lines(
        cx: &mut Context<Self>,
        lines: &[String],
    ) -> Vec<Entity<Block>> {
        let mut blocks = lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                Self::new_block(
                    cx,
                    BlockData::new(BlockKind::Paragraph, BlockText::from_markdown(line)),
                )
            })
            .collect::<Vec<_>>();

        if blocks.is_empty() && !lines.is_empty() {
            blocks.push(Self::new_block(
                cx,
                BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
            ));
        }

        blocks
    }
}
