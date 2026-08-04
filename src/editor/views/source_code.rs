//! Source code panel — the raw Markdown buffer as a standalone block.

use gpui::*;

use crate::editor::actions::BlockAction;
use crate::editor::block::Block;
use crate::editor::controller::Editor;
use crate::model::block::BlockData;

/// The standalone raw-source block backing the source-code panel.
#[derive(Default)]
pub(crate) struct SourcePanelState {
    pub(crate) block: Option<Entity<Block>>,
    pub(crate) doc_hash: u64,
}

impl Editor {
    /// Ensure the Source panel's interactive editor block exists.  Only
    /// rebuilds when the document was changed by an external source
    /// (e.g. the Block panel), never when the user is actively editing
    /// the source block itself.
    ///
    /// The block is created as a standalone entity with a minimal
    /// subscription that only syncs Changed events back to the document.
    pub(crate) fn refresh_source_panel_block(&mut self, cx: &mut Context<Self>) {
        let doc_text = self.document.to_markdown(cx);
        let doc_hash = Self::hash_str(&doc_text);

        if self.source_panel.block.is_none() || doc_hash != self.source_panel.doc_hash {
            self.source_panel.block = None;
            let block = Self::new_standalone_block(cx, BlockData::paragraph(doc_text));
            block.update(cx, |block, _cx| block.set_source_document_mode());
            cx.subscribe(&block, Self::on_source_panel_changed).detach();
            self.source_panel.block = Some(block);
            self.source_panel.doc_hash = doc_hash;
        }
    }

    /// Minimal event handler for the Source panel block.  Only syncs text
    /// changes back to the shared document — no structural event processing.
    pub(crate) fn on_source_panel_changed(
        &mut self,
        block: Entity<Block>,
        event: &BlockAction,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, BlockAction::Changed) {
            return;
        }
        let text = block.read(cx).display_text().to_string();
        let doc = self.document.to_markdown(cx);
        if text == doc {
            return;
        }
        let mut roots = Self::parse_document(cx, &text);
        if roots.is_empty() {
            roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
        }
        self.document.replace_blocks(roots, cx);
        self.rebuild_table_runtimes(cx);
        self.rebuild_image_runtimes(cx);
        self.source_panel.doc_hash = Self::hash_str(&text);
        self.mark_dirty(cx);
    }

    pub(crate) fn source_panel_hash(&self) -> u64 {
        self.source_panel.doc_hash
    }
}
