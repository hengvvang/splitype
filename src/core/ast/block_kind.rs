//! Block kind enumeration — the semantic type of every node in the document tree.
//!
//! Each variant maps to a CommonMark / GFM block-level syntax element and
//! determines both how the block is parsed from Markdown and how it is rendered.

use gpui::SharedString;

use super::callout_kind::CalloutKind;
use super::code_fence::CodeFenceOpening;

/// The semantic kind of a block-level node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// Plain paragraph with inline formatting.
    Paragraph,
    /// Thematic break (CommonMark §4.1): a line of `---`, `***`, or `___`.
    ThematicBreak,
    /// ATX or Setext heading with a CommonMark heading level (1–6).
    Heading { level: u8 },
    /// Unordered list item (CommonMark §5.2).
    UnorderedListItem,
    /// Task-list item with checked state (GFM extension).
    TaskListItem { checked: bool },
    /// Ordered list item (CommonMark §5.3); serialization uses canonical dot markers.
    OrderedListItem,
    /// Blockquote container (CommonMark §5.1).
    Blockquote,
    /// GitHub-style alert / callout container.
    Callout(CalloutKind),
    /// Footnote definition container.
    FootnoteDefinition,
    /// GFM pipe-table block.
    Table,
    /// Fenced or indented code block with optional language info string.
    CodeBlock { language: Option<SharedString> },
    /// HTML comment block preserved as visible comment text.
    HtmlComment,
    /// Raw HTML block rendered through native GPUI semantic elements.
    HtmlBlock,
    /// Display LaTeX math block.
    LatexMath,
    /// Mermaid fenced block rendered as SVG.
    MermaidDiagram,
    /// Raw Markdown fallback for syntax outside the native runtime subset.
    RawMarkdown,
}

impl BlockKind {
    pub fn is_list_item(&self) -> bool {
        matches!(
            self,
            Self::UnorderedListItem | Self::TaskListItem { .. } | Self::OrderedListItem
        )
    }

    /// Parse an ATX heading line `### Title` into level and content.
    pub fn parse_atx_heading_line(line: &str) -> Option<(u8, String)> {
        let trimmed_end = line.trim_end();
        let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
        if leading_spaces > 3 {
            return None;
        }
        let rest = &trimmed_end[leading_spaces..];
        let level = rest.bytes().take_while(|b| *b == b'#').count();
        if !(1..=6).contains(&level) {
            return None;
        }
        let content = rest[level..].strip_prefix(' ')?;
        let mut content = content.trim_end().to_string();
        if let Some(closing_hash_start) = content.rfind(' ')
            && content[closing_hash_start + 1..]
                .chars()
                .all(|ch| ch == '#')
        {
            content.truncate(closing_hash_start);
            content = content.trim_end().to_string();
        }
        Some((level as u8, content))
    }

    /// Parse a Setext underline (`===` or `---`) into the heading level.
    pub fn parse_setext_underline(line: &str) -> Option<u8> {
        let trimmed_end = line.trim_end();
        let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
        if leading_spaces > 3 {
            return None;
        }
        let rest = &trimmed_end[leading_spaces..];
        if rest.len() < 3 {
            return None;
        }
        if rest.bytes().all(|b| b == b'=') {
            Some(1)
        } else if rest.bytes().all(|b| b == b'-') {
            Some(2)
        } else {
            None
        }
    }

    /// Parse a code fence opening (`` ``` `` or `~~~`).
    pub fn parse_code_fence_opening(value: &str) -> Option<CodeFenceOpening> {
        let trimmed = value.trim_end();
        let ch = trimmed.chars().next()?;
        if ch != '`' && ch != '~' {
            return None;
        }
        let len = trimmed.chars().take_while(|&c| c == ch).count();
        if len < 3 {
            return None;
        }
        let rest = &trimmed[ch.len_utf8() * len..];
        if ch == '`' && rest.contains('`') {
            return None;
        }
        let language = rest.trim();
        Some(CodeFenceOpening {
            ch,
            len,
            language: if language.is_empty() {
                None
            } else {
                Some(language.to_string().into())
            },
        })
    }

    /// Detect a thematic break line (`---`, `***`, `___`).
    pub fn parse_thematic_break_line(value: &str) -> bool {
        let trimmed_end = value.trim_end();
        let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
        if leading_spaces > 3 {
            return false;
        }
        let rest = &trimmed_end[leading_spaces..];
        let mut marker = None;
        let mut marker_count = 0usize;
        for ch in rest.chars() {
            if ch == ' ' {
                continue;
            }
            if !matches!(ch, '-' | '*' | '_') {
                return false;
            }
            if let Some(existing) = marker {
                if existing != ch {
                    return false;
                }
            } else {
                marker = Some(ch);
            }
            marker_count += 1;
        }
        marker_count >= 3
    }

    /// Parse a task-list item marker: `[ ]`, `[x]`, `[X]`.
    pub fn parse_task_list_item_prefix(value: &str) -> Option<(bool, usize)> {
        let bytes = value.as_bytes();
        if bytes.len() < 3 || bytes[0] != b'[' || bytes[2] != b']' {
            return None;
        }
        let checked = match bytes[1] {
            b' ' => false,
            b'x' | b'X' => true,
            _ => return None,
        };
        if bytes.len() == 3 {
            return Some((checked, 3));
        }
        if matches!(bytes[3], b' ' | b'\t') {
            Some((checked, 4))
        } else {
            None
        }
    }
}
