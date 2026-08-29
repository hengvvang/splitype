//! Source-offset mapping generators for headings, lists, footnotes, raw, and code blocks.

use gpui::*;

use crate::editor::engine::controller::*;
use editor_wysiwyg::markdown::parse::BlockKind;

impl Editor {
    pub(crate) fn push_inline_block_mapping(
        &self,
        block: &Entity<Block>,
        content_markdown: String,
        first_prefix: String,
        continuation_prefix: String,
        quote_depth: usize,
        absolute_start: usize,
        mappings: &mut Vec<SourceTargetMapping>,
    ) -> usize {
        let (full_text, content_to_source, source_to_content) =
            Self::build_prefixed_content_mapping(
                &content_markdown,
                &first_prefix,
                &continuation_prefix,
            );
        let (full_text, content_to_source, source_to_content) =
            Self::wrap_source_mapping_with_quotes(
                full_text,
                content_to_source,
                source_to_content,
                quote_depth,
            );
        mappings.push(SourceTargetMapping {
            entity: block.clone(),
            full_source_range: absolute_start..absolute_start + full_text.len(),
            content_to_source,
            source_to_content,
        });
        full_text.len()
    }

    /// Pushes the single source mapping for a footnote definition block whose
    /// display text is `{id}: {content}` and whose source is
    /// `[^{id}]: {content}` (continuation lines indented four spaces). The id
    /// maps into the `[^…]` label, the content maps after `]: `.
    pub(crate) fn push_footnote_definition_full_mapping(
        block: &Entity<Block>,
        footnote_id: &str,
        first_line: &str,
        quote_depth: usize,
        absolute_start: usize,
        mappings: &mut Vec<SourceTargetMapping>,
    ) -> usize {
        let id_len = footnote_id.len();
        let content = format!("{footnote_id}: {first_line}");
        let mut full_text = format!("[^{footnote_id}]");
        let mut content_to_source = vec![0; content.len() + 1];
        let mut source_to_content = vec![0; full_text.len() + 1];

        // `[^` maps to the display start; the id maps inside the label.
        let id_start = 2usize;
        for offset in 0..=id_len {
            content_to_source[offset] = id_start + offset;
        }
        for source_offset in 0..=full_text.len() {
            source_to_content[source_offset] = if source_offset <= id_start {
                0
            } else {
                (source_offset - id_start).min(id_len)
            };
        }

        // `: ` follows the label; `:` and ` ` in the display map past it.
        let content_prefix_end = id_len + 2;
        full_text.push_str(": ");
        source_to_content.resize(full_text.len() + 1, id_len);
        for index in id_start + id_len..=id_start + id_len + 1 {
            source_to_content[index] = id_len;
        }
        source_to_content[id_start + id_len + 2] = id_len + 1;
        source_to_content[id_start + id_len + 3] = id_len + 2;
        content_to_source[id_len] = id_start + id_len;
        content_to_source[id_len + 1] = full_text.len();

        // The content part: display content[id_len+2..] ↔ source after `]: `,
        // with each continuation line indented four spaces.
        let mut content_offset = content_prefix_end;
        while content_offset < content.len() {
            content_to_source[content_offset] = full_text.len();
            let ch = content[content_offset..]
                .chars()
                .next()
                .expect("content offset should stay on char boundaries");
            let start = full_text.len();
            full_text.push(ch);
            source_to_content.resize(full_text.len() + 1, content_offset);
            for index in start..=full_text.len() {
                source_to_content[index] = content_offset;
            }
            content_offset += ch.len_utf8();
            if ch == '\n' {
                let prefix_start = full_text.len();
                full_text.push_str("    ");
                source_to_content.resize(full_text.len() + 1, content_offset);
                for index in prefix_start..=full_text.len() {
                    source_to_content[index] = content_offset;
                }
            }
        }
        content_to_source[content.len()] = full_text.len();
        source_to_content[full_text.len()] = content.len();

        let (full_text, content_to_source, source_to_content) =
            Self::wrap_source_mapping_with_quotes(
                full_text,
                content_to_source,
                source_to_content,
                quote_depth,
            );
        mappings.push(SourceTargetMapping {
            entity: block.clone(),
            full_source_range: absolute_start..absolute_start + full_text.len(),
            content_to_source,
            source_to_content,
        });
        full_text.len()
    }

    pub(crate) fn push_raw_block_mapping(
        &self,
        block: &Entity<Block>,
        quote_depth: usize,
        absolute_start: usize,
        mappings: &mut Vec<SourceTargetMapping>,
        cx: &App,
    ) -> usize {
        let (content, indentation) = {
            let block_ref = block.read(cx);
            (
                block_ref.display_text().to_string(),
                if block_ref.render_depth == 0 {
                    String::new()
                } else {
                    "  ".repeat(block_ref.render_depth)
                },
            )
        };
        let (full_text, content_to_source, source_to_content) =
            Self::build_prefixed_content_mapping(&content, &indentation, &indentation);
        let (full_text, content_to_source, source_to_content) =
            Self::wrap_source_mapping_with_quotes(
                full_text,
                content_to_source,
                source_to_content,
                quote_depth,
            );
        mappings.push(SourceTargetMapping {
            entity: block.clone(),
            full_source_range: absolute_start..absolute_start + full_text.len(),
            content_to_source,
            source_to_content,
        });
        full_text.len()
    }

    /// Pushes the source mapping for a fenced block (math or Mermaid). The
    /// stored block text is the bare body, while the serialized document
    /// wraps it in `$$` / fence markers, so the mapping composes body offsets
    /// through the rebuilt fence lines to the full source text.
    pub(crate) fn push_fenced_block_mapping(
        &self,
        block: &Entity<Block>,
        quote_depth: usize,
        absolute_start: usize,
        mappings: &mut Vec<SourceTargetMapping>,
        cx: &App,
    ) -> usize {
        let (body_len, indentation, serialized, body_range) = {
            let block_ref = block.read(cx);
            let body = block_ref.display_text().to_string();
            let indentation = if block_ref.render_depth == 0 {
                String::new()
            } else {
                "  ".repeat(block_ref.render_depth)
            };
            let (serialized, body_range) = match block_ref.kind() {
                BlockKind::MathBlock => {
                    editor_wysiwyg::markdown::block::math::serialize_display_math_source(&body)
                }
                BlockKind::MermaidBlock => {
                    editor_wysiwyg::markdown::block::mermaid::serialize_mermaid_source(&body)
                }
                _ => (body, 0..0),
            };
            (body_range.len(), indentation, serialized, body_range)
        };

        let (full_text, serialized_to_full, full_to_serialized) =
            Self::build_prefixed_content_mapping(&serialized, &indentation, &indentation);
        // Map the body region (inside the fences) onto the full source text;
        // fence characters snap to the nearest body boundary.
        let content_to_source: Vec<usize> = (0..=body_len)
            .map(|offset| serialized_to_full[body_range.start + offset])
            .collect();
        let source_to_content: Vec<usize> = full_to_serialized
            .iter()
            .map(|offset| offset.saturating_sub(body_range.start).min(body_len))
            .collect();
        let (full_text, content_to_source, source_to_content) =
            Self::wrap_source_mapping_with_quotes(
                full_text,
                content_to_source,
                source_to_content,
                quote_depth,
            );
        mappings.push(SourceTargetMapping {
            entity: block.clone(),
            full_source_range: absolute_start..absolute_start + full_text.len(),
            content_to_source,
            source_to_content,
        });
        full_text.len()
    }

    pub(crate) fn push_code_block_mapping(
        &self,
        block: &Entity<Block>,
        quote_depth: usize,
        absolute_start: usize,
        mappings: &mut Vec<SourceTargetMapping>,
        cx: &App,
    ) -> usize {
        let (language, indentation, content) = {
            let block_ref = block.read(cx);
            (
                match block_ref.kind() {
                    BlockKind::CodeBlock { language } => language.clone(),
                    _ => None,
                },
                "  ".repeat(block_ref.render_depth),
                block_ref.display_text().to_string(),
            )
        };

        let (full_text, content_to_source, source_to_content) =
            Self::build_code_block_content_mapping(&content, &indentation, language.as_deref());
        let (full_text, content_to_source, source_to_content) =
            Self::wrap_source_mapping_with_quotes(
                full_text,
                content_to_source,
                source_to_content,
                quote_depth,
            );
        mappings.push(SourceTargetMapping {
            entity: block.clone(),
            full_source_range: absolute_start..absolute_start + full_text.len(),
            content_to_source,
            source_to_content,
        });
        full_text.len()
    }

    pub(crate) fn wrap_source_mapping_with_quotes(
        mut full_text: String,
        mut content_to_source: Vec<usize>,
        mut source_to_content: Vec<usize>,
        quote_depth: usize,
    ) -> (String, Vec<usize>, Vec<usize>) {
        for _ in 0..quote_depth {
            let (wrapped_text, inner_to_wrapped, wrapped_to_inner) =
                Self::build_prefixed_content_mapping(&full_text, "> ", "> ");
            let max_inner_to_wrapped = inner_to_wrapped.len().saturating_sub(1);
            let max_source_to_content = source_to_content.len().saturating_sub(1);

            let wrapped_content_to_source = content_to_source
                .iter()
                .map(|offset| inner_to_wrapped[(*offset).min(max_inner_to_wrapped)])
                .collect::<Vec<_>>();
            let wrapped_source_to_content = wrapped_to_inner
                .iter()
                .map(|offset| source_to_content[(*offset).min(max_source_to_content)])
                .collect::<Vec<_>>();

            full_text = wrapped_text;
            content_to_source = wrapped_content_to_source;
            source_to_content = wrapped_source_to_content;
        }

        (full_text, content_to_source, source_to_content)
    }
}
