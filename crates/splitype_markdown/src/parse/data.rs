//! Persistent data of a block, independent of the editor runtime.
//!
//! `BlockData` is the pure data for one block in the document tree: identity,
//! semantic kind, inline-formatted text, optional payloads (table, HTML), and
//! tree references (parent / children via [`BlockId`]). Raw-preserved
//! Markdown keeps its original source in `raw_source` so it round-trips
//! through save/load losslessly.

use super::id::BlockId;
use super::kind::BlockKind;
use crate::block::html::{HtmlDocument, parse_html_document};
use crate::block::image::parse_standalone_image;
use crate::block::math::parse_display_math_source;
use crate::block::mermaid::parse_mermaid_fence_source;
use crate::block::table::TableData;
use crate::inline::text::BlockText;

/// Persistent data of a single block in the document tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockData {
    /// Unique block identifier.
    pub id: BlockId,
    /// Semantic block kind.
    pub kind: BlockKind,
    /// Inline-formatted block content.
    pub text: BlockText,
    /// Native table data (only for `BlockKind::Table`).
    pub table: Option<TableData>,
    /// Parsed HTML document (only for `BlockKind::HtmlBlock`).
    pub html: Option<HtmlDocument>,
    /// Parent block id (`None` for root blocks).
    pub parent: Option<BlockId>,
    /// Ordered list of child block ids.
    pub children: Vec<BlockId>,
    /// Raw Markdown source fallback for opaque block kinds.
    pub raw_source: Option<String>,
}

impl BlockData {
    /// Create a new block with the given kind and text.
    pub fn new(kind: BlockKind, text: BlockText) -> Self {
        let mut data = Self {
            id: BlockId::new(),
            kind,
            text,
            table: None,
            html: None,
            parent: None,
            children: Vec::new(),
            raw_source: None,
        };
        data.sync_raw_source();
        data
    }

    /// Create a new block with plain text.
    pub fn with_plain_text(kind: BlockKind, text: impl Into<String>) -> Self {
        Self::new(kind, BlockText::plain(text.into()))
    }

    /// Convenience: create a paragraph block.
    pub fn paragraph(text: impl Into<String>) -> Self {
        Self::with_plain_text(BlockKind::Paragraph, text)
    }

    /// Create a raw Markdown fallback block.
    pub fn raw_markdown(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::RawMarkdown, markdown.clone());
        data.raw_source = Some(markdown);
        data
    }

    /// Create an HTML comment block.
    pub fn html_comment(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::HtmlComment, markdown.clone());
        data.raw_source = Some(markdown);
        data
    }

    /// Create an HTML block, parsing the HTML document.
    pub fn html_block(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let html = parse_html_document(&markdown);
        let mut data = Self::with_plain_text(BlockKind::HtmlBlock, markdown.clone());
        data.html = Some(html);
        data.raw_source = Some(markdown);
        data
    }

    /// Create a LaTeX math block. The stored text is the formula body between
    /// the display delimiters; the `$$` fences are rebuilt on serialization.
    pub fn latex_block(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let body = parse_display_math_source(&markdown)
            .map(|source| source.body)
            .unwrap_or(markdown);
        Self::with_plain_text(BlockKind::MathBlock, body)
    }

    /// Create a Mermaid diagram block. The stored text is the diagram source
    /// between the fences; the fences are rebuilt on serialization.
    pub fn mermaid_block(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let body = parse_mermaid_fence_source(&markdown)
            .map(|source| source.body)
            .unwrap_or(markdown);
        Self::with_plain_text(BlockKind::MermaidBlock, body)
    }

    /// Create a table block from existing table data.
    pub fn table(table: TableData) -> Self {
        let mut data = Self::new(BlockKind::Table, BlockText::plain(String::new()));
        data.table = Some(table);
        data
    }

    /// Replace the block text.
    pub fn set_text(&mut self, text: BlockText) {
        self.text = text;
        self.sync_raw_source();
    }

    /// Export the block text as Markdown: fragment style flags are
    /// serialized back to delimiter markers via [`BlockText::serialize_markdown`].
    pub fn text_markdown(&self) -> String {
        self.text.serialize_markdown()
    }

    /// Returns true for block kinds that keep their source text in
    /// `raw_source` because they are preserved as opaque Markdown. Math and
    /// Mermaid blocks store their bare body here and rebuild the `$$` / fence
    /// markers on serialization; the other kinds keep the full source verbatim.
    pub fn preserves_raw_source(&self) -> bool {
        matches!(
            self.kind,
            BlockKind::ThematicBreak
                | BlockKind::RawMarkdown
                | BlockKind::HtmlComment
                | BlockKind::HtmlBlock
                | BlockKind::MathBlock
                | BlockKind::MermaidBlock
        )
    }

    /// Serialize this block back to a single Markdown line, including
    /// indentation for nested blocks and list ordinal for numbered items.
    /// Raw-preserved blocks produce their fallback text when at depth 0.
    /// Math and Mermaid blocks rebuild their fences around the stored body.
    pub fn serialize_markdown_line(&self, depth: usize, list_ordinal: Option<usize>) -> String {
        let indentation = "  ".repeat(depth);
        let text_markdown = self.text_markdown_for_output();
        match self.kind {
            BlockKind::Paragraph => indent_multiline(&text_markdown, &indentation),
            BlockKind::ThematicBreak => {
                // Prefer the edited plain text so focused editing of the
                // separator round-trips; the raw source only preserves the
                // original marker style (e.g. `***`) when the text is emptied.
                let plain = self.text.plain_text();
                if !plain.trim().is_empty() {
                    plain.to_string()
                } else if let Some(raw) = &self.raw_source
                    && !raw.trim().is_empty()
                {
                    raw.clone()
                } else {
                    "---".to_string()
                }
            }
            BlockKind::Heading { level } => {
                format!(
                    "{indentation}{} {text_markdown}",
                    "#".repeat(level as usize)
                )
            }
            BlockKind::BulletListItem => prefixed_multiline(
                &text_markdown,
                &format!("{indentation}- "),
                &format!("{indentation}  "),
            ),
            BlockKind::TaskListItem { checked } => prefixed_multiline(
                &text_markdown,
                &format!("{indentation}- [{}] ", if checked { "x" } else { " " }),
                &format!("{indentation}      "),
            ),
            BlockKind::NumberedListItem => {
                let ordinal = list_ordinal.unwrap_or(1);
                prefixed_multiline(
                    &text_markdown,
                    &format!("{indentation}{ordinal}. "),
                    &format!("{indentation}   "),
                )
            }
            BlockKind::Blockquote => prefixed_multiline(
                &crate::block::CalloutKind::escape_plain_quote_header(&text_markdown),
                &format!("{indentation}> "),
                &format!("{indentation}> "),
            ),
            BlockKind::Callout(variant) => {
                format!("{indentation}> {}", variant.header_markdown(&text_markdown))
            }
            BlockKind::FootnoteDefinition => {
                let (id, content) =
                    crate::block::footnote::split_footnote_definition_text(&text_markdown);
                if content.is_empty() {
                    format!("{indentation}[^{id}]:")
                } else {
                    format!("{indentation}[^{id}]: {content}")
                }
            }
            BlockKind::Table => String::new(),
            BlockKind::CodeBlock { .. } => text_markdown,
            BlockKind::MathBlock | BlockKind::MermaidBlock => {
                let body = self.raw_source.clone().unwrap_or(text_markdown);
                let (source, _) = match self.kind {
                    BlockKind::MathBlock => {
                        crate::block::math::serialize_display_math_source(&body)
                    }
                    _ => crate::block::mermaid::serialize_mermaid_source(&body),
                };
                if depth == 0 {
                    source
                } else {
                    indent_multiline(&source, &indentation)
                }
            }
            BlockKind::RawMarkdown | BlockKind::HtmlComment | BlockKind::HtmlBlock => {
                if depth == 0 {
                    self.raw_source.clone().unwrap_or(text_markdown)
                } else {
                    indent_multiline(
                        &self.raw_source.clone().unwrap_or(text_markdown),
                        &indentation,
                    )
                }
            }
        }
    }

    fn text_markdown_for_output(&self) -> String {
        let plain = self.text.plain_text();
        if self.can_present_text_as_standalone_image() && parse_standalone_image(&plain).is_some() {
            return plain;
        }

        self.text_markdown()
    }

    fn can_present_text_as_standalone_image(&self) -> bool {
        matches!(
            self.kind,
            BlockKind::Paragraph
                | BlockKind::BulletListItem
                | BlockKind::NumberedListItem
                | BlockKind::TaskListItem { .. }
        )
    }

    fn sync_raw_source(&mut self) {
        if self.preserves_raw_source() {
            self.raw_source = Some(self.text.plain_text().to_string());
            if self.kind == BlockKind::HtmlBlock {
                self.html = self.raw_source.as_ref().map(|raw| parse_html_document(raw));
            }
        } else {
            self.raw_source = None;
            self.html = None;
        }
    }
}

fn indent_multiline(content: &str, indentation: &str) -> String {
    content
        .split('\n')
        .map(|line| format!("{indentation}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn prefixed_multiline(content: &str, first_prefix: &str, continuation_prefix: &str) -> String {
    let mut lines = content.split('\n');
    let mut rendered = String::new();
    if let Some(first) = lines.next() {
        rendered.push_str(first_prefix);
        rendered.push_str(first);
    }

    for line in lines {
        rendered.push('\n');
        rendered.push_str(continuation_prefix);
        rendered.push_str(line);
    }

    rendered
}



