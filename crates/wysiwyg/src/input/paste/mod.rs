//! Classification and block construction helpers for external paste.

pub mod image;
pub mod plain;
pub mod quote;

pub use image::*;
pub use plain::*;
pub use quote::*;

use gpui::{App, AppContext, Entity};

use crate::model::block::Block;
use markdown_parser::inline::text::BlockText;
use markdown_parser::parse::{BlockData, BlockKind};

/// Build one paragraph block per non-empty physical line.
pub fn build_plain_paste_blocks_from_lines(lines: &[String], cx: &mut App) -> Vec<Entity<Block>> {
    let mut blocks = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            cx.new(|cx| {
                Block::with_data(
                    cx,
                    BlockData::new(BlockKind::Paragraph, BlockText::from_markdown(line)),
                )
            })
        })
        .collect::<Vec<_>>();

    if blocks.is_empty() && !lines.is_empty() {
        blocks.push(cx.new(|cx| {
            Block::with_data(
                cx,
                BlockData::new(BlockKind::Paragraph, BlockText::plain(String::new())),
            )
        }));
    }

    blocks
}
