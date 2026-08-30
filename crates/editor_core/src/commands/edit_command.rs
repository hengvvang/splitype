//! Unified document editing and formatting command system.
//!
//! Provides a declarative, single-path execution pipeline for text formatting,
//! block structure transformations, insertions, and clipboard operations across both
//! WYSIWYG and Source Code editing modes without redundant compatibility layers.

use gpui::*;

use crate::engine::controller::Editor;
use editor_wysiwyg::document::block::Block;
use editor_wysiwyg::markdown::block::table::TableData;

/// Inline markup formatting variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineFormatKind {
    Bold,
    Italic,
    Strikethrough,
    Highlight,
    InlineCode,
    InlineMath,
    Comment,
    ClearFormat,
}

impl InlineFormatKind {
    /// Markup template for empty caret insertion: (template, caret_offset_inside_template).
    pub fn empty_template(&self) -> (&'static str, usize) {
        match self {
            Self::Bold => ("****", 2),
            Self::Italic => ("**", 1),
            Self::Strikethrough => ("~~~~", 2),
            Self::Highlight => ("====", 2),
            Self::InlineCode => ("``", 1),
            Self::InlineMath => ("$$", 1),
            Self::Comment => ("<!---->", 4),
            Self::ClearFormat => ("", 0),
        }
    }

    /// Markup wrapper delimiters for active selection: (prefix, suffix).
    pub fn wrap_delimiters(&self) -> (&'static str, &'static str) {
        match self {
            Self::Bold => ("**", "**"),
            Self::Italic => ("*", "*"),
            Self::Strikethrough => ("~~", "~~"),
            Self::Highlight => ("==", "=="),
            Self::InlineCode => ("`", "`"),
            Self::InlineMath => ("$", "$"),
            Self::Comment => ("<!--", "-->"),
            Self::ClearFormat => ("", ""),
        }
    }
}

/// Block-level structure kinds and list/heading transformations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockStructureKind {
    Heading(u8),
    Paragraph,
    BulletList,
    NumberedList,
    TaskList,
    Blockquote,
}

impl BlockStructureKind {
    /// Source-mode Markdown line prefix.
    pub fn source_prefix(&self) -> &'static str {
        match self {
            Self::Heading(1) => "# ",
            Self::Heading(2) => "## ",
            Self::Heading(3) => "### ",
            Self::Heading(4) => "#### ",
            Self::Heading(5) => "##### ",
            Self::Heading(_) => "###### ",
            Self::Paragraph => "",
            Self::BulletList => "- ",
            Self::NumberedList => "1. ",
            Self::TaskList => "- [ ] ",
            Self::Blockquote => "> ",
        }
    }
}

/// Structural block insertions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertBlockKind {
    Footnote,
    Callout,
    ThematicBreak,
    CodeBlock,
    MathBlock,
    Mermaid,
}

/// Top-level unified editing command for document content manipulation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentEditCommand {
    Format(InlineFormatKind),
    Structure(BlockStructureKind),
    Insert(InsertBlockKind),
    InsertTable {
        rows: usize,
        columns: usize,
    },
    Cut,
    Copy,
    Paste,
    PastePlain,
    SelectAll,
}

impl Editor {
    /// Centralized dispatcher for all document edit commands across WYSIWYG and Source modes.
    pub fn execute_edit_command(
        &mut self,
        cmd: DocumentEditCommand,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        match cmd {
            DocumentEditCommand::Format(format_kind) => {
                self.execute_format_command(format_kind, target_entity, cx);
            }
            DocumentEditCommand::Structure(structure_kind) => {
                self.execute_structure_command(structure_kind, target_entity, cx);
            }
            DocumentEditCommand::Insert(insert_kind) => {
                self.execute_insert_command(insert_kind, target_entity, cx);
            }
            DocumentEditCommand::InsertTable { rows, columns } => {
                self.execute_insert_table_command(rows, columns, target_entity, cx);
            }
            DocumentEditCommand::Cut => {
                self.execute_cut(cx);
            }
            DocumentEditCommand::Copy => {
                self.execute_copy(cx);
            }
            DocumentEditCommand::Paste => {
                self.execute_paste(target_entity, false, cx);
            }
            DocumentEditCommand::PastePlain => {
                self.execute_paste(target_entity, true, cx);
            }
            DocumentEditCommand::SelectAll => {
                self.execute_select_all(cx);
            }
        }
    }

    fn execute_format_command(
        &mut self,
        kind: InlineFormatKind,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        if kind == InlineFormatKind::ClearFormat {
            self.apply_clear_format(target_entity, cx);
            return;
        }

        let (empty_tpl, caret_offset) = kind.empty_template();
        let (wrap_prefix, wrap_suffix) = kind.wrap_delimiters();
        self.apply_inline_markup(
            target_entity,
            empty_tpl,
            caret_offset,
            wrap_prefix,
            wrap_suffix,
            cx,
        );
    }

    fn execute_structure_command(
        &mut self,
        kind: BlockStructureKind,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        if kind == BlockStructureKind::Paragraph {
            self.apply_line_prefix(target_entity, "", cx);
        } else {
            self.apply_line_prefix(target_entity, kind.source_prefix(), cx);
        }
    }

    fn execute_insert_command(
        &mut self,
        kind: InsertBlockKind,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        match kind {
            InsertBlockKind::Footnote => {
                self.apply_inline_markup(target_entity, "[^1]", 3, "[^", "]", cx);
            }
            InsertBlockKind::Callout => {
                self.apply_inline_markup(target_entity, "> [!]", 4, "> [!", "]", cx);
            }
            InsertBlockKind::ThematicBreak => {
                self.apply_snippet(target_entity, "---", 3, cx);
            }
            InsertBlockKind::CodeBlock => {
                self.apply_inline_markup(
                    target_entity,
                    "``````",
                    3,
                    "```",
                    "```",
                    cx,
                );
            }
            InsertBlockKind::MathBlock => {
                self.apply_inline_markup(target_entity, "$$$$", 2, "$$", "$$", cx);
            }
            InsertBlockKind::Mermaid => {
                self.apply_inline_markup(
                    target_entity,
                    "```mermaid```",
                    10,
                    "```mermaid",
                    "```",
                    cx,
                );
            }
        }
    }

    fn execute_insert_table_command(
        &mut self,
        rows: usize,
        columns: usize,
        target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        let body_rows = rows.saturating_sub(1).max(1);
        let cols = columns.max(1);
        let table = TableData::new_empty(body_rows, cols);
        let table_md = format!("\n{}\n", table.serialize_markdown());
        let first_cell_offset = if table_md.starts_with("\n| ") { 3 } else { 2 };
        self.apply_snippet(target_entity, &table_md, first_cell_offset, cx);
    }

    fn execute_cut(&mut self, _cx: &mut Context<Self>) {}

    fn execute_copy(&mut self, _cx: &mut Context<Self>) {}

    fn execute_paste(&mut self, target_entity: Option<EntityId>, is_plain: bool, cx: &mut Context<Self>) {
        let Some(clipboard_item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(mut text) = clipboard_item.text() else {
            return;
        };
        if is_plain {
            text = text.replace("\r\n", "\n");
        }
        if let Some(block) = self.active_or_target_block(target_entity, cx) {
            self.focus_wysiwyg_block(block.entity_id());
            block.update(cx, |block, cx| {
                let range = block.selected_range.clone();
                let text_len = text.len();
                block.replace_text_in_display_range(
                    range,
                    &text,
                    Some(text_len..text_len),
                    false,
                    cx,
                );
            });
        }
        self.mark_dirty(cx);
        cx.notify();
    }

    fn execute_select_all(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    pub fn active_or_target_block(
        &mut self,
        target_entity: Option<EntityId>,
        _cx: &mut Context<Self>,
    ) -> Option<Entity<Block>> {
        let doc = self.active_doc()?;
        target_entity
            .and_then(|id| doc.block_entity_by_id(id))
            .or_else(|| doc.first_root().cloned())
    }

    pub fn apply_inline_markup(
        &mut self,
        _target_entity: Option<EntityId>,
        empty_template: &str,
        caret_offset_in_empty: usize,
        wrap_prefix: &str,
        wrap_suffix: &str,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.apply_wrapped_or_template(
                empty_template,
                caret_offset_in_empty,
                wrap_prefix,
                wrap_suffix,
                cx,
            );
            self.mark_dirty(cx);
            cx.notify();
        }
    }

    pub fn apply_line_prefix(
        &mut self,
        _target_entity: Option<EntityId>,
        prefix: &str,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.apply_line_prefix(prefix, cx);
            self.mark_dirty(cx);
            cx.notify();
        }
    }

    pub fn apply_snippet(
        &mut self,
        _target_entity: Option<EntityId>,
        snippet: &str,
        caret_offset: usize,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.apply_snippet(snippet, caret_offset, cx);
            self.mark_dirty(cx);
            cx.notify();
        }
    }

    pub fn apply_clear_format(
        &mut self,
        _target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        if let Some(state) = self.pane_state_mut(pane_id) {
            state.pane.apply_clear_format(cx);
            self.mark_dirty(cx);
            cx.notify();
        }
    }
}

pub fn strip_markdown_line_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix("###### ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("##### ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("#### ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("### ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("## ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("# ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [ ] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [x] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- [X] ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("- ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("* ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("> ") {
        rest
    } else if let Some(idx) = trimmed.find(". ") {
        if !trimmed[..idx].is_empty() && trimmed[..idx].chars().all(|c| c.is_ascii_digit()) {
            &trimmed[idx + 2..]
        } else {
            line
        }
    } else {
        line
    }
}
