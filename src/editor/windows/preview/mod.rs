//! Preview panel — read-only rendered snapshot of the document.

pub(crate) mod render;

use gpui::*;

use crate::editor::tree::block::Block;
use crate::editor::controller::Editor;
use crate::model::block::BlockData;

/// Read-only block tree shown in the preview panel.
#[derive(Default)]
pub(crate) struct PreviewState {
    pub(crate) blocks: Vec<Entity<Block>>,
    pub(crate) source_hash: u64,
}

impl Editor {
    /// Rebuild the preview block tree whenever the document source changes.
    pub(crate) fn refresh_preview_blocks(&mut self, cx: &mut Context<Self>) {
        let source = self.doc().to_markdown(cx);
        let hash = Self::hash_str(&source);
        if hash != self.tab().preview.source_hash || self.tab().preview.blocks.is_empty() {
            let mut roots = Self::parse_document(cx, &source);
            if roots.is_empty() {
                roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
            }
            self.tab_mut().preview.blocks = roots;
            self.tab_mut().preview.source_hash = hash;
        }
    }

    pub(crate) fn hash_str(s: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        s.hash(&mut h);
        h.finish()
    }
}
