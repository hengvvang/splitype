//! Fenced code block opening metadata.
//!
//! Records the fence character and run length so only a matching
//! closing fence can terminate the block.

/// Opening fence parsed from a fenced code block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeFenceOpening {
    /// Fence character: backtick `` ` `` or tilde `~`.
    pub ch: char,
    /// Length of the opening fence run.
    pub len: usize,
    /// Optional language / info string after the opening fence.
    pub language: Option<String>,
}

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

/// Generates a safe Markdown code fence delimiter (`` ``` `` or `~~~`) that is
/// strictly longer than any internal fence marker run in `content`.
pub fn safe_code_fence(content: &str) -> String {
    let longest_backticks = longest_marker_run(content, '`');
    if longest_backticks < 3 {
        return "```".to_string();
    }

    let longest_tildes = longest_marker_run(content, '~');
    "~".repeat(longest_tildes.max(2) + 1)
}

/// Generates a safe Markdown code fence delimiter, switching to tildes if the
/// info string contains backticks.
pub fn safe_code_fence_with_info(content: &str, info: Option<&str>) -> String {
    if info.is_some_and(|info| info.contains('`')) {
        let longest_tildes = longest_marker_run(content, '~');
        return "~".repeat(longest_tildes.max(2) + 1);
    }

    safe_code_fence(content)
}

