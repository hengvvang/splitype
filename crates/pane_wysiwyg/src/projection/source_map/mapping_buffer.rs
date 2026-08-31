//! Buffer and prefix offset mapping calculators for SourceMap generation.

use crate::model::Document;
use crate::model::block::Block;

/// Checks if the block is an empty root paragraph.
pub fn is_empty_root_paragraph(block: &Block) -> bool {
    Document::is_empty_root_paragraph(block)
}

/// Computes the forward and backward index mappings for content with leading
/// and continuation line prefixes.
pub fn build_prefixed_content_mapping(
    content: &str,
    first_prefix: &str,
    continuation_prefix: &str,
) -> (String, Vec<usize>, Vec<usize>) {
    let mut full = String::new();
    let mut content_to_source = vec![0; content.len() + 1];
    let mut source_to_content = vec![0];

    full.push_str(first_prefix);
    source_to_content.resize(full.len() + 1, 0);

    let mut content_offset = 0usize;
    while content_offset < content.len() {
        content_to_source[content_offset] = full.len();
        let ch = content[content_offset..]
            .chars()
            .next()
            .expect("content offset should stay on char boundaries");
        let start = full.len();
        full.push(ch);
        source_to_content.resize(full.len() + 1, content_offset);
        for index in start..=full.len() {
            source_to_content[index] = content_offset;
        }
        content_offset += ch.len_utf8();
        if ch == '\n' {
            let prefix_start = full.len();
            full.push_str(continuation_prefix);
            source_to_content.resize(full.len() + 1, content_offset);
            for index in prefix_start..=full.len() {
                source_to_content[index] = content_offset;
            }
        }
    }
    content_to_source[content.len()] = full.len();
    source_to_content[full.len()] = content.len();

    (full, content_to_source, source_to_content)
}

/// Computes the forward and backward index mappings for a fenced code block.
pub fn build_code_block_content_mapping(
    content: &str,
    indentation: &str,
    language: Option<&str>,
) -> (String, Vec<usize>, Vec<usize>) {
    let fence = crate::model::serialize::safe_code_fence_with_info(content, language);
    let mut full = String::new();
    let mut content_to_source = vec![0; content.len() + 1];
    let mut source_to_content = vec![0];

    full.push_str(&fence);
    if let Some(language) = language {
        full.push_str(language);
    }
    full.push('\n');
    source_to_content.resize(full.len() + 1, 0);

    let prefix_start = full.len();
    full.push_str(indentation);
    source_to_content.resize(full.len() + 1, 0);
    for index in prefix_start..=full.len() {
        source_to_content[index] = 0;
    }

    let mut content_offset = 0usize;
    while content_offset < content.len() {
        content_to_source[content_offset] = full.len();
        let ch = content[content_offset..]
            .chars()
            .next()
            .expect("content offset should stay on char boundaries");
        let start = full.len();
        full.push(ch);
        source_to_content.resize(full.len() + 1, content_offset);
        for index in start..=full.len() {
            source_to_content[index] = content_offset;
        }
        content_offset += ch.len_utf8();
        if ch == '\n' {
            let line_prefix_start = full.len();
            full.push_str(indentation);
            source_to_content.resize(full.len() + 1, content_offset);
            for index in line_prefix_start..=full.len() {
                source_to_content[index] = content_offset;
            }
        }
    }
    content_to_source[content.len()] = full.len();
    source_to_content[full.len()] = content.len();

    full.push('\n');
    source_to_content.resize(full.len() + 1, content.len());
    full.push_str(&fence);
    source_to_content.resize(full.len() + 1, content.len());
    source_to_content[full.len()] = content.len();

    (full, content_to_source, source_to_content)
}


