//! Rendered-quote metadata refresh — detecting when a pure text edit inside
//! a quote needs the tree metadata (depths, anchors, ordinals) rebuilt.
//!
//! Pure text edits never change tree metadata — only structural edits do
//! (those already rebuild via `with_structure_mutation`/`replace_blocks`),
//! or kind changes on the edited block (tracked by its kind-trait snapshot).
//! Skipping the full-tree DFS while both are current keeps typing inside
//! quotes from paying an O(document) cost per keystroke.

use gpui::*;

use crate::editor::controller::{Editor, EditorMode};
use crate::editor::tree::block::Block;
use crate::model::block::BlockKind;

impl Editor {
    pub(crate) fn block_is_quote_structure_related(&self, block: &Entity<Block>, cx: &App) -> bool {
        if self.tab().mode != EditorMode::Wysiwyg {
            return false;
        }

        let block_ref = block.read(cx);
        block_ref.kind().is_quote_container()
            || block_ref.quote_depth > 0
            || block_ref.quote_group_id.is_some()
    }

    pub(crate) fn refresh_rendered_quote_metadata_if_needed(
        &mut self,
        block: &Entity<Block>,
        cx: &mut Context<Self>,
    ) {
        if !self.block_is_quote_structure_related(block, cx) {
            return;
        }

        if self.doc().metadata_is_current() && block.read(cx).tree_metadata_is_current() {
            return;
        }
        self.doc_mut().rebuild_metadata_and_snapshot(cx);
    }

    pub(crate) fn rendered_quote_text_requires_reparse(block: &Entity<Block>, cx: &App) -> bool {
        let block_ref = block.read(cx);
        if block_ref.quote_depth == 0 && !block_ref.kind().is_quote_container() {
            return false;
        }

        let text = block_ref.display_text();
        if !text.contains('\n') {
            return false;
        }

        text.split('\n').skip(1).any(|line| {
            let trimmed_end = line.trim_end();
            if trimmed_end.is_empty() {
                return false;
            }

            let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
            if leading_spaces >= 4 {
                return true;
            }

            BlockKind::detect_markdown_shortcut(&format!("{trimmed_end} "))
                .is_some_and(|(kind, _)| kind != BlockKind::Paragraph)
                || BlockKind::parse_code_fence_opening(trimmed_end).is_some()
                || BlockKind::parse_thematic_break_line(trimmed_end)
                || BlockKind::parse_atx_heading_line(trimmed_end).is_some()
        })
    }
}
