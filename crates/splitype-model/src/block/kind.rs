//! Block kind enumeration — the semantic type of every block in the document
//! tree.
//!
//! Each variant maps to a CommonMark / GFM block-level syntax element and
//! determines both how the block is parsed from Markdown and how it is rendered.

use gpui::SharedString;

use super::callout::CalloutKind;
use super::fence::CodeFenceOpening;

/// The semantic kind of a block.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BlockKind {
    /// Plain paragraph with inline formatting.
    Paragraph,
    /// ATX or Setext heading with a CommonMark heading level (1–6).
    Heading { level: u8 },
    /// Thematic break (CommonMark §4.1): a line of `---`, `***`, or `___`.
    ThematicBreak,
    /// Unordered list item (CommonMark §5.2).
    BulletListItem,
    /// Task-list item with checked state (GFM extension).
    TaskListItem { checked: bool },
    /// Ordered list item (CommonMark §5.3); serialization uses canonical dot markers.
    NumberedListItem,
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
    MathBlock,
    /// Mermaid fenced block rendered as SVG.
    MermaidBlock,
    /// Raw Markdown fallback for syntax outside the native runtime subset.
    RawMarkdown,
}

impl BlockKind {
    /// Returns true when blocks of this kind may own child blocks in the
    /// current runtime tree.
    pub fn supports_children(&self) -> bool {
        self.is_list_item() || self.is_quote_container() || self.is_footnote_definition()
    }

    pub fn is_list_item(&self) -> bool {
        matches!(
            self,
            Self::BulletListItem | Self::TaskListItem { .. } | Self::NumberedListItem
        )
    }

    pub fn is_numbered_list_item(&self) -> bool {
        matches!(self, Self::NumberedListItem)
    }

    pub fn is_task_list_item(&self) -> bool {
        matches!(self, Self::TaskListItem { .. })
    }

    pub fn is_code_block(&self) -> bool {
        matches!(self, Self::CodeBlock { .. })
    }

    /// Block kinds whose text is edited and displayed as highlighted code:
    /// code blocks plus math and diagram blocks share the code editor UI.
    pub fn uses_code_highlighting(&self) -> bool {
        matches!(
            self,
            Self::CodeBlock { .. } | Self::MathBlock | Self::MermaidBlock
        )
    }

    /// Whether the right-click "Insert Table" affordance makes sense when a
    /// block of this kind is the target. Atomic/structural blocks (tables,
    /// code, math, etc.) render as self-contained widgets where inserting a
    /// table from within them is nonsensical, so they are excluded.
    pub fn allows_context_table_insert(&self) -> bool {
        !matches!(
            self,
            Self::Table
                | Self::CodeBlock { .. }
                | Self::MathBlock
                | Self::MermaidBlock
                | Self::HtmlBlock
                | Self::HtmlComment
                | Self::RawMarkdown
        )
    }

    pub fn is_quote_container(&self) -> bool {
        matches!(self, Self::Blockquote | Self::Callout(_))
    }

    /// Blocks that render as self-contained widgets with no caret position
    /// after them. At the end of a rendered document they need a trailing
    /// paragraph so a rendered-first user can keep typing past the structure
    /// instead of having to drop to source mode.
    pub fn is_atomic_structural(&self) -> bool {
        matches!(
            self,
            Self::ThematicBreak
                | Self::Table
                | Self::CodeBlock { .. }
                | Self::MathBlock
                | Self::MermaidBlock
                | Self::HtmlBlock
                | Self::HtmlComment
                | Self::RawMarkdown
        )
    }

    pub fn is_callout(&self) -> bool {
        matches!(self, Self::Callout(_))
    }

    /// Blocks edited as multi-line raw text that render as self-contained
    /// widgets (code, math, HTML, mermaid, comment, raw markdown). Exiting one
    /// downward with `Down` or `Ctrl/Cmd+Enter` needs a line below to land on.
    pub fn is_multiline_text_block(&self) -> bool {
        self.is_code_block()
            || matches!(
                self,
                Self::MathBlock
                    | Self::HtmlBlock
                    | Self::MermaidBlock
                    | Self::HtmlComment
                    | Self::RawMarkdown
            )
    }

    pub fn is_footnote_definition(&self) -> bool {
        matches!(self, Self::FootnoteDefinition)
    }

    pub fn callout_kind(&self) -> Option<CalloutKind> {
        match self {
            Self::Callout(kind) => Some(*kind),
            _ => None,
        }
    }

    pub fn is_thematic_break(&self) -> bool {
        matches!(self, Self::ThematicBreak)
    }

    pub fn can_nest_under(&self, parent: &Self) -> bool {
        if !parent.is_list_item() {
            return false;
        }

        self.is_list_item()
            || matches!(
                self,
                Self::Paragraph
                    | Self::Blockquote
                    | Self::Callout(_)
                    | Self::FootnoteDefinition
                    | Self::Table
                    | Self::CodeBlock { .. }
                    | Self::HtmlComment
                    | Self::HtmlBlock
                    | Self::MathBlock
                    | Self::MermaidBlock
                    | Self::RawMarkdown
            )
    }

    pub fn newline_sibling_kind(&self) -> Self {
        if matches!(self, Self::TaskListItem { .. }) {
            Self::TaskListItem { checked: false }
        } else if self.is_list_item() {
            self.clone()
        } else if self.is_quote_container() {
            self.clone()
        } else {
            Self::Paragraph
        }
    }

    /// Live-detects a Markdown prefix from user input and returns the
    /// corresponding [`BlockKind`] together with the character count of
    /// the prefix that should be stripped.
    pub fn detect_markdown_shortcut(value: &str) -> Option<(Self, usize)> {
        if value.starts_with("###### ") {
            Some((Self::Heading { level: 6 }, 7))
        } else if value.starts_with("##### ") {
            Some((Self::Heading { level: 5 }, 6))
        } else if value.starts_with("#### ") {
            Some((Self::Heading { level: 4 }, 5))
        } else if value.starts_with("### ") {
            Some((Self::Heading { level: 3 }, 4))
        } else if value.starts_with("## ") {
            Some((Self::Heading { level: 2 }, 3))
        } else if value.starts_with("# ") {
            Some((Self::Heading { level: 1 }, 2))
        } else if let Some((checked, prefix_len)) = Self::parse_task_list_shortcut(value) {
            Some((Self::TaskListItem { checked }, prefix_len))
        } else if value.starts_with("* ") || value.starts_with("+ ") {
            Some((Self::BulletListItem, 2))
        } else if value.starts_with("- ") {
            Some((Self::BulletListItem, 2))
        } else if let Some(prefix_len) = Self::numbered_list_shortcut_prefix_len(value) {
            Some((Self::NumberedListItem, prefix_len))
        } else if value.starts_with("> ") {
            Some((Self::Blockquote, 2))
        } else {
            None
        }
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

    fn parse_task_list_shortcut(value: &str) -> Option<(bool, usize)> {
        let rest = value.strip_prefix("- ")?;
        let (checked, prefix_len) = Self::parse_task_list_item_prefix(rest)?;
        Some((checked, 2 + prefix_len))
    }

    fn numbered_list_shortcut_prefix_len(value: &str) -> Option<usize> {
        let digit_len = value.bytes().take_while(|b| b.is_ascii_digit()).count();
        if !(1..=9).contains(&digit_len) {
            return None;
        }

        let marker = *value.as_bytes().get(digit_len)?;
        if !matches!(marker, b'.' | b')') {
            return None;
        }

        let separator = *value.as_bytes().get(digit_len + 1)?;
        matches!(separator, b' ' | b'\t').then_some(digit_len + 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_table_insert_excludes_atomic_blocks() {
        assert!(BlockKind::Paragraph.allows_context_table_insert());
        assert!(BlockKind::Heading { level: 1 }.allows_context_table_insert());
        assert!(BlockKind::Blockquote.allows_context_table_insert());
        assert!(!BlockKind::Table.allows_context_table_insert());
        assert!(!BlockKind::CodeBlock { language: None }.allows_context_table_insert());
        assert!(!BlockKind::MathBlock.allows_context_table_insert());
        assert!(!BlockKind::MermaidBlock.allows_context_table_insert());
    }

    #[test]
    fn detects_markdown_shortcuts() {
        assert_eq!(
            BlockKind::detect_markdown_shortcut("- item"),
            Some((BlockKind::BulletListItem, 2))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("1. item"),
            Some((BlockKind::NumberedListItem, 3))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("12. item"),
            Some((BlockKind::NumberedListItem, 4))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("1) item"),
            Some((BlockKind::NumberedListItem, 3))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("12)\titem"),
            Some((BlockKind::NumberedListItem, 4))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("1234567890) item"),
            None
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("# heading"),
            Some((BlockKind::Heading { level: 1 }, 2))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("## heading"),
            Some((BlockKind::Heading { level: 2 }, 3))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("### heading"),
            Some((BlockKind::Heading { level: 3 }, 4))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("#### heading"),
            Some((BlockKind::Heading { level: 4 }, 5))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("##### heading"),
            Some((BlockKind::Heading { level: 5 }, 6))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("###### heading"),
            Some((BlockKind::Heading { level: 6 }, 7))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("- [ ] task"),
            Some((BlockKind::TaskListItem { checked: false }, 6))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("- [x] task"),
            Some((BlockKind::TaskListItem { checked: true }, 6))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("* item"),
            Some((BlockKind::BulletListItem, 2))
        );
        assert_eq!(
            BlockKind::detect_markdown_shortcut("+ item"),
            Some((BlockKind::BulletListItem, 2))
        );
        assert_eq!(BlockKind::detect_markdown_shortcut("#no-space"), None);
        assert_eq!(BlockKind::detect_markdown_shortcut("```"), None);
        assert_eq!(
            BlockKind::detect_markdown_shortcut("> quote"),
            Some((BlockKind::Blockquote, 2))
        );
        assert_eq!(BlockKind::detect_markdown_shortcut(">no-space"), None);
    }

    #[test]
    fn parses_thematic_break_lines() {
        assert!(BlockKind::parse_thematic_break_line("---"));
        assert!(BlockKind::parse_thematic_break_line("----"));
        assert!(BlockKind::parse_thematic_break_line("***"));
        assert!(BlockKind::parse_thematic_break_line("_ _ _"));
        assert!(BlockKind::parse_thematic_break_line(" - - - "));
        assert!(!BlockKind::parse_thematic_break_line("--"));
        assert!(BlockKind::parse_thematic_break_line(" ---"));
        assert!(!BlockKind::parse_thematic_break_line("---x"));
    }

    #[test]
    fn parses_code_fence_openings() {
        assert_eq!(
            BlockKind::parse_code_fence_opening("```rust"),
            Some(CodeFenceOpening {
                ch: '`',
                len: 3,
                language: Some("rust".into()),
            })
        );
        assert_eq!(
            BlockKind::parse_code_fence_opening("~~~ts"),
            Some(CodeFenceOpening {
                ch: '~',
                len: 3,
                language: Some("ts".into()),
            })
        );
        assert_eq!(
            BlockKind::parse_code_fence_opening("```"),
            Some(CodeFenceOpening {
                ch: '`',
                len: 3,
                language: None,
            })
        );
        assert_eq!(BlockKind::parse_code_fence_opening("``"), None);
        assert_eq!(BlockKind::parse_code_fence_opening("```ru`st"), None);
    }

    #[test]
    fn parses_task_list_item_prefixes() {
        assert_eq!(
            BlockKind::parse_task_list_item_prefix("[ ] a"),
            Some((false, 4))
        );
        assert_eq!(
            BlockKind::parse_task_list_item_prefix("[x] a"),
            Some((true, 4))
        );
        assert_eq!(
            BlockKind::parse_task_list_item_prefix("[X] a"),
            Some((true, 4))
        );
        assert_eq!(BlockKind::parse_task_list_item_prefix("[a] a"), None);
    }

    #[test]
    fn parses_h2_and_h3_lines_with_correct_levels() {
        let h2 = BlockKind::parse_atx_heading_line("## hello");
        assert_eq!(h2, Some((2, "hello".to_string())));

        let h3 = BlockKind::parse_atx_heading_line("### hello");
        assert_eq!(h3, Some((3, "hello".to_string())));
    }

    #[test]
    fn parses_atx_headings_with_closing_hashes_and_setext_underlines() {
        let atx = BlockKind::parse_atx_heading_line("  ### title ######");
        assert_eq!(atx, Some((3, "title".to_string())));

        assert_eq!(BlockKind::parse_setext_underline("==="), Some(1));
        assert_eq!(BlockKind::parse_setext_underline("---"), Some(2));
        assert_eq!(BlockKind::parse_setext_underline("- - -"), None);
    }

    #[test]
    fn code_block_kind_stores_language() {
        let kind = BlockKind::CodeBlock {
            language: Some(SharedString::from("rust")),
        };
        assert!(kind.is_code_block());
        assert!(!kind.is_list_item());

        let no_lang = BlockKind::CodeBlock { language: None };
        assert!(no_lang.is_code_block());
    }
}
