//! Unified document editing and formatting command system.
//!
//! Provides a declarative, single-path execution pipeline for text formatting,
//! block structure transformations, insertions, and clipboard operations across
//! all registered pane modes without mode-specific branching in editor.

use gpui::*;

use crate::editor::Editor;

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
    CodeBlock,
    MathBlock,
    HorizontalRule,
    HtmlBlock,
}

impl BlockStructureKind {
    /// Markdown line prefix associated with this block kind.
    pub fn line_prefix(&self) -> Option<&'static str> {
        match self {
            Self::Heading(1) => Some("# "),
            Self::Heading(2) => Some("## "),
            Self::Heading(3) => Some("### "),
            Self::Heading(4) => Some("#### "),
            Self::Heading(5) => Some("##### "),
            Self::Heading(6) => Some("###### "),
            Self::Heading(_) => Some("# "),
            Self::BulletList => Some("- "),
            Self::NumberedList => Some("1. "),
            Self::TaskList => Some("- [ ] "),
            Self::Blockquote => Some("> "),
            Self::Paragraph => Some(""),
            Self::CodeBlock | Self::MathBlock | Self::HorizontalRule | Self::HtmlBlock => None,
        }
    }

    /// Snippet template for structural blocks: (snippet, caret_offset_inside_snippet).
    pub fn block_snippet(&self) -> Option<(&'static str, usize)> {
        match self {
            Self::CodeBlock => Some(("```\n\n```\n", 4)),
            Self::MathBlock => Some(("$$\n\n$$\n", 3)),
            Self::HorizontalRule => Some(("\n---\n\n", 5)),
            Self::HtmlBlock => Some(("<div>\n\n</div>\n", 6)),
            _ => None,
        }
    }
}

/// Insertion operations for links, images, tables, formulas, etc.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InsertionKind {
    Link { label: String, url: String },
    Image { alt: String, url: String },
    Table { rows: usize, cols: usize },
    Footnote { label: String },
    MermaidDiagram,
    CustomSnippet { snippet: String, caret_offset: usize },
}

/// The unified edit command representing any formatting or structure change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditCommand {
    FormatInline(InlineFormatKind),
    TransformBlock(BlockStructureKind),
    Insert(InsertionKind),
    Cut,
    Copy,
    Paste { is_plain: bool },
    SelectAll,
}

impl Editor {
    /// Dispatches a unified edit command to the active pane.
    pub fn execute_edit_command(
        &mut self,
        command: EditCommand,
        target_entity: Option<EntityId>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match command {
            EditCommand::FormatInline(inline) => {
                self.execute_format_inline(target_entity, inline, cx);
            }
            EditCommand::TransformBlock(block) => {
                self.execute_transform_block(target_entity, block, cx);
            }
            EditCommand::Insert(insertion) => {
                self.execute_insert(target_entity, insertion, cx);
            }
            EditCommand::Cut => {
                self.execute_cut(cx);
            }
            EditCommand::Copy => {
                self.execute_copy(cx);
            }
            EditCommand::Paste { is_plain } => {
                self.execute_paste(target_entity, is_plain, cx);
            }
            EditCommand::SelectAll => {
                self.execute_select_all(cx);
            }
        }
        cx.notify();
    }

    fn execute_format_inline(
        &mut self,
        target_entity: Option<EntityId>,
        kind: InlineFormatKind,
        cx: &mut Context<Self>,
    ) {
        if kind == InlineFormatKind::ClearFormat {
            self.apply_clear_format(target_entity, cx);
            return;
        }

        let (empty_tpl, caret_off) = kind.empty_template();
        let (wrap_pre, wrap_suf) = kind.wrap_delimiters();
        self.apply_inline_markup(target_entity, empty_tpl, caret_off, wrap_pre, wrap_suf, cx);
    }

    fn execute_transform_block(
        &mut self,
        target_entity: Option<EntityId>,
        kind: BlockStructureKind,
        cx: &mut Context<Self>,
    ) {
        if let Some((snippet, caret_offset)) = kind.block_snippet() {
            self.apply_snippet(target_entity, snippet, caret_offset, cx);
            return;
        }

        if let Some(prefix) = kind.line_prefix() {
            self.apply_line_prefix(target_entity, prefix, cx);
        }
    }

    fn execute_insert(
        &mut self,
        target_entity: Option<EntityId>,
        insertion: InsertionKind,
        cx: &mut Context<Self>,
    ) {
        match insertion {
            InsertionKind::Link { label, url } => {
                let snippet = if url.is_empty() {
                    format!("[{label}]()")
                } else {
                    format!("[{label}]({url})")
                };
                let caret = if url.is_empty() { label.len() + 2 } else { snippet.len() };
                self.apply_snippet(target_entity, &snippet, caret, cx);
            }
            InsertionKind::Image { alt, url } => {
                let snippet = if url.is_empty() {
                    format!("![{alt}]()")
                } else {
                    format!("![{alt}]({url})")
                };
                let caret = if url.is_empty() { alt.len() + 3 } else { snippet.len() };
                self.apply_snippet(target_entity, &snippet, caret, cx);
            }
            InsertionKind::Table { rows, cols } => {
                self.execute_insert_table(target_entity, rows, cols, cx);
            }
            InsertionKind::Footnote { label } => {
                let snippet = format!("[^{label}]");
                let caret = snippet.len();
                self.apply_snippet(target_entity, &snippet, caret, cx);
            }
            InsertionKind::MermaidDiagram => {
                let snippet = "```mermaid\ngraph TD;\n    A-->B;\n```\n";
                self.apply_snippet(target_entity, snippet, 23, cx);
            }
            InsertionKind::CustomSnippet { snippet, caret_offset } => {
                self.apply_snippet(target_entity, &snippet, caret_offset, cx);
            }
        }
    }

    fn execute_insert_table(
        &mut self,
        target_entity: Option<EntityId>,
        rows: usize,
        cols: usize,
        cx: &mut Context<Self>,
    ) {
        let cols = cols.max(1);
        let header_row = format!("| {} |\n", (0..cols).map(|_| "Header").collect::<Vec<_>>().join(" | "));
        let sep_row = format!("| {} |\n", (0..cols).map(|_| "---").collect::<Vec<_>>().join(" | "));
        let body_row = format!("| {} |\n", (0..cols).map(|_| " ").collect::<Vec<_>>().join(" | "));
        let body = body_row.repeat(rows.max(1));
        let table_md = format!("\n{}{}{}\n", header_row, sep_row, body);
        self.apply_snippet(target_entity, &table_md, 3, cx);
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
        let text_len = text.len();
        self.apply_snippet(target_entity, &text, text_len, cx);
    }

    fn execute_select_all(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    /// Commits pane-produced document text once. The authoritative text is
    /// replaced, the revision bumps, the tab becomes dirty, and the other
    /// panes receive the next snapshot — all in one atomic step.
    pub(crate) fn commit_pane_text(
        &mut self,
        pane_id: core_contracts::PaneId,
        text: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = text {
            self.update_raw_document_text(text, pane_id, cx);
        }
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
        let text = self
            .pane_state_mut(pane_id)
            .map(|state| {
                state.pane.apply_wrapped_or_template(
                    empty_template,
                    caret_offset_in_empty,
                    wrap_prefix,
                    wrap_suffix,
                    cx,
                )
            })
            .flatten();
        self.commit_pane_text(pane_id, text, cx);
        cx.notify();
    }

    pub fn apply_line_prefix(
        &mut self,
        _target_entity: Option<EntityId>,
        prefix: &str,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        let text = self
            .pane_state_mut(pane_id)
            .map(|state| state.pane.apply_line_prefix(prefix, cx))
            .flatten();
        self.commit_pane_text(pane_id, text, cx);
        cx.notify();
    }

    pub fn apply_snippet(
        &mut self,
        _target_entity: Option<EntityId>,
        snippet: &str,
        caret_offset: usize,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        let text = self
            .pane_state_mut(pane_id)
            .map(|state| state.pane.apply_snippet(snippet, caret_offset, cx))
            .flatten();
        self.commit_pane_text(pane_id, text, cx);
        cx.notify();
    }

    pub fn apply_clear_format(
        &mut self,
        _target_entity: Option<EntityId>,
        cx: &mut Context<Self>,
    ) {
        let pane_id = self.active_pane_id();
        let text = self
            .pane_state_mut(pane_id)
            .map(|state| state.pane.apply_clear_format(cx))
            .flatten();
        self.commit_pane_text(pane_id, text, cx);
        cx.notify();
    }
}
