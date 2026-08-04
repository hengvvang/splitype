//! Pure Markdown-to-NodeTree parser.
//!
//! Converts raw Markdown text into a flat list of [`NodeData`] with
//! parent-child relationships expressed through [`NodeId`] references.
//! This module has no GPUI context or entity dependencies — it operates
//! entirely on plain data types from `core::ast` and `core::text`.

use crate::core::ast::block_kind::BlockKind;
use crate::core::ast::callout_kind::CalloutKind;
use crate::core::ast::code_fence::CodeFenceOpening;
use crate::core::ast::node::NodeData;
use crate::core::extensions::footnote_def::parse_footnote_definition_head;
use crate::core::extensions::html_doc::{HtmlSafetyClass, parse_html_document};
use crate::core::extensions::image_ref::parse_standalone_image;
use crate::core::extensions::mermaid_source::is_mermaid_info_string;
use crate::core::extensions::table::{
    collect_pipeless_table_region, collect_root_table_candidate_region,
    collect_table_candidate_region, is_root_table_candidate_line, is_table_candidate_line,
    parse_root_table_region, parse_table_region,
};
use crate::core::io::helpers::{
    collect_until_blank_line, dedent_lines, display_columns, is_quote_start,
    leading_indent_columns_and_bytes, strip_fence_indent, strip_indented_code_prefix,
    strip_leading_columns, strip_one_quote_level,
};
use crate::core::text::rich_text::RichText;
use crate::services::latex_render::parse_display_math_source;

// ---------------------------------------------------------------------------
// Local types
// ---------------------------------------------------------------------------

/// Parsed opening code-fence metadata.
type FenceInfo = CodeFenceOpening;

/// HTML block form recognized by the Markdown importer.
enum HtmlBlockStart {
    /// HTML comment region beginning with `<!--`.
    Comment,
    /// HTML tag block whose closing behavior depends on the tag shape.
    Tag {
        name: String,
        self_closing: bool,
        closes_same_line: bool,
    },
}

/// Ordered-list or unordered-list marker parsed from one source line.
#[derive(Clone)]
struct ListMarker {
    kind: BlockKind,
    indent_columns: usize,
    content_indent_columns: usize,
    text: String,
}

// ---------------------------------------------------------------------------
// Utility / preamble helpers
// ---------------------------------------------------------------------------

fn collect_html_fallback_region(lines: &[String], start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() {
        if lines[index].trim().is_empty()
            || looks_like_root_block_start(lines, index)
            || parse_standalone_image(&lines[index]).is_some()
        {
            break;
        }
        index += 1;
    }
    index
}

fn pending_inline_code_run_len(markdown: &str) -> Option<usize> {
    let mut open_run_len = None;
    let mut chars = markdown.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        if open_run_len.is_none() && ch == '\\' {
            let _ = chars.next();
            continue;
        }

        if ch != '`' {
            continue;
        }

        let mut run_len = 1usize;
        while chars.peek().is_some_and(|(_, ch)| *ch == '`') {
            let _ = chars.next();
            run_len += 1;
        }

        if open_run_len == Some(run_len) {
            open_run_len = None;
        } else if open_run_len.is_none() {
            open_run_len = Some(run_len);
        }
    }

    open_run_len
}

fn line_contains_matching_backtick_run(line: &str, run_len: usize) -> bool {
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '`' {
            continue;
        }

        let mut current_run_len = 1usize;
        while chars.peek().is_some_and(|ch| *ch == '`') {
            let _ = chars.next();
            current_run_len += 1;
        }

        if current_run_len == run_len {
            return true;
        }
    }

    false
}

fn paragraph_can_continue_through_boundary(
    paragraph_lines: &[String],
    lines: &[String],
    boundary_index: usize,
) -> bool {
    let Some(run_len) = pending_inline_code_run_len(&paragraph_lines.join("\n")) else {
        return false;
    };

    lines[boundary_index..]
        .iter()
        .any(|line| line_contains_matching_backtick_run(line, run_len))
}

fn parse_opening_fence(line: &str) -> Option<FenceInfo> {
    BlockKind::parse_code_fence_opening(strip_fence_indent(line)?.trim_end())
}

fn is_closing_fence(line: &str, opener: &FenceInfo) -> bool {
    let Some(trimmed) = strip_fence_indent(line).map(str::trim_end) else {
        return false;
    };
    if !trimmed.starts_with(opener.ch) {
        return false;
    }
    let run_len = trimmed.chars().take_while(|&c| c == opener.ch).count();
    if run_len != opener.len {
        return false;
    }
    trimmed[opener.ch.len_utf8() * run_len..].trim().is_empty()
}

fn find_matching_closing_fence(
    lines: &[String],
    start_index: usize,
    opener: &FenceInfo,
) -> Option<usize> {
    for index in (start_index + 1)..lines.len() {
        let line = &lines[index];
        if is_closing_fence(line, opener) {
            return Some(index);
        }

        if parse_opening_fence(line)
            .as_ref()
            .and_then(|fence| fence.language.as_ref())
            .is_some()
        {
            break;
        }
    }

    None
}

fn parse_list_marker(line: &str) -> Option<ListMarker> {
    let (indent_columns, indent_bytes) = leading_indent_columns_and_bytes(line);
    let rest = &line[indent_bytes..];

    if let Some(marker) = rest.chars().next()
        && matches!(marker, '-' | '*' | '+')
    {
        let after_marker = &rest[marker.len_utf8()..];
        let separator_len = after_marker
            .chars()
            .next()
            .filter(|ch| matches!(ch, ' ' | '\t'))
            .map(char::len_utf8)?;
        let text = after_marker
            .strip_prefix(' ')
            .or_else(|| after_marker.strip_prefix('\t'))?;
        let (kind, text) =
            if let Some((checked, prefix_len)) = BlockKind::parse_task_list_item_prefix(text) {
                (
                    BlockKind::TaskListItem { checked },
                    text[prefix_len..].to_string(),
                )
            } else {
                (BlockKind::UnorderedListItem, text.to_string())
            };
        return Some(ListMarker {
            kind,
            indent_columns,
            content_indent_columns: display_columns(
                &line[..indent_bytes + marker.len_utf8() + separator_len],
            ),
            text,
        });
    }

    let (digit_len, marker_len, text) = parse_ordered_list_marker(rest)?;
    Some(ListMarker {
        kind: BlockKind::OrderedListItem,
        indent_columns,
        content_indent_columns: display_columns(&line[..indent_bytes + digit_len + marker_len]),
        text: text.to_string(),
    })
}

fn parse_ordered_list_marker(rest: &str) -> Option<(usize, usize, &str)> {
    let digit_len = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if !(1..=9).contains(&digit_len) {
        return None;
    }

    let marker = *rest.as_bytes().get(digit_len)?;
    if !matches!(marker, b'.' | b')') {
        return None;
    }

    let separator = *rest.as_bytes().get(digit_len + 1)?;
    if !matches!(separator, b' ' | b'\t') {
        return None;
    }

    Some((digit_len, 2, &rest[digit_len + 2..]))
}

fn is_reference_definition_start(line: &str) -> bool {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return false;
    }

    let rest = &trimmed_end[leading_spaces..];
    let Some(label_end) = rest.find("]:") else {
        return false;
    };
    rest.starts_with('[') && label_end > 1
}

fn is_footnote_definition_start(line: &str) -> bool {
    let trimmed_end = line.trim_end();
    let leading_spaces = trimmed_end.bytes().take_while(|b| *b == b' ').count();
    if leading_spaces > 3 {
        return false;
    }

    let rest = &trimmed_end[leading_spaces..];
    let Some(label_end) = rest.find("]:") else {
        return false;
    };
    rest.starts_with("[^") && label_end > 2
}

fn is_reference_definition_title_continuation(line: &str) -> bool {
    let (_, indent_bytes) = leading_indent_columns_and_bytes(line);
    if indent_bytes == 0 {
        return false;
    }

    let trimmed = line[indent_bytes..].trim();
    (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('(') && trimmed.ends_with(')'))
}

fn is_block_html_start(line: &str) -> bool {
    parse_html_block_start(line).is_some()
}

fn collect_closed_html_comment_region(lines: &[String], start: usize) -> Option<usize> {
    match parse_html_block_start(&lines[start])? {
        HtmlBlockStart::Comment => {}
        HtmlBlockStart::Tag { .. } => return None,
    }

    if lines[start].contains("-->") {
        return Some(start + 1);
    }

    let mut index = start + 1;
    while index < lines.len() {
        if lines[index].contains("-->") {
            return Some(index + 1);
        }
        index += 1;
    }

    None
}

fn collect_block_html_region(lines: &[String], start: usize) -> usize {
    match parse_html_block_start(&lines[start]) {
        Some(HtmlBlockStart::Comment) => collect_closed_html_comment_region(lines, start)
            .unwrap_or_else(|| collect_html_fallback_region(lines, start)),
        Some(HtmlBlockStart::Tag {
            name,
            self_closing,
            closes_same_line,
        }) => {
            if self_closing || closes_same_line {
                return start + 1;
            }

            let mut depth = 1usize;
            let mut index = start + 1;
            while index < lines.len() {
                if let Some(HtmlBlockStart::Tag {
                    name: nested_name,
                    self_closing,
                    closes_same_line,
                }) = parse_html_block_start(&lines[index])
                    && nested_name == name
                    && !self_closing
                    && !closes_same_line
                {
                    depth += 1;
                }

                if let Some(close_name) = parse_html_close_tag_name(&lines[index])
                    && close_name == name
                {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return index + 1;
                    }
                }

                index += 1;
            }
            collect_html_fallback_region(lines, start)
        }
        None => collect_until_blank_line(lines, start),
    }
}

fn collect_reference_definition_region(lines: &[String], start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() && is_reference_definition_title_continuation(&lines[index]) {
        index += 1;
    }
    index
}

fn collect_footnote_definition_region(lines: &[String], start: usize) -> usize {
    let mut index = start + 1;
    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            index += 1;
            continue;
        }

        let (indent_columns, _) = leading_indent_columns_and_bytes(line);
        if indent_columns > 0 {
            index += 1;
            continue;
        }

        break;
    }
    index
}

fn is_display_math_start(line: &str) -> bool {
    strip_fence_indent(line)
        .map(str::trim_end)
        .is_some_and(|rest| rest.starts_with("$$"))
}

fn collect_display_math_region(lines: &[String], start: usize) -> usize {
    let opener = strip_fence_indent(&lines[start])
        .map(str::trim_end)
        .unwrap_or_default();
    if opener != "$$" && opener[2..].contains("$$") {
        return start + 1;
    }

    let mut index = start + 1;
    while index < lines.len() {
        if lines[index].trim() == "$$" {
            return index + 1;
        }

        if lines[index].trim().is_empty() {
            let mut lookahead = index + 1;
            while lookahead < lines.len() && lines[lookahead].trim().is_empty() {
                lookahead += 1;
            }

            if lookahead >= lines.len() || looks_like_root_block_start(lines, lookahead) {
                return lookahead;
            }
        }

        index += 1;
    }

    lines.len()
}

fn parse_html_block_start(line: &str) -> Option<HtmlBlockStart> {
    let rest = strip_fence_indent(line)?.trim_end();
    if rest.starts_with("<!--") {
        return Some(HtmlBlockStart::Comment);
    }

    let tagged = rest.strip_prefix('<')?;
    if tagged.starts_with('/') {
        return None;
    }

    let name_len = tagged
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .count();
    if name_len == 0 {
        return None;
    }

    let name = &tagged[..name_len];
    let suffix = &tagged[name_len..];
    let next = suffix.chars().next()?;
    if !matches!(next, '>' | ' ' | '\t' | '/') {
        return None;
    }

    Some(HtmlBlockStart::Tag {
        name: name.to_string(),
        self_closing: rest.ends_with("/>") || is_html_void_block_tag(name),
        closes_same_line: rest.contains(&format!("</{name}>")),
    })
}

fn is_html_void_block_tag(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "br" | "hr" | "img")
}

fn parse_html_close_tag_name(line: &str) -> Option<String> {
    let rest = strip_fence_indent(line)?.trim_end();
    let tagged = rest.strip_prefix("</")?;
    let name_len = tagged
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .count();
    if name_len == 0 {
        return None;
    }

    let name = &tagged[..name_len];
    let suffix = &tagged[name_len..];
    let next = suffix.chars().next()?;
    if !matches!(next, '>' | ' ' | '\t') {
        return None;
    }

    Some(name.to_string())
}

fn collect_quote_raw_region(lines: &[String], start: usize) -> usize {
    let mut index = start;
    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() || !is_quote_start(line) {
            break;
        }
        index += 1;
    }
    index
}

fn quote_content_starts_unsupported(lines: &[String], index: usize) -> bool {
    let line = &lines[index];
    is_block_html_start(line)
        || is_footnote_definition_start(line)
        || is_reference_definition_start(line)
        || is_root_table_candidate_line(line)
        || is_display_math_start(line)
        || BlockKind::parse_atx_heading_line(line).is_some()
        || BlockKind::parse_thematic_break_line(line)
        || lines
            .get(index + 1)
            .and_then(|next| BlockKind::parse_setext_underline(next))
            .is_some()
}

fn collect_unsupported_quote_region(lines: &[String], start: usize) -> Option<usize> {
    if start >= lines.len() {
        return None;
    }

    let line = &lines[start];
    if is_block_html_start(line) {
        return Some(collect_block_html_region(lines, start));
    }
    if is_footnote_definition_start(line) {
        return Some(collect_footnote_definition_region(lines, start));
    }
    if is_reference_definition_start(line) {
        return Some(collect_reference_definition_region(lines, start));
    }
    if is_root_table_candidate_line(line) {
        return Some(collect_root_table_candidate_region(lines, start));
    }
    if is_display_math_start(line) {
        return Some(collect_display_math_region(lines, start));
    }
    if BlockKind::parse_atx_heading_line(line).is_some()
        || BlockKind::parse_thematic_break_line(line)
    {
        return Some(start + 1);
    }
    if lines
        .get(start + 1)
        .and_then(|next| BlockKind::parse_setext_underline(next))
        .is_some()
    {
        return Some((start + 2).min(lines.len()));
    }

    None
}

fn collect_list_item_region(lines: &[String], start: usize, marker_indent_columns: usize) -> usize {
    let mut index = start + 1;
    let mut pending_blank_lines = 0usize;
    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            index += 1;
            continue;
        }

        if parse_list_marker(line)
            .is_some_and(|marker| marker.indent_columns <= marker_indent_columns)
        {
            return index.saturating_sub(pending_blank_lines);
        }

        if parse_list_marker(line).is_some() {
            pending_blank_lines = 0;
            index += 1;
            continue;
        }

        let (indent_columns, _) = leading_indent_columns_and_bytes(line);
        if indent_columns > marker_indent_columns || pending_blank_lines == 0 {
            pending_blank_lines = 0;
            index += 1;
            continue;
        }

        return index.saturating_sub(pending_blank_lines);
    }
    index
}

fn looks_like_root_block_start(lines: &[String], index: usize) -> bool {
    let line = &lines[index];
    if line.trim().is_empty() {
        return true;
    }

    parse_opening_fence(line).is_some()
        || is_block_html_start(line)
        || is_footnote_definition_start(line)
        || is_reference_definition_start(line)
        || strip_indented_code_prefix(line).is_some()
        || parse_list_marker(line).is_some()
        || is_quote_start(line)
        || BlockKind::parse_atx_heading_line(line).is_some()
        || BlockKind::parse_thematic_break_line(line)
        || lines
            .get(index + 1)
            .and_then(|next| BlockKind::parse_setext_underline(next))
            .is_some()
        || is_root_table_candidate_line(line)
        || is_display_math_start(line)
}

// ---------------------------------------------------------------------------
// Node-tree relationship helpers
// ---------------------------------------------------------------------------

/// Set up parent -> child relationship between two `NodeData` nodes.
fn attach_child_node(parent: &mut NodeData, child: &mut NodeData) {
    child.parent_id = Some(parent.id);
    parent.child_ids.push(child.id);
}

/// Set up parent -> children relationships for multiple children.
fn attach_child_nodes(parent: &mut NodeData, children: &mut [NodeData]) {
    for child in children.iter_mut() {
        child.parent_id = Some(parent.id);
        parent.child_ids.push(child.id);
    }
}

// ---------------------------------------------------------------------------
// Block constructors (pure - no GPUI)
// ---------------------------------------------------------------------------

fn native_block(kind: BlockKind, markdown: &str) -> NodeData {
    NodeData::new(kind, RichText::from_markdown(markdown))
}

fn build_code_block(language: Option<gpui::SharedString>, content: String) -> NodeData {
    NodeData::with_plain_text(BlockKind::CodeBlock { language }, content)
}

fn raw_block(markdown: String) -> NodeData {
    NodeData::raw_markdown(markdown)
}

fn comment_block(markdown: String) -> NodeData {
    NodeData::html_comment(markdown)
}

fn html_or_raw_block(markdown: String) -> NodeData {
    let document = parse_html_document(&markdown);
    if document.safety == HtmlSafetyClass::Semantic {
        NodeData::html_block(markdown)
    } else {
        raw_block(markdown)
    }
}

fn math_or_raw_block(markdown: String) -> NodeData {
    if parse_display_math_source(&markdown).is_some() {
        NodeData::latex_math(markdown)
    } else {
        raw_block(markdown)
    }
}

fn plain_text_paragraph_block(text: String) -> NodeData {
    NodeData::paragraph(text)
}

fn standalone_image_block(markdown: String) -> NodeData {
    NodeData::paragraph(markdown.trim().to_string())
}

fn is_standalone_image_paragraph(lines: &[String]) -> bool {
    lines.len() == 1 && parse_standalone_image(&lines[0]).is_some()
}

fn starts_with_standalone_image_child_paragraph(lines: &[String]) -> bool {
    if lines.is_empty() || !is_standalone_image_paragraph(&lines[..1]) {
        return false;
    }

    lines.get(1).is_none_or(|next| {
        next.trim().is_empty()
            || parse_list_marker(next).is_some()
            || is_quote_start(next)
            || parse_opening_fence(next).is_some()
            || strip_indented_code_prefix(next).is_some()
            || is_block_html_start(next)
            || is_footnote_definition_start(next)
            || is_reference_definition_start(next)
            || is_root_table_candidate_line(next)
            || is_display_math_start(next)
    })
}

fn append_markdown_to_node(node: &mut NodeData, separator: &str, markdown: &str) {
    if !separator.is_empty() {
        node.title
            .append_tree(RichText::plain(separator.to_string()));
    }
    node.title.append_tree(RichText::from_markdown(markdown));
}

fn append_separator_children(children: &mut Vec<NodeData>, count: usize) {
    for _ in 0..count {
        children.push(NodeData::paragraph(""));
    }
}

// ---------------------------------------------------------------------------
// Collectors - return (block(s), next_index)
// ---------------------------------------------------------------------------

fn collect_fenced_code_block(lines: &[String], start: usize) -> Option<(NodeData, usize)> {
    let fence = parse_opening_fence(&lines[start])?;
    let closing_index = find_matching_closing_fence(lines, start, &fence)?;
    if is_mermaid_info_string(fence.language.as_ref().map(|language| language.as_ref())) {
        let raw = lines[start..=closing_index].join("\n");
        return Some((NodeData::mermaid_diagram(raw), closing_index + 1));
    }

    let code_lines = lines[start + 1..closing_index].to_vec();
    Some((
        build_code_block(fence.language.clone(), code_lines.join("\n")),
        closing_index + 1,
    ))
}

fn collect_indented_code_block(lines: &[String], start: usize) -> Option<(NodeData, usize)> {
    let stripped = strip_indented_code_prefix(&lines[start])?;
    let mut code_lines = vec![stripped.to_string()];
    let mut code_index = start + 1;
    while code_index < lines.len() {
        if let Some(stripped) = strip_indented_code_prefix(&lines[code_index]) {
            code_lines.push(stripped.to_string());
            code_index += 1;
        } else if lines[code_index].trim().is_empty() {
            code_lines.push(String::new());
            code_index += 1;
        } else {
            break;
        }
    }

    Some((build_code_block(None, code_lines.join("\n")), code_index))
}

fn collect_comment_block(lines: &[String], start: usize) -> Option<(NodeData, usize)> {
    let end = collect_closed_html_comment_region(lines, start)?;
    Some((comment_block(lines[start..end].join("\n")), end))
}

fn collect_paragraph_block(lines: &[String], start: usize) -> (NodeData, usize) {
    let mut paragraph_lines = vec![lines[start].to_string()];
    let mut index = start + 1;
    while index < lines.len() {
        if (lines[index].trim().is_empty() || looks_like_root_block_start(lines, index))
            && !paragraph_can_continue_through_boundary(&paragraph_lines, lines, index)
        {
            break;
        }
        paragraph_lines.push(lines[index].to_string());
        index += 1;
    }

    (
        native_block(BlockKind::Paragraph, &paragraph_lines.join("\n")),
        index,
    )
}

// ---------------------------------------------------------------------------
// Compound block builders - return Vec<NodeData> with parent-child ids set up
// ---------------------------------------------------------------------------

fn build_native_footnote_definition_node(lines: &[String]) -> Option<Vec<NodeData>> {
    let (id, first_line) = parse_footnote_definition_head(lines.first()?)?;
    let mut body_lines = Vec::new();
    if !first_line.is_empty() {
        body_lines.push(first_line);
    }

    for line in lines.iter().skip(1) {
        if line.trim().is_empty() {
            body_lines.push(String::new());
        } else {
            body_lines.push(
                strip_leading_columns(line, 4)
                    .unwrap_or(line.as_str())
                    .to_string(),
            );
        }
    }

    let mut children = build_blocks_from_lines_internal(&body_lines, false);
    let mut block = NodeData::with_plain_text(BlockKind::FootnoteDefinition, id);

    attach_child_nodes(&mut block, &mut children);

    let mut result = vec![block];
    result.extend(children);
    Some(result)
}

fn collect_quote_block(lines: &[String], start: usize) -> (Vec<NodeData>, usize) {
    let end = collect_quote_raw_region(lines, start);
    let region = &lines[start..end];
    let mut dequoted = Vec::with_capacity(region.len());
    for line in region {
        if line.trim().is_empty() {
            dequoted.push(String::new());
            continue;
        }

        let Some(content) = strip_one_quote_level(line) else {
            let raw_node = raw_block(region.join("\n"));
            return (vec![raw_node], end);
        };
        dequoted.push(content);
    }

    let Some(result) = build_native_quote_block(&dequoted) else {
        let raw_node = raw_block(region.join("\n"));
        return (vec![raw_node], end);
    };

    (result, end)
}

fn build_native_quote_block(lines: &[String]) -> Option<Vec<NodeData>> {
    if let Some(header_index) = lines.iter().position(|line| !line.trim().is_empty())
        && let Some((variant, title)) = CalloutKind::parse_header_line(&lines[header_index])
    {
        return build_native_callout_block(&lines[header_index + 1..], variant, title);
    }

    let mut title_markdown = String::new();
    let mut child_nodes: Vec<NodeData> = Vec::new();
    let mut index = 0usize;
    let mut pending_blank_lines = 0usize;
    let mut saw_child = false;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            index += 1;
            continue;
        }

        if is_table_candidate_line(line) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let table_end = collect_table_candidate_region(lines, index);
            let table_region = &lines[index..table_end];
            if let Some(table) = parse_table_region(table_region) {
                child_nodes.push(NodeData::table(table));
            } else {
                child_nodes.push(raw_block(table_region.join("\n")));
            }
            saw_child = true;
            pending_blank_lines = 0;
            index = table_end;
            continue;
        }

        if is_footnote_definition_start(line) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let footnote_end = collect_footnote_definition_region(lines, index);
            if let Some(mut footnote_nodes) =
                build_native_footnote_definition_node(&lines[index..footnote_end])
            {
                child_nodes.append(&mut footnote_nodes);
                saw_child = true;
                pending_blank_lines = 0;
                index = footnote_end;
                continue;
            }
        }

        if let Some((comment, consumed)) = collect_comment_block(lines, index) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(comment);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if is_block_html_start(line) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let html_end = collect_block_html_region(lines, index);
            child_nodes.push(html_or_raw_block(lines[index..html_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = html_end;
            continue;
        }

        if is_display_math_start(line) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let math_end = collect_display_math_region(lines, index);
            child_nodes.push(math_or_raw_block(lines[index..math_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = math_end;
            continue;
        }

        if let Some(unsupported_end) = collect_unsupported_quote_region(lines, index) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(raw_block(lines[index..unsupported_end].join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            index = unsupported_end;
            continue;
        }

        if is_quote_start(line) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let (mut nested_quote_nodes, consumed) = collect_quote_block(lines, index);
            if !nested_quote_nodes.is_empty()
                && nested_quote_nodes[0].kind == BlockKind::RawMarkdown
            {
                return None;
            }
            child_nodes.append(&mut nested_quote_nodes);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if parse_list_marker(line).is_some() {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            let (mut list_nodes, consumed) = collect_list_blocks(lines, index);
            if list_nodes
                .iter()
                .any(|node| node.kind == BlockKind::RawMarkdown)
            {
                return None;
            }
            child_nodes.append(&mut list_nodes);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if parse_opening_fence(line).is_some()
            && let Some((code_block, consumed)) = collect_fenced_code_block(lines, index)
        {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(code_block);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        if starts_with_standalone_image_child_paragraph(&lines[index..]) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(standalone_image_block(line.to_string()));
            saw_child = true;
            pending_blank_lines = 0;
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some()
            && let Some((code_block, consumed)) = collect_indented_code_block(lines, index)
        {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(code_block);
            saw_child = true;
            pending_blank_lines = 0;
            index = consumed;
            continue;
        }

        let mut paragraph_lines = vec![line.clone()];
        index += 1;
        while index < lines.len() {
            let next = &lines[index];
            if next.trim().is_empty()
                || is_quote_start(next)
                || parse_list_marker(next).is_some()
                || parse_opening_fence(next).is_some()
                || strip_indented_code_prefix(next).is_some()
                || quote_content_starts_unsupported(lines, index)
            {
                break;
            }

            paragraph_lines.push(next.clone());
            index += 1;
        }

        if is_standalone_image_paragraph(&paragraph_lines) {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(standalone_image_block(paragraph_lines.join("\n")));
            saw_child = true;
            pending_blank_lines = 0;
            continue;
        }

        if saw_child {
            if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
                append_separator_children(&mut child_nodes, pending_blank_lines);
            }
            child_nodes.push(native_block(
                BlockKind::Paragraph,
                &paragraph_lines.join("\n"),
            ));
            pending_blank_lines = 0;
            continue;
        }

        if !title_markdown.is_empty() {
            title_markdown.push_str(if pending_blank_lines > 0 {
                "\n\n"
            } else {
                "\n"
            });
        }
        title_markdown.push_str(&paragraph_lines.join("\n"));
        pending_blank_lines = 0;
    }

    if pending_blank_lines > 0 && (!title_markdown.is_empty() || !child_nodes.is_empty()) {
        append_separator_children(&mut child_nodes, pending_blank_lines);
    }

    let mut block = native_block(BlockKind::Blockquote, &title_markdown);
    attach_child_nodes(&mut block, &mut child_nodes);

    let mut result = vec![block];
    result.extend(child_nodes);
    Some(result)
}

fn build_native_callout_block(
    lines: &[String],
    variant: CalloutKind,
    title: String,
) -> Option<Vec<NodeData>> {
    let mut child_nodes = Vec::new();
    let mut index = 0usize;
    let mut pending_blank_lines = 0usize;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            pending_blank_lines += 1;
            index += 1;
            continue;
        }

        if pending_blank_lines > 0 {
            append_separator_children(&mut child_nodes, pending_blank_lines);
            pending_blank_lines = 0;
        }

        if is_table_candidate_line(line) {
            let table_end = collect_table_candidate_region(lines, index);
            let table_region = &lines[index..table_end];
            if let Some(table) = parse_table_region(table_region) {
                child_nodes.push(NodeData::table(table));
            } else {
                child_nodes.push(raw_block(table_region.join("\n")));
            }
            index = table_end;
            continue;
        }

        if is_footnote_definition_start(line) {
            let footnote_end = collect_footnote_definition_region(lines, index);
            if let Some(mut footnote_nodes) =
                build_native_footnote_definition_node(&lines[index..footnote_end])
            {
                child_nodes.append(&mut footnote_nodes);
                index = footnote_end;
                continue;
            }
        }

        if let Some((comment, consumed)) = collect_comment_block(lines, index) {
            child_nodes.push(comment);
            index = consumed;
            continue;
        }

        if is_block_html_start(line) {
            let html_end = collect_block_html_region(lines, index);
            child_nodes.push(html_or_raw_block(lines[index..html_end].join("\n")));
            index = html_end;
            continue;
        }

        if is_display_math_start(line) {
            let math_end = collect_display_math_region(lines, index);
            child_nodes.push(math_or_raw_block(lines[index..math_end].join("\n")));
            index = math_end;
            continue;
        }

        if let Some(unsupported_end) = collect_unsupported_quote_region(lines, index) {
            child_nodes.push(raw_block(lines[index..unsupported_end].join("\n")));
            index = unsupported_end;
            continue;
        }

        if is_quote_start(line) {
            let (mut nested_quote_nodes, consumed) = collect_quote_block(lines, index);
            if !nested_quote_nodes.is_empty()
                && nested_quote_nodes[0].kind == BlockKind::RawMarkdown
            {
                return None;
            }
            child_nodes.append(&mut nested_quote_nodes);
            index = consumed;
            continue;
        }

        if parse_list_marker(line).is_some() {
            let (mut list_nodes, consumed) = collect_list_blocks(lines, index);
            if list_nodes
                .iter()
                .any(|node| node.kind == BlockKind::RawMarkdown)
            {
                return None;
            }
            child_nodes.append(&mut list_nodes);
            index = consumed;
            continue;
        }

        if parse_opening_fence(line).is_some()
            && let Some((code_block, consumed)) = collect_fenced_code_block(lines, index)
        {
            child_nodes.push(code_block);
            index = consumed;
            continue;
        }

        if starts_with_standalone_image_child_paragraph(&lines[index..]) {
            child_nodes.push(standalone_image_block(line.to_string()));
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some()
            && let Some((code_block, consumed)) = collect_indented_code_block(lines, index)
        {
            child_nodes.push(code_block);
            index = consumed;
            continue;
        }

        let mut paragraph_lines = vec![line.clone()];
        index += 1;
        while index < lines.len() {
            let next = &lines[index];
            if next.trim().is_empty()
                || is_quote_start(next)
                || parse_list_marker(next).is_some()
                || parse_opening_fence(next).is_some()
                || strip_indented_code_prefix(next).is_some()
                || quote_content_starts_unsupported(lines, index)
            {
                break;
            }

            paragraph_lines.push(next.clone());
            index += 1;
        }

        child_nodes.push(native_block(
            BlockKind::Paragraph,
            &paragraph_lines.join("\n"),
        ));
    }

    if pending_blank_lines > 0 {
        append_separator_children(&mut child_nodes, pending_blank_lines);
    }

    let mut block = NodeData::new(BlockKind::Callout(variant), RichText::from_markdown(&title));
    attach_child_nodes(&mut block, &mut child_nodes);

    let mut result = vec![block];
    result.extend(child_nodes);
    Some(result)
}

fn collect_list_blocks(lines: &[String], start: usize) -> (Vec<NodeData>, usize) {
    let mut roots = Vec::new();
    let mut index = start;

    while index < lines.len() {
        let Some(marker) = parse_list_marker(&lines[index]) else {
            break;
        };

        let item_end = collect_list_item_region(lines, index, marker.indent_columns);
        let mut block = native_block(marker.kind.clone(), &marker.text);
        let mut body_index = index + 1;
        let mut pending_blank_lines = 0usize;
        let mut fallback_raw = false;
        let mut saw_child = false;
        let mut item_children: Vec<NodeData> = Vec::new();

        while body_index < item_end {
            let line = &lines[body_index];
            if line.trim().is_empty() {
                pending_blank_lines += 1;
                body_index += 1;
                continue;
            }

            let (line_indent_columns, _) = leading_indent_columns_and_bytes(line);
            if line_indent_columns > marker.indent_columns {
                let anchor_dedented =
                    dedent_lines(&lines[body_index..item_end], line_indent_columns);

                if parse_list_marker(&anchor_dedented[0]).is_some() {
                    let (mut children, consumed) = collect_list_blocks(&anchor_dedented, 0);
                    attach_child_nodes(&mut block, &mut children);
                    item_children.append(&mut children);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_quote_start(&anchor_dedented[0]) {
                    let (mut quote_nodes, consumed) = collect_quote_block(&anchor_dedented, 0);
                    if !quote_nodes.is_empty() && quote_nodes[0].kind == BlockKind::RawMarkdown {
                        fallback_raw = true;
                        break;
                    }

                    attach_child_nodes(&mut block, &mut quote_nodes);
                    item_children.append(&mut quote_nodes);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if parse_opening_fence(&anchor_dedented[0]).is_some()
                    && let Some((mut code_block, consumed)) =
                        collect_fenced_code_block(&anchor_dedented, 0)
                {
                    attach_child_node(&mut block, &mut code_block);
                    item_children.push(code_block);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_root_table_candidate_line(&anchor_dedented[0]) {
                    let table_end = collect_root_table_candidate_region(&anchor_dedented, 0);
                    let table_region = &anchor_dedented[..table_end];
                    let mut child = if let Some(table) = parse_root_table_region(table_region) {
                        NodeData::table(table)
                    } else {
                        raw_block(table_region.join("\n"))
                    };
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += table_end;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if starts_with_standalone_image_child_paragraph(&anchor_dedented) {
                    let mut child = standalone_image_block(anchor_dedented[0].clone());
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += 1;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if line_indent_columns >= marker.content_indent_columns {
                    let content_dedented =
                        dedent_lines(&lines[body_index..item_end], marker.content_indent_columns);
                    if strip_indented_code_prefix(&content_dedented[0]).is_some() {
                        let Some((mut code_block, consumed)) =
                            collect_indented_code_block(&content_dedented, 0)
                        else {
                            unreachable!("indented code prefix disappeared after child detection");
                        };

                        attach_child_node(&mut block, &mut code_block);
                        item_children.push(code_block);
                        body_index += consumed;
                        pending_blank_lines = 0;
                        saw_child = true;
                        continue;
                    }
                }

                if is_reference_definition_start(&anchor_dedented[0]) {
                    let consumed = collect_reference_definition_region(&anchor_dedented, 0);
                    let mut child = raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if let Some((mut comment, consumed)) = collect_comment_block(&anchor_dedented, 0) {
                    attach_child_node(&mut block, &mut comment);
                    item_children.push(comment);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_block_html_start(&anchor_dedented[0]) {
                    let consumed = collect_block_html_region(&anchor_dedented, 0);
                    let mut child = html_or_raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_footnote_definition_start(&anchor_dedented[0]) {
                    let consumed = collect_footnote_definition_region(&anchor_dedented, 0);
                    let mut child = raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                if is_display_math_start(&anchor_dedented[0]) {
                    let consumed = collect_display_math_region(&anchor_dedented, 0);
                    let mut child = math_or_raw_block(anchor_dedented[..consumed].join("\n"));
                    attach_child_node(&mut block, &mut child);
                    item_children.push(child);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }

                let should_promote_plain_child = pending_blank_lines > 0
                    || saw_child
                    || block.title.visible_text().is_empty()
                    || parse_standalone_image(&block.title.serialize_markdown()).is_some();
                if should_promote_plain_child {
                    let (mut paragraph, consumed) = collect_paragraph_block(&anchor_dedented, 0);
                    attach_child_node(&mut block, &mut paragraph);
                    item_children.push(paragraph);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }
            }

            if line_indent_columns >= marker.content_indent_columns {
                let content_dedented =
                    dedent_lines(&lines[body_index..item_end], marker.content_indent_columns);
                if strip_indented_code_prefix(&content_dedented[0]).is_some() {
                    let Some((mut code_block, consumed)) =
                        collect_indented_code_block(&content_dedented, 0)
                    else {
                        unreachable!("indented code prefix disappeared after detection");
                    };

                    attach_child_node(&mut block, &mut code_block);
                    item_children.push(code_block);
                    body_index += consumed;
                    pending_blank_lines = 0;
                    saw_child = true;
                    continue;
                }
            }

            let trimmed = line.trim_start_matches([' ', '\t']);
            append_markdown_to_node(
                &mut block,
                if pending_blank_lines > 0 {
                    "\n\n"
                } else {
                    "\n"
                },
                trimmed,
            );
            pending_blank_lines = 0;
            body_index += 1;
        }

        if fallback_raw {
            roots.push(raw_block(lines[index..item_end].join("\n")));
        } else {
            roots.push(block);
            roots.append(&mut item_children);
        }
        index = item_end;
    }

    (roots, index)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a full Markdown document into a flat list of [`NodeData`] nodes.
///
/// The returned nodes have `parent_id` and `child_ids` set up for tree
/// reconstruction. Root nodes have `parent_id == None`.
pub fn parse_document(markdown: &str) -> Vec<NodeData> {
    let lines = markdown
        .split('\n')
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    build_blocks_from_lines_internal(&lines, true)
}

/// Build nodes from pre-split Markdown lines.
///
/// Equivalent to the (now pure) version of the legacy
/// `Editor::build_blocks_from_lines`.
pub fn build_blocks_from_lines(lines: &[String]) -> Vec<NodeData> {
    build_blocks_from_lines_internal(lines, true)
}

/// Internal dispatch: walk every line and emit native nodes or raw fallbacks.
fn build_blocks_from_lines_internal(
    lines: &[String],
    allow_root_footnote_definitions: bool,
) -> Vec<NodeData> {
    let mut roots = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            let blank_start = index;
            while index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
            }

            let blank_run_len = index - blank_start;
            let previous_root_is_list_item = roots
                .last()
                .map(|node: &NodeData| node.kind.is_list_item())
                .unwrap_or(false);
            let next_root_is_list_item = lines
                .get(index)
                .is_some_and(|line| parse_list_marker(line).is_some());
            let preserved_empty_blocks = if roots.is_empty() {
                blank_run_len
            } else if previous_root_is_list_item && next_root_is_list_item {
                blank_run_len
            } else {
                blank_run_len.saturating_sub(1)
            };

            for _ in 0..preserved_empty_blocks {
                roots.push(native_block(BlockKind::Paragraph, ""));
            }
            continue;
        }

        if parse_opening_fence(line).is_some() {
            let Some((block, next_index)) = collect_fenced_code_block(lines, index) else {
                let paragraph = collect_paragraph_block(lines, index);
                roots.push(paragraph.0);
                index = paragraph.1;
                continue;
            };

            roots.push(block);
            index = next_index;
            continue;
        }

        if let Some((block, end)) = collect_comment_block(lines, index) {
            roots.push(block);
            index = end;
            continue;
        }

        if is_block_html_start(line) {
            let end = collect_block_html_region(lines, index);
            roots.push(html_or_raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        if is_footnote_definition_start(line) {
            let end = collect_footnote_definition_region(lines, index);
            if allow_root_footnote_definitions {
                if let Some(mut nodes) = build_native_footnote_definition_node(&lines[index..end]) {
                    roots.append(&mut nodes);
                } else {
                    roots.push(raw_block(lines[index..end].join("\n")));
                }
            } else {
                roots.push(raw_block(lines[index..end].join("\n")));
            }
            index = end;
            continue;
        }

        if is_reference_definition_start(line) {
            let end = collect_reference_definition_region(lines, index);
            roots.push(raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        if let Some(level) = lines
            .get(index + 1)
            .and_then(|next| BlockKind::parse_setext_underline(next))
        {
            roots.push(native_block(BlockKind::Heading { level }, line.trim_end()));
            index += 2;
            continue;
        }

        if parse_standalone_image(line).is_some() {
            roots.push(standalone_image_block(line.to_string()));
            index += 1;
            continue;
        }

        if strip_indented_code_prefix(line).is_some() {
            let Some((block, next_index)) = collect_indented_code_block(lines, index) else {
                unreachable!("indented code prefix disappeared after detection");
            };

            roots.push(block);
            index = next_index;
            continue;
        }

        if parse_list_marker(line).is_some() {
            let (mut blocks, next_index) = collect_list_blocks(lines, index);
            roots.append(&mut blocks);
            index = next_index;
            continue;
        }

        if is_quote_start(line) {
            let (mut blocks, next_index) = collect_quote_block(lines, index);
            roots.append(&mut blocks);
            index = next_index;
            continue;
        }

        if let Some((level, content)) = BlockKind::parse_atx_heading_line(line) {
            roots.push(native_block(BlockKind::Heading { level }, &content));
            index += 1;
            continue;
        }

        if BlockKind::parse_thematic_break_line(line) {
            roots.push(NodeData::with_plain_text(
                BlockKind::ThematicBreak,
                line.to_string(),
            ));
            index += 1;
            continue;
        }

        if is_root_table_candidate_line(line) {
            let end = collect_root_table_candidate_region(lines, index);
            let region = &lines[index..end];
            if let Some(table) = parse_root_table_region(region) {
                roots.push(NodeData::table(table));
            } else {
                roots.extend(
                    region
                        .iter()
                        .cloned()
                        .map(|line| plain_text_paragraph_block(line)),
                );
            }
            index = end;
            continue;
        }

        if let Some(end) = collect_pipeless_table_region(lines, index)
            && let Some(table) = parse_root_table_region(&lines[index..end])
        {
            roots.push(NodeData::table(table));
            index = end;
            continue;
        }

        if is_display_math_start(line) {
            let end = collect_display_math_region(lines, index);
            roots.push(math_or_raw_block(lines[index..end].join("\n")));
            index = end;
            continue;
        }

        let paragraph = collect_paragraph_block(lines, index);
        roots.push(paragraph.0);
        index = paragraph.1;
    }

    roots
}
