//! AST node identity and data types.
//!
//! `NodeId` is a strongly-typed wrapper around `Uuid` for compile-time
//! safety when passing node references. `NodeData` is the pure data
//! model for a single block-level node in the document tree.

use uuid::Uuid;

use super::block_kind::BlockKind;
use crate::core::extensions::html_doc::HtmlDocument;
use crate::core::extensions::table::TableData;
use crate::core::text::rich_text::RichText;

/// Strongly-typed unique identifier for a document tree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub Uuid);

impl NodeId {
    /// Generate a new random `NodeId`.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistent data for a single block-level node.
///
/// This is the pure data record, independent of any GPUI view state.
/// It holds the node's identity, semantic kind, inline-formatted title,
/// and tree references (parent / children via [`NodeId`]).
#[derive(Debug, Clone)]
pub struct NodeData {
    /// Unique node identifier.
    pub id: NodeId,
    /// Semantic block kind.
    pub kind: BlockKind,
    /// Inline-formatted title text.
    pub title: RichText,
    /// Native table data (only for `BlockKind::Table`).
    pub table_data: Option<TableData>,
    /// Parsed HTML document (only for `BlockKind::HtmlBlock`).
    pub html_document: Option<HtmlDocument>,
    /// Parent node id (`None` for root nodes).
    pub parent_id: Option<NodeId>,
    /// Ordered list of child node ids.
    pub child_ids: Vec<NodeId>,
    /// Raw Markdown source fallback for opaque block kinds.
    pub raw_fallback: Option<String>,
}

impl NodeData {
    /// Create a new node with the given kind and title.
    pub fn new(kind: BlockKind, title: RichText) -> Self {
        let mut data = Self {
            id: NodeId::new(),
            kind,
            title,
            table_data: None,
            html_document: None,
            parent_id: None,
            child_ids: Vec::new(),
            raw_fallback: None,
        };
        data.sync_raw_fallback();
        data
    }

    /// Create a new node with plain text as title.
    pub fn with_plain_text(kind: BlockKind, text: impl Into<String>) -> Self {
        Self::new(kind, RichText::plain(text.into()))
    }

    /// Convenience: create a paragraph node.
    pub fn paragraph(text: impl Into<String>) -> Self {
        Self::with_plain_text(BlockKind::Paragraph, text)
    }

    /// Create a raw Markdown fallback node.
    pub fn raw_markdown(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::RawMarkdown, markdown.clone());
        data.raw_fallback = Some(markdown);
        data
    }

    /// Create an HTML comment node.
    pub fn html_comment(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::HtmlComment, markdown.clone());
        data.raw_fallback = Some(markdown);
        data
    }

    /// Create an HTML block node, parsing the HTML document.
    pub fn html_block(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let html = crate::core::extensions::html_doc::parse_html_document(&markdown);
        let mut data = Self::with_plain_text(BlockKind::HtmlBlock, markdown.clone());
        data.html_document = Some(html);
        data.raw_fallback = Some(markdown);
        data
    }

    /// Create a LaTeX math block node.
    pub fn latex_math(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::LatexMath, markdown.clone());
        data.raw_fallback = Some(markdown);
        data
    }

    /// Create a Mermaid diagram block node.
    pub fn mermaid_diagram(markdown: impl Into<String>) -> Self {
        let markdown = markdown.into();
        let mut data = Self::with_plain_text(BlockKind::MermaidDiagram, markdown.clone());
        data.raw_fallback = Some(markdown);
        data
    }

    /// Create a table node from existing table data.
    pub fn table(table: TableData) -> Self {
        let mut data = Self::new(BlockKind::Table, RichText::plain(String::new()));
        data.table_data = Some(table);
        data
    }

    /// Returns true for block kinds that preserve their original source
    /// in `raw_fallback` because they are opaque to the runtime.
    pub fn uses_raw_fallback(&self) -> bool {
        matches!(
            self.kind,
            BlockKind::ThematicBreak
                | BlockKind::RawMarkdown
                | BlockKind::HtmlComment
                | BlockKind::HtmlBlock
                | BlockKind::LatexMath
                | BlockKind::MermaidDiagram
        )
    }

    pub fn sync_raw_fallback(&mut self) {
        if self.uses_raw_fallback() {
            self.raw_fallback = Some(self.title.visible_text().to_string());
            if self.kind == BlockKind::HtmlBlock {
                self.html_document = self
                    .raw_fallback
                    .as_ref()
                    .map(|raw| crate::core::extensions::html_doc::parse_html_document(raw));
            }
        } else {
            self.raw_fallback = None;
            self.html_document = None;
        }
    }
}
