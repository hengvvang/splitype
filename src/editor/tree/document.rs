//! Runtime ownership for the editor block tree.
//!
//! [`Document`] is the only mutable owner of block ordering and parent-child
//! relationships inside the editor. It also maintains a cached
//! [`BlockIndex`] so hot-path lookups do not re-run a full DFS on every
//! focus, scroll, or mutation event.

use std::collections::HashMap;

use gpui::*;

use crate::editor::controller::Editor;
use crate::editor::tree::block::Block;
use crate::model::block::CalloutKind;
use crate::model::block::image::parse_standalone_image;
use crate::model::block::table::serialize_table_markdown_lines;
use crate::model::parse::{BlockId, BlockKind};

/// A block together with its position in the flattened document (DFS) order.
#[derive(Clone)]
pub(crate) struct BlockEntry {
    pub entity: Entity<Block>,
}

/// A block's position inside the runtime tree.
#[derive(Clone)]
pub(crate) struct BlockLocation {
    pub parent: Option<Entity<Block>>,
    pub index: usize,
}

/// Cached flattened-order metadata for the current runtime tree.
#[derive(Default, Clone)]
pub(crate) struct BlockIndex {
    entries: Vec<BlockEntry>,
    index_by_entity: HashMap<EntityId, usize>,
    location_by_entity: HashMap<EntityId, BlockLocation>,
    last_descendant_by_entity: HashMap<EntityId, EntityId>,
}

impl BlockIndex {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.index_by_entity.clear();
        self.location_by_entity.clear();
        self.last_descendant_by_entity.clear();
    }
}

/// Canonical owner of the runtime block tree.
///
/// The Markdown importer builds root blocks and nested list children, then
/// hands the structure to `Document`. From that point on, every structural
/// edit must go through this type so the runtime tree stays aligned with the
/// subset of Markdown that the importer and serializer can reconstruct.
pub(crate) struct Document {
    roots: Vec<Entity<Block>>,
    snapshot: BlockIndex,
    /// Incremented on every structural change (insert/remove/replace).
    /// Reference-context syncs use it to detect newly-created blocks that
    /// have never received a context, so unchanged registries can still
    /// skip the per-block loop when the block set is identical to the last
    /// sync.
    structure_version: u64,
    /// The structure version the tree metadata (quote depths, anchors,
    /// ordinals, entries snapshot) was last rebuilt for. Text-only edits do
    /// not change tree metadata, so callers can skip the full DFS while
    /// this matches [`Self::structure_version`].
    metadata_rebuild_version: u64,
}

impl Document {
    pub(crate) fn new(roots: Vec<Entity<Block>>) -> Self {
        Self {
            roots,
            snapshot: BlockIndex::default(),
            structure_version: 0,
            metadata_rebuild_version: 0,
        }
    }

    /// Version of the current block set; grows on every structural edit.
    pub(crate) fn structure_version(&self) -> u64 {
        self.structure_version
    }

    /// Whether the cached tree metadata was rebuilt for the current
    /// structure. Text-only edits leave it true.
    pub(crate) fn metadata_is_current(&self) -> bool {
        self.metadata_rebuild_version == self.structure_version
    }

    /// Records that runtime-only blocks (e.g. table cells) were recreated,
    /// so the next reference-context sync refreshes them even when the
    /// document registries are unchanged.
    pub(crate) fn mark_structure_changed(&mut self) {
        self.structure_version += 1;
    }

    pub(crate) fn first_root(&self) -> Option<&Entity<Block>> {
        self.roots.first()
    }

    pub(crate) fn root_blocks(&self) -> &[Entity<Block>] {
        &self.roots
    }

    pub(crate) fn root_count(&self) -> usize {
        self.roots.len()
    }

    pub(crate) fn blocks(&self) -> &[BlockEntry] {
        &self.snapshot.entries
    }

    pub(crate) fn flatten_entries(&self) -> Vec<BlockEntry> {
        self.snapshot.entries.clone()
    }

    pub(crate) fn focused_block_entity_id(&self, window: &Window, cx: &App) -> Option<EntityId> {
        self.snapshot
            .entries
            .iter()
            .find(|entries| entries.entity.read(cx).focus_handle.is_focused(window))
            .map(|entries| entries.entity.entity_id())
    }

    pub(crate) fn index_for_entity_id(&self, entity_id: EntityId) -> Option<usize> {
        self.snapshot.index_by_entity.get(&entity_id).copied()
    }

    pub(crate) fn block_entity_by_id(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        self.index_for_entity_id(entity_id)
            .and_then(|index| self.snapshot.entries.get(index))
            .map(|entries| entries.entity.clone())
    }

    pub(crate) fn find_block_location(&self, entity_id: EntityId) -> Option<BlockLocation> {
        self.snapshot.location_by_entity.get(&entity_id).cloned()
    }

    /// Returns the sibling immediately before `entity_id` within the same
    /// parent, if any.
    pub(crate) fn previous_sibling(&self, entity_id: EntityId, cx: &App) -> Option<Entity<Block>> {
        let location = self.find_block_location(entity_id)?;
        let prev_index = location.index.checked_sub(1)?;
        match &location.parent {
            Some(parent) => parent.read(cx).children.get(prev_index).cloned(),
            None => self.roots.get(prev_index).cloned(),
        }
    }

    pub(crate) fn last_descendant(&self, entity_id: EntityId) -> Option<Entity<Block>> {
        let descendant_id = self
            .snapshot
            .last_descendant_by_entity
            .get(&entity_id)
            .copied()?;
        self.block_entity_by_id(descendant_id)
    }

    pub(crate) fn replace_blocks(&mut self, roots: Vec<Entity<Block>>, cx: &mut Context<Editor>) {
        self.roots = roots;
        self.structure_version += 1;
        self.rebuild_metadata_and_snapshot(cx);
    }

    pub(crate) fn serialize_markdown(&self, cx: &App) -> String {
        let mut lines = Vec::new();
        Self::collect_root_markdown_lines(&self.roots, cx, &mut lines);
        lines.join("\n")
    }

    pub(crate) fn serialize_source_text(&self, cx: &App) -> String {
        self.snapshot
            .entries
            .iter()
            .map(|entries| entries.entity.read(cx).display_text().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn insert_blocks_at(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut Context<Editor>,
    ) {
        self.with_structure_mutation(cx, move |tree, cx| {
            tree.insert_blocks_at_raw(parent, index, blocks, cx);
        });
    }

    /// Runs a tree mutation and then eagerly rebuilds metadata and the entries
    /// snapshot exactly once for that mutation batch.
    pub(crate) fn with_structure_mutation<R>(
        &mut self,
        cx: &mut Context<Editor>,
        mutate: impl FnOnce(&mut Self, &mut Context<Editor>) -> R,
    ) -> R {
        let result = mutate(self, cx);
        self.structure_version += 1;
        self.rebuild_metadata_and_snapshot(cx);
        result
    }

    /// Rebuilds tree metadata and cached flattened-order data from the current
    /// roots.
    ///
    /// The pass first normalizes impossible runtime-only shapes by hoisting
    /// children out of leaf blocks. It then performs one DFS to update parent
    /// UUIDs, child UUID lists, render depth, numbered-list ordinals, and the
    /// entries snapshot.
    pub(crate) fn rebuild_metadata_and_snapshot(&mut self, cx: &mut Context<Editor>) {
        Self::normalize_block_list(&mut self.roots, cx);
        self.snapshot.clear();
        Self::sync_block_list(
            &self.roots.clone(),
            None,
            None,
            0,
            0,
            None,
            None,
            0,
            None,
            None,
            None,
            cx,
            &mut self.snapshot,
        );
        self.metadata_rebuild_version = self.structure_version;
    }

    pub(crate) fn take_children(
        block: &Entity<Block>,
        cx: &mut Context<Editor>,
    ) -> Vec<Entity<Block>> {
        let mut children = Vec::new();
        block.update(cx, |block, _cx| {
            children = std::mem::take(&mut block.children);
        });
        children
    }

    pub(crate) fn insert_blocks_at_raw(
        &mut self,
        parent: Option<Entity<Block>>,
        index: usize,
        blocks: Vec<Entity<Block>>,
        cx: &mut Context<Editor>,
    ) {
        if blocks.is_empty() {
            return;
        }

        if let Some(parent) = parent {
            parent.update(cx, move |parent, _cx| {
                for (offset, block) in blocks.iter().cloned().enumerate() {
                    parent.children.insert(index + offset, block);
                }
            });
        } else {
            for (offset, block) in blocks.into_iter().enumerate() {
                self.roots.insert(index + offset, block);
            }
        }
    }

    pub(crate) fn remove_block_by_id_raw(
        &mut self,
        entity_id: EntityId,
        cx: &mut Context<Editor>,
    ) -> Option<(Entity<Block>, BlockLocation)> {
        let location = self.find_block_location(entity_id)?;
        let removed = if let Some(parent) = location.parent.clone() {
            let mut removed = None;
            parent.update(cx, |parent, _cx| {
                removed = Some(parent.children.remove(location.index));
            });
            removed?
        } else {
            self.roots.remove(location.index)
        };

        Some((removed, location))
    }

    /// Normalizes a sibling list so only container-capable block kinds retain
    /// children.
    ///
    /// Children attached to leaf blocks are hoisted into the same parent list
    /// immediately after the leaf that previously owned them.
    pub(crate) fn normalize_block_list(blocks: &mut Vec<Entity<Block>>, cx: &mut Context<Editor>) {
        let mut index = 0;
        while index < blocks.len() {
            let block = blocks[index].clone();
            let mut children = Self::take_children(&block, cx);
            Self::normalize_block_list(&mut children, cx);

            if block.read(cx).kind().supports_children() {
                block.update(cx, {
                    let children = children.clone();
                    move |block, _cx| {
                        block.children = children.clone();
                    }
                });
            } else if !children.is_empty() {
                blocks.splice(index + 1..index + 1, children);
            }

            index += 1;
        }
    }

    pub(crate) fn sync_block_list(
        blocks: &[Entity<Block>],
        parent_entity: Option<Entity<Block>>,
        parent_id: Option<BlockId>,
        list_depth: usize,
        inherited_quote_depth: usize,
        inherited_quote_group_id: Option<BlockId>,
        inherited_visible_quote_group_id: Option<BlockId>,
        inherited_callout_depth: usize,
        inherited_callout_group_id: Option<BlockId>,
        inherited_callout_variant: Option<CalloutKind>,
        inherited_footnote_group_id: Option<BlockId>,
        cx: &mut Context<Editor>,
        snapshot: &mut BlockIndex,
    ) {
        let mut numbered_list_ordinal = 0;
        let mut previous_was_list_item = false;
        for (index, block) in blocks.iter().enumerate() {
            let entity_id = block.entity_id();
            let entry_index = snapshot.entries.len();
            snapshot.entries.push(BlockEntry {
                entity: block.clone(),
            });
            snapshot.index_by_entity.insert(entity_id, entry_index);
            snapshot.location_by_entity.insert(
                entity_id,
                BlockLocation {
                    parent: parent_entity.clone(),
                    index,
                },
            );

            let (block_id, kind, children, is_empty_paragraph) = {
                let block_ref = block.read(cx);
                (
                    block_ref.data.id,
                    block_ref.kind(),
                    block_ref.children.clone(),
                    block_ref.kind() == BlockKind::Paragraph
                        && block_ref.data.text.plain_text().is_empty()
                        && block_ref.children.is_empty(),
                )
            };
            let parent_is_list_item = parent_entity
                .as_ref()
                .is_some_and(|parent| parent.read(cx).kind().is_list_item());

            let content = children
                .iter()
                .map(|child| child.read(cx).data.id)
                .collect::<Vec<_>>();
            let list_ordinal = if kind.is_numbered_list_item() {
                numbered_list_ordinal += 1;
                Some(numbered_list_ordinal)
            } else {
                numbered_list_ordinal = 0;
                None
            };
            let is_quote_container = kind.is_quote_container();
            let own_callout_variant = kind.callout_kind();
            let quote_depth = inherited_quote_depth + usize::from(is_quote_container);
            let quote_group_id = if is_quote_container {
                inherited_quote_group_id.or(Some(block_id))
            } else {
                inherited_quote_group_id
            };
            let callout_depth =
                inherited_callout_depth + usize::from(own_callout_variant.is_some());
            let callout_group_id = if own_callout_variant.is_some() {
                Some(block_id)
            } else {
                inherited_callout_group_id
            };
            let callout_variant = own_callout_variant.or(inherited_callout_variant);
            let visible_quote_depth = quote_depth.saturating_sub(callout_depth);
            let visible_quote_group_id = match kind {
                BlockKind::Blockquote => inherited_visible_quote_group_id.or(Some(block_id)),
                BlockKind::Callout(_) => None,
                _ if visible_quote_depth == 0 => None,
                _ => inherited_visible_quote_group_id,
            };
            let child_visible_quote_group_id = if own_callout_variant.is_some() {
                None
            } else {
                visible_quote_group_id
            };
            let footnote_group_id = if kind.is_footnote_definition() {
                Some(block_id)
            } else {
                inherited_footnote_group_id
            };
            let child_list_depth = list_depth + usize::from(kind.is_list_item());
            let list_group_separator_candidate = is_empty_paragraph && previous_was_list_item;

            block.update(cx, move |block, _cx| {
                block.data.parent = parent_id;
                block.data.children = content.clone();
                block.render_depth = list_depth;
                block.quote_depth = quote_depth;
                block.quote_group_id = quote_group_id;
                block.visible_quote_depth = visible_quote_depth;
                block.visible_quote_group_id = visible_quote_group_id;
                block.callout_depth = callout_depth;
                block.callout_group_id = callout_group_id;
                block.callout_variant = callout_variant;
                block.footnote_group_id = footnote_group_id;
                block.parent_is_list_item = parent_is_list_item;
                block.list_ordinal = list_ordinal;
                block.list_group_separator_candidate = list_group_separator_candidate;
                block.tree_metadata_flags = block.kind_metadata_flags();
            });

            let last_descendant_id = if children.is_empty() {
                entity_id
            } else {
                Self::sync_block_list(
                    &children,
                    Some(block.clone()),
                    Some(block_id),
                    child_list_depth,
                    quote_depth,
                    quote_group_id,
                    child_visible_quote_group_id,
                    callout_depth,
                    callout_group_id,
                    callout_variant,
                    footnote_group_id,
                    cx,
                    snapshot,
                );
                snapshot
                    .last_descendant_by_entity
                    .get(&children.last().expect("children checked").entity_id())
                    .copied()
                    .unwrap_or_else(|| children.last().expect("children checked").entity_id())
            };

            snapshot
                .last_descendant_by_entity
                .insert(entity_id, last_descendant_id);
            previous_was_list_item = kind.is_list_item();
        }
    }

    pub(crate) fn is_empty_root_paragraph(block: &Block) -> bool {
        block.kind() == BlockKind::Paragraph
            && block.data.text.plain_text().is_empty()
            && block.children.is_empty()
    }

    pub(crate) fn collect_root_markdown_lines(
        blocks: &[Entity<Block>],
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        let mut pending_empty_roots = 0usize;
        let mut wrote_non_empty_root = false;
        let mut previous_was_list_item = false;

        for block in blocks {
            let block_ref = block.read(cx);
            if Self::is_empty_root_paragraph(block_ref) {
                pending_empty_roots += 1;
                continue;
            }

            let current_is_list_item = block_ref.kind().is_list_item();
            let current_is_footnote = block_ref.kind() == BlockKind::FootnoteDefinition;
            if wrote_non_empty_root {
                let separator_count = if previous_was_list_item && current_is_list_item {
                    pending_empty_roots
                } else if current_is_footnote && pending_empty_roots == 0 {
                    // Adjacent footnote definitions (or a definition directly
                    // after a paragraph) stay tight: no blank line is forced
                    // between them, so `[^a]: x\n[^b]: y` round-trips.
                    0
                } else {
                    pending_empty_roots + 1
                };
                lines.extend(std::iter::repeat_n(String::new(), separator_count));
            } else if pending_empty_roots > 0 {
                lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots));
            }

            Self::collect_single_block_markdown_lines(block_ref, 0, cx, lines);
            wrote_non_empty_root = true;
            pending_empty_roots = 0;
            previous_was_list_item = current_is_list_item;
        }

        if wrote_non_empty_root {
            if pending_empty_roots > 0 {
                lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots + 1));
            }
        } else if pending_empty_roots > 1 {
            lines.extend(std::iter::repeat_n(String::new(), pending_empty_roots));
        }
    }

    pub(crate) fn collect_single_block_markdown_lines(
        block_ref: &Block,
        list_depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
    ) {
        match block_ref.kind() {
            BlockKind::Table => {
                if let Some(table) = block_ref.data.table.as_ref() {
                    lines.extend(serialize_table_markdown_lines(table));
                }
            }
            BlockKind::CodeBlock { language } => {
                let indentation = "  ".repeat(list_depth);
                let lang_str = language.as_ref().map(|s| s.as_ref()).unwrap_or("");
                let fence = crate::editor::tree::serialize::safe_code_fence_with_info(
                    &block_ref.data.text.plain_text(),
                    language.as_ref().map(|language| language.as_ref()),
                );
                lines.push(format!("{indentation}{fence}{lang_str}"));
                let content = block_ref.data.text.plain_text();
                for code_line in content.split('\n') {
                    lines.push(format!("{indentation}{code_line}"));
                }
                lines.push(format!("{indentation}{fence}"));
            }
            BlockKind::Blockquote => {
                let text_markdown =
                    CalloutKind::escape_plain_quote_header(&block_ref.data.text_markdown());
                let indentation = "  ".repeat(list_depth);
                if !text_markdown.is_empty() || block_ref.children.is_empty() {
                    for line in text_markdown.split('\n') {
                        lines.push(format!("{indentation}> {line}"));
                    }
                }

                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::Callout(variant) => {
                let indentation = "  ".repeat(list_depth);
                lines.push(format!(
                    "{indentation}> {}",
                    variant.header_markdown(&block_ref.data.text_markdown())
                ));
                if !block_ref.children.is_empty() {
                    let mut child_lines = Vec::new();
                    Self::collect_markdown_lines(
                        &block_ref.children,
                        list_depth,
                        cx,
                        &mut child_lines,
                        false,
                    );
                    lines.extend(
                        child_lines
                            .into_iter()
                            .map(|line| format!("{indentation}> {line}")),
                    );
                }
            }
            BlockKind::FootnoteDefinition => {
                let indentation = "  ".repeat(list_depth);
                let full_text = block_ref.data.text_markdown();
                let (id, first_line) =
                    crate::model::block::footnote::split_footnote_definition_text(&full_text);
                if first_line.is_empty() && block_ref.children.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                    return;
                }

                let mut first_lines = first_line.split('\n');
                let first = first_lines.next().unwrap_or_default();
                if first.is_empty() {
                    lines.push(format!("{indentation}[^{id}]:"));
                } else {
                    lines.push(format!("{indentation}[^{id}]: {first}"));
                }
                for line in first_lines {
                    if line.is_empty() {
                        lines.push(String::new());
                    } else {
                        lines.push(format!("{indentation}    {line}"));
                    }
                }

                if !block_ref.children.is_empty() {
                    lines.push(String::new());
                    Self::collect_markdown_lines(&block_ref.children, 2, cx, lines, true);
                }
            }
            BlockKind::RawMarkdown | BlockKind::HtmlComment | BlockKind::HtmlBlock => {
                let indentation = "  ".repeat(list_depth);
                let raw_markdown = block_ref
                    .data
                    .raw_source
                    .clone()
                    .unwrap_or_else(|| block_ref.data.text_markdown());
                for line in raw_markdown.split('\n') {
                    if indentation.is_empty() {
                        lines.push(line.to_string());
                    } else {
                        lines.push(format!("{indentation}{line}"));
                    }
                }
            }
            BlockKind::BulletListItem
            | BlockKind::TaskListItem { .. }
            | BlockKind::NumberedListItem => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
                let child_list_depth = list_depth + 1;
                for child in &block_ref.children {
                    let child_ref = child.read(cx);
                    if Self::list_child_requires_leading_blank_line(child_ref) {
                        lines.push(String::new());
                    }
                    Self::collect_single_block_markdown_lines(
                        child_ref,
                        child_list_depth,
                        cx,
                        lines,
                    );
                }
            }
            _ => {
                lines.push(
                    block_ref
                        .data
                        .serialize_markdown_line(list_depth, block_ref.list_ordinal),
                );
                let child_list_depth = list_depth + usize::from(block_ref.kind().is_list_item());
                Self::collect_markdown_lines(
                    &block_ref.children,
                    child_list_depth,
                    cx,
                    lines,
                    false,
                );
            }
        }
    }

    pub(crate) fn list_child_requires_leading_blank_line(block_ref: &Block) -> bool {
        if block_ref.kind() != BlockKind::Paragraph || !block_ref.children.is_empty() {
            return false;
        }

        let markdown = block_ref.data.text_markdown();
        !markdown.is_empty() && parse_standalone_image(&markdown).is_none()
    }

    pub(crate) fn collect_markdown_lines(
        blocks: &[Entity<Block>],
        depth: usize,
        cx: &App,
        lines: &mut Vec<String>,
        blank_line_between_siblings: bool,
    ) {
        let mut first = true;
        let mut previous_was_list_item = false;
        for block in blocks {
            let current_is_list_item = block.read(cx).kind().is_list_item();
            if !first
                && blank_line_between_siblings
                && !(previous_was_list_item && current_is_list_item)
            {
                lines.push(String::new());
            }
            first = false;

            let block_ref = block.read(cx);
            Self::collect_single_block_markdown_lines(block_ref, depth, cx, lines);
            previous_was_list_item = current_is_list_item;
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use crate::editor::controller::Editor;
    use crate::model::parse::{BlockData, BlockKind};

    #[gpui::test]
    async fn snapshot_tracks_nested_visible_order(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "- a\n  - b\n    - c\n- d".to_string(), None));

        editor.update(cx, |editor, _cx| {
            let entries = editor.doc().blocks().to_vec();
            let a = entries[0].entity.clone();
            let b = entries[1].entity.clone();
            let c = entries[2].entity.clone();
            let d = entries[3].entity.clone();

            assert_eq!(editor.doc().index_for_entity_id(a.entity_id()), Some(0));
            assert_eq!(editor.doc().index_for_entity_id(b.entity_id()), Some(1));
            assert_eq!(editor.doc().index_for_entity_id(c.entity_id()), Some(2));
            assert_eq!(editor.doc().index_for_entity_id(d.entity_id()), Some(3));

            let c_location = editor
                .doc()
                .find_block_location(c.entity_id())
                .expect("location");
            assert_eq!(
                c_location.parent.expect("nested parent").entity_id(),
                b.entity_id()
            );
            assert_eq!(c_location.index, 0);

            assert_eq!(
                editor
                    .doc()
                    .last_descendant(a.entity_id())
                    .expect("descendant")
                    .entity_id(),
                c.entity_id()
            );
        });
    }

    #[gpui::test]
    async fn rebuild_hoists_children_from_leaf_blocks(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, String::new(), None));

        editor.update(cx, |editor, cx| {
            let root = editor.doc().first_root().expect("root").clone();
            let child = Editor::new_block(cx, BlockData::paragraph("child"));

            root.update(cx, {
                let child = child.clone();
                move |root, _cx| {
                    root.children.push(child.clone());
                }
            });

            editor.doc_mut().rebuild_metadata_and_snapshot(cx);

            assert!(root.read(cx).children.is_empty());
            let visible_ids = editor
                .doc()
                .blocks()
                .iter()
                .map(|entries| entries.entity.entity_id())
                .collect::<Vec<_>>();
            assert_eq!(visible_ids, vec![root.entity_id(), child.entity_id()]);

            let location = editor
                .doc()
                .find_block_location(child.entity_id())
                .expect("child location");
            assert!(location.parent.is_none());
            assert_eq!(location.index, 1);
        });
    }

    #[gpui::test]
    async fn code_block_language_edit_serializes_to_opening_fence(cx: &mut TestAppContext) {
        let editor =
            cx.new(|cx| Editor::from_markdown(cx, "```rust\nfn main() {}\n```".into(), None));

        editor.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block").clone();
            block.update(cx, |block, cx| {
                let range = 0..block.code_language_text().len();
                block.replace_code_language_text_in_range(range, "unknown-lang", None, false, cx);
            });

            assert_eq!(
                editor.doc().serialize_markdown(cx),
                "```unknown-lang\nfn main() {}\n```"
            );
        });
    }

    #[gpui::test]
    async fn code_block_language_with_backtick_round_trips_with_tilde_fence(
        cx: &mut TestAppContext,
    ) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "```rust\nbody\n```".into(), None));

        let markdown = editor.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block").clone();
            block.update(cx, |block, cx| {
                let range = 0..block.code_language_text().len();
                block.replace_code_language_text_in_range(range, "we`rd", None, false, cx);
            });
            editor.doc().serialize_markdown(cx)
        });

        assert_eq!(markdown, "~~~we`rd\nbody\n~~~");

        let round_tripped = cx.new(|cx| Editor::from_markdown(cx, markdown, None));
        round_tripped.update(cx, |editor, cx| {
            let block = editor.doc().first_root().expect("code block");
            assert_eq!(block.read(cx).code_language_text(), "we`rd");
            assert!(matches!(block.read(cx).kind(), BlockKind::CodeBlock { .. }));
        });
    }

    #[gpui::test]
    async fn structure_mutation_rebuilds_snapshot_after_relocation(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| Editor::from_markdown(cx, "- a\n- b\n- c".to_string(), None));

        editor.update(cx, |editor, cx| {
            let entries = editor.doc().blocks().to_vec();
            let a = entries[0].entity.clone();
            let b = entries[1].entity.clone();
            let c = entries[2].entity.clone();

            editor
                .doc_mut()
                .with_structure_mutation(cx, |document, cx| {
                    let moved = document
                        .remove_block_by_id_raw(c.entity_id(), cx)
                        .expect("remove c")
                        .0;
                    document.insert_blocks_at_raw(
                        Some(a.clone()),
                        a.read(cx).children.len(),
                        vec![moved],
                        cx,
                    );
                });

            assert_eq!(editor.doc().index_for_entity_id(a.entity_id()), Some(0));
            assert_eq!(editor.doc().index_for_entity_id(c.entity_id()), Some(1));
            assert_eq!(editor.doc().index_for_entity_id(b.entity_id()), Some(2));

            let c_location = editor
                .doc()
                .find_block_location(c.entity_id())
                .expect("c location");
            assert_eq!(
                c_location.parent.expect("nested parent").entity_id(),
                a.entity_id()
            );
            assert_eq!(c_location.index, 0);

            assert_eq!(
                editor
                    .doc()
                    .last_descendant(a.entity_id())
                    .expect("descendant")
                    .entity_id(),
                c.entity_id()
            );
        });
    }
}
