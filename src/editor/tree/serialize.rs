//! Document serialization — tree to Markdown text.
//!
//! Rendered mode serializes the semantic block tree back to normalized
//! Markdown. Source mode writes the raw source buffer directly so literal
//! delimiters are preserved. The save dialogs that use this text live in
//! `crate::editor::file`.

use gpui::*;

use crate::editor::controller::Editor;

fn longest_marker_run(text: &str, marker: char) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;

    for ch in text.chars() {
        if ch == marker {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }

    longest
}

pub(crate) fn safe_code_fence(content: &str) -> String {
    let longest_backticks = longest_marker_run(content, '`');
    if longest_backticks < 3 {
        return "```".to_string();
    }

    let longest_tildes = longest_marker_run(content, '~');
    "~".repeat(longest_tildes.max(2) + 1)
}

pub(crate) fn safe_code_fence_with_info(content: &str, info: Option<&str>) -> String {
    if info.is_some_and(|info| info.contains('`')) {
        let longest_tildes = longest_marker_run(content, '~');
        return "~".repeat(longest_tildes.max(2) + 1);
    }

    safe_code_fence(content)
}

impl Editor {
    pub(crate) fn serialized_document_text(&self, cx: &App) -> String {
        if self.tab().mode == crate::editor::controller::EditorMode::SourceCode {
            self.doc().serialize_source_text(cx)
        } else {
            self.doc().serialize_markdown(cx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{safe_code_fence, safe_code_fence_with_info};

    #[test]
    pub(crate) fn safe_code_fence_is_longer_than_any_inner_backtick_run() {
        assert_eq!(safe_code_fence("plain code"), "```");
        assert_eq!(safe_code_fence("```\ncode"), "~~~");
        assert_eq!(safe_code_fence("value = `````"), "~~~");
        assert_eq!(safe_code_fence("```\n~~~"), "~~~~");
    }

    #[test]
    pub(crate) fn safe_code_fence_with_info_uses_tildes_when_info_contains_backticks() {
        assert_eq!(
            safe_code_fence_with_info("plain code", Some("we`rd")),
            "~~~"
        );
        assert_eq!(
            safe_code_fence_with_info("plain\n~~~\ncode", Some("we`rd")),
            "~~~~"
        );
        assert_eq!(safe_code_fence_with_info("plain code", Some("rust")), "```");
    }
}
