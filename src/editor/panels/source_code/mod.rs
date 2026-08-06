//! Source code panel — the raw Markdown buffer as a standalone block.

pub(crate) mod render;

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;
use crate::layout::types::AreaId;
use crate::model::block::BlockData;

/// The standalone raw-source block backing ONE source-code panel.
///
/// Every `Editing(SourceCode)` panel owns its own runtime (its own block
/// entity, cursor, and subscription) so multiple source panels edit
/// independently; the document content itself stays shared. `area_id`
/// routes the block's `Changed` events back to the owning editor area.
#[derive(Default)]
pub(crate) struct SourcePanelRuntime {
    pub(crate) block: Option<Entity<Block>>,
    pub(crate) doc_hash: u64,
    pub(crate) area_id: AreaId,
}

impl Editor {
    /// Ensure the Source panel's interactive editor block exists. Only
    /// rebuilds when the document was changed by an external source
    /// (e.g. the Block panel), never when the user is actively editing
    /// the source block itself.
    ///
    /// The block is created as a standalone entity with a minimal
    /// subscription that only syncs Changed events back to the document.
    pub(crate) fn refresh_source_panel_block(
        &mut self,
        area_id: AreaId,
        panel_id: usize,
        cx: &mut Context<Self>,
    ) {
        let doc_text = self.doc().to_markdown(cx);
        let doc_hash = Self::hash_str(&doc_text);

        let runtime = self
            .source_panel_runtimes
            .entry(panel_id)
            .or_insert_with(|| SourcePanelRuntime {
                block: None,
                doc_hash: 0,
                area_id,
            });
        if runtime.block.is_none() || doc_hash != runtime.doc_hash {
            runtime.block = None;
            let block = Self::new_standalone_block(cx, BlockData::paragraph(doc_text));
            block.update(cx, |block, _cx| block.set_source_document_mode());
            let area = area_id;
            let panel = panel_id;
            cx.subscribe(
                &block,
                move |this, block, event, cx| {
                    this.with_current_tab_area(area, |editor| {
                        editor.on_source_panel_changed(panel, block, event, cx);
                    });
                },
            )
            .detach();
            runtime.block = Some(block);
            runtime.doc_hash = doc_hash;
        }
    }

    /// Minimal event handler for a Source panel block. Only syncs text
    /// changes back to the shared document — no structural event
    /// processing. Routed to the owning area by the subscription closure.
    pub(crate) fn on_source_panel_changed(
        &mut self,
        panel_id: usize,
        block: Entity<Block>,
        event: &BlockAction,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, BlockAction::Changed) {
            return;
        }
        let text = block.read(cx).display_text().to_string();
        let doc = self.doc().to_markdown(cx);
        if text == doc {
            return;
        }
        let mut roots = Self::parse_document(cx, &text);
        if roots.is_empty() {
            roots.push(Self::new_block(cx, BlockData::paragraph(String::new())));
        }
        self.doc_mut().replace_blocks(roots, cx);
        self.rebuild_table_runtimes(cx);
        self.rebuild_image_runtimes(cx);
        // Record the fingerprint of the synced document, not of the user's
        // raw bytes: markdown parsing normalizes the text (a trailing
        // newline, for instance, does not survive a parse round-trip), so
        // hashing the raw bytes here would make the next render rebuild the
        // block and drop the user's trailing newline. The block keeps the
        // user's bytes; the document is the parsed form.
        let synced_hash = Self::hash_str(&self.doc().to_markdown(cx));
        if let Some(runtime) = self.source_panel_runtimes.get_mut(&panel_id) {
            runtime.doc_hash = synced_hash;
        }
        self.mark_dirty(cx);
    }
}
