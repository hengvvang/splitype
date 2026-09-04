//! WysiwygDocumentController — sync_commit handlers.

use std::ops::Range;
use std::sync::Arc;

use editor_contracts::{CursorHint, EditTransaction};
use gpui::{App, Context};

use super::WysiwygDocumentController;
impl WysiwygDocumentController {
    pub fn sync_document(
        &mut self,
        document: &editor_contracts::DocumentSnapshot,
        cx: &mut Context<Self>,
    ) {
        let next_base_dir = document
            .base_dir
            .as_deref()
            .map(std::path::Path::to_path_buf);
        let base_dir_changed = self.references.base_dir != next_base_dir;
        self.references.base_dir = next_base_dir;

        if self.synced_revision == Some(document.revision) && self.document.is_some() {
            if base_dir_changed {
                self.sync_reference_context(None, cx);
            }
            return;
        }
        if self.pending_edit {
            self.synced_revision = Some(document.revision);
            self.pending_edit = false;
            if base_dir_changed {
                self.sync_reference_context(None, cx);
            }
            return;
        }
        // The buffer's shared block projection always matches the
        // revision (it is maintained synchronously with each edit), so the
        // pane patches its entities against it without ever parsing.
        self.patch_from_blocks(
            document.blocks.as_slice(),
            document.revision,
            document.text.len(),
            cx,
        );
        if let Some(hint) = document.restore_cursor {
            self.restore_cursor_hint(hint, cx);
        }
    }

    pub fn document_text(&self, cx: &App) -> Option<String> {
        self.document.as_ref().map(|d| d.serialize_markdown(cx))
    }

    /// Current caret position of the active block as a document-level
    /// cursor hint (1-based line/column in the serialized Markdown).
    pub fn cursor_hint(&self, cx: &App) -> CursorHint {
        let Some(doc) = &self.document else {
            return CursorHint::new(1, 1);
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        self.cursor_hint_from(&lines, &mappings, cx)
    }

    /// Cursor hint from an already-serialized document, so commits pay for
    /// one serialization instead of two.
    fn cursor_hint_from(
        &self,
        lines: &[String],
        mappings: &[crate::model::serialize::BlockLineMapping],
        cx: &App,
    ) -> CursorHint {
        let Some(active) = &self.active_entity else {
            return CursorHint::new(1, 1);
        };
        if lines.is_empty() || mappings.is_empty() {
            return CursorHint::new(1, 1);
        }
        let Some(mapping) = mappings.iter().find(|m| m.entity_id == active.entity_id()) else {
            return CursorHint::new(1, 1);
        };
        if mapping.own_start_line >= mapping.own_end_line || mapping.own_start_line >= lines.len() {
            return CursorHint::new((mapping.own_start_line + 1).min(lines.len()) as u32, 1);
        }

        let block = active.read(cx);
        let caret = block.cursor_offset();
        let intra = block.display_range_to_source_range(caret..caret);
        let markdown_text = block.data.text.serialize_markdown();
        let intra_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            intra.start.min(markdown_text.len()),
        );
        let before = &markdown_text[..intra_offset];
        let line_in_block = before.matches('\n').count();
        let col_in_block = before.rsplit('\n').next().unwrap_or("").chars().count();

        let num_own_lines = mapping.own_end_line - mapping.own_start_line;
        let line_offset_in_own = if block.kind().is_code_block() {
            (line_in_block + 1).min(num_own_lines.saturating_sub(1))
        } else {
            line_in_block.min(num_own_lines.saturating_sub(1))
        };
        let doc_line = mapping.own_start_line + line_offset_in_own;
        if doc_line >= lines.len() {
            return CursorHint::new(lines.len() as u32, 1);
        }

        let line_str = &lines[doc_line];
        let text_line = markdown_text.split('\n').nth(line_in_block).unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_chars = prefix_chars + col_in_block;

        CursorHint::new((doc_line + 1) as u32, (col_chars + 1) as u32)
    }

    /// Moves the active caret to the document position described by a
    /// cursor hint (used to apply `restore_cursor` after undo/redo).
    pub fn restore_cursor_hint(&mut self, hint: CursorHint, cx: &mut Context<Self>) {
        let Some(doc) = &self.document else {
            return;
        };
        let (lines, mappings) = doc.serialize_markdown_lines_with_mapping(cx);
        if mappings.is_empty() || lines.is_empty() {
            return;
        }

        let target_line = hint.line.saturating_sub(1) as usize;
        let target_col = hint.column.saturating_sub(1) as usize;

        let mut best = &mappings[0];
        for m in &mappings {
            if target_line >= m.own_start_line {
                best = m;
            }
            if target_line >= m.own_start_line && target_line < m.own_end_line {
                best = m;
                break;
            }
        }

        let Some(target_entity) = doc.block_entity_by_id(best.entity_id) else {
            return;
        };

        let block_ref = target_entity.read(cx);
        let markdown_text = block_ref.data.text.serialize_markdown();
        let num_own_lines = best.own_end_line.saturating_sub(best.own_start_line);
        let own_line_offset = target_line
            .saturating_sub(best.own_start_line)
            .min(num_own_lines.saturating_sub(1));
        let doc_line = (best.own_start_line + own_line_offset).min(lines.len().saturating_sub(1));
        let line_str = &lines[doc_line];

        let text_line_idx = if block_ref.kind().is_code_block() {
            own_line_offset.saturating_sub(1)
        } else {
            own_line_offset
        };

        let text_lines: Vec<&str> = markdown_text.split('\n').collect();
        let text_line = text_lines.get(text_line_idx).copied().unwrap_or("");
        let prefix_bytes = if line_str.ends_with(text_line) {
            line_str.len() - text_line.len()
        } else {
            markdown_parser::inline::serialize::clamp_to_char_boundary(
                line_str,
                line_str.len().saturating_sub(text_line.len()),
            )
        };
        let prefix_chars = line_str[..prefix_bytes].chars().count();
        let col_in_text_line = target_col
            .saturating_sub(prefix_chars)
            .min(text_line.chars().count());

        let mut source_offset = 0usize;
        for &prev in &text_lines[..text_line_idx.min(text_lines.len())] {
            source_offset += prev.len() + 1;
        }
        for ch in text_line.chars().take(col_in_text_line) {
            source_offset += ch.len_utf8();
        }
        let source_offset = markdown_parser::inline::serialize::clamp_to_char_boundary(
            &markdown_text,
            source_offset.min(markdown_text.len()),
        );

        self.active_entity = Some(target_entity.clone());
        target_entity.update(cx, |b, cx| {
            let display = b.source_range_to_display_range(source_offset..source_offset);
            let caret = display.start.min(b.display_len());
            b.selected_range = caret..caret;
            b.selection_reversed = false;
            b.marked_range = None;
            b.start_cursor_blink(cx);
            cx.notify();
        });
        cx.notify();
    }

    /// Commits the current document as one edit transaction.
    ///
    /// `merge` marks continuation of the previous undo transaction (typing
    /// runs). The caret hints anchor the buffer-level undo/redo restore.
    pub fn commit_document_edit(&mut self, merge: bool, cx: &mut App) {
        if let Some(edit) = self.take_edit_transaction(merge, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Commits a block-level text change, deriving the typing-run merge
    /// flag from the serialized text. One serialization serves the merge
    /// check, the caret hint, and the transaction itself.
    pub fn commit_typing_edit(&mut self, cx: &mut App) {
        let Some((lines, mappings)) = self
            .document
            .as_ref()
            .map(|doc| doc.serialize_markdown_lines_with_mapping(cx))
        else {
            return;
        };
        if let Some(edit) = self.build_transaction(None, &lines, &mappings, cx) {
            if let Some(host) = self.host.clone() {
                host.commit_edit(edit, cx);
            }
        }
    }

    /// Builds the edit transaction for the current document state and
    /// updates the typing-run bookkeeping. Used both by direct commits and
    /// by contract methods that hand the transaction to the editor.
    pub fn take_edit_transaction(&mut self, merge: bool, cx: &App) -> Option<EditTransaction> {
        let (lines, mappings) = self
            .document
            .as_ref()?
            .serialize_markdown_lines_with_mapping(cx);
        self.build_transaction(Some(merge), &lines, &mappings, cx)
    }

    /// Assembles the transaction from one already-performed serialization:
    /// the caret hint reuses `lines`/`mappings` instead of re-serializing,
    /// and the commit is diffed against the previously committed
    /// serialization — a line-level affix narrows the changed region, then
    /// a byte-level affix snaps its edges — so the transaction carries only
    /// the changed bytes: the buffer's rope rebuild, syntax invalidation,
    /// and undo payload are all O(edit). `merge` of `None` means "derive
    /// from the typing-run bookkeeping".
    fn build_transaction(
        &mut self,
        merge: Option<bool>,
        lines: &[String],
        mappings: &[crate::model::serialize::BlockLineMapping],
        cx: &App,
    ) -> Option<EditTransaction> {
        let new = Serialization::from_lines(lines);
        let (range, inserted) = match &self.last_committed {
            Some(old) => diff_edit(old, &new),
            // First commit after a patch sync: replace the whole document;
            // the buffer compresses it down to the true diff.
            None => (0..self.last_synced_len, new.text.clone()),
        };
        if range.is_empty() && inserted.is_empty() {
            self.pending_edit = false;
            return None;
        }

        let cursor_after = self.cursor_hint_from(lines, mappings, cx);
        let merge = match merge {
            Some(merge) => merge,
            None => self.is_typing_continuation(cx, &range, &inserted),
        };
        let cursor_before = if merge {
            self.typing_run_start_hint.unwrap_or(cursor_after)
        } else {
            self.last_cursor_hint.unwrap_or(cursor_after)
        };

        if merge {
            if self.typing_run_start_hint.is_none() {
                self.typing_run_start_hint = self.last_cursor_hint;
            }
            self.last_typing_insert_at = self
                .single_char_insert_at(&range, &inserted)
                .map(|at| range.start + at);
        } else {
            self.typing_run_start_hint = self.last_cursor_hint;
            self.last_typing_insert_at = None;
        }

        self.last_cursor_hint = Some(cursor_after);
        self.last_committed = Some(new);
        Some(EditTransaction::new(
            range,
            Arc::from(inserted),
            merge,
            cursor_before,
            cursor_after,
        ))
    }

    /// Whether the current serialization continues the previous typing run:
    /// a single-character insertion at exactly the previous insertion point,
    /// or an update while an IME composition is active.
    fn is_typing_continuation(&self, cx: &App, range: &Range<usize>, inserted: &str) -> bool {
        if let Some(active) = &self.active_entity {
            if active.read(cx).marked_range.is_some() {
                return true;
            }
        }
        let Some(insert_at) = self.last_typing_insert_at else {
            return false;
        };
        let Some(old) = &self.last_committed else {
            return false;
        };
        let old_changed = &old.text[range.clone()];
        Self::single_char_insert_at_of(old_changed, inserted) == Some(insert_at)
    }

    /// Local insertion point when `new_changed` is a single-character
    /// insertion into `old_changed`.
    fn single_char_insert_at_of(old_changed: &str, new_changed: &str) -> Option<usize> {
        if new_changed.len() != old_changed.len() + 1 {
            return None;
        }
        let old = old_changed.as_bytes();
        let new = new_changed.as_bytes();
        let mut at = 0;
        while at < old.len() && at < new.len() && old[at] == new[at] {
            at += 1;
        }
        if new_changed.is_char_boundary(at) && at < new.len() && old[at..] == new[at + 1..] {
            Some(at)
        } else {
            None
        }
    }

    /// Whether the current commit is a single-character insertion at the
    /// same position as the previous typing commit.
    fn single_char_insert_at(&self, range: &Range<usize>, inserted: &str) -> Option<usize> {
        let old = self.last_committed.as_ref()?;
        let old_changed = &old.text[range.clone()];
        Self::single_char_insert_at_of(old_changed, inserted)
    }
}

/// One commit's serialization of the document: the lines, the joined byte
/// text, and the byte offset of every line start. The text and offsets are
/// built once per commit (alongside the serialization walk) and cached as
/// the diff baseline for the next one.
pub(crate) struct Serialization {
    lines: Vec<Arc<str>>,
    text: String,
    /// `line_starts[i]` is the byte offset of line `i`; the final entry is
    /// the total byte length.
    line_starts: Vec<usize>,
}

impl Serialization {
    fn from_lines(lines: &[String]) -> Self {
        let lines: Vec<Arc<str>> = lines.iter().map(|line| Arc::from(line.as_str())).collect();
        let text = lines.join("\n");
        let mut line_starts = Vec::with_capacity(lines.len() + 1);
        line_starts.push(0);
        for (index, line) in lines.iter().enumerate() {
            let next = line_starts[index] + line.len() + usize::from(index + 1 < lines.len());
            line_starts.push(next);
        }
        Self {
            lines,
            text,
            line_starts,
        }
    }
}

/// Longest common line prefix and suffix of two serializations. Compared
/// by content: unchanged runs cost one string compare per line, so the
/// scan short-circuits around the edit.
fn line_affix(old: &[Arc<str>], new: &[Arc<str>]) -> (usize, usize) {
    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len() - prefix
        && suffix < new.len() - prefix
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    (prefix, suffix)
}

/// The changed bytes of a serialization commit, as (replaced range,
/// inserted text). A line-level affix first bounds the changed middle to
/// the edited lines plus one boundary line on each side; a byte-level
/// affix over that window then snaps the edges exactly, absorbing the
/// separator asymmetries line equality cannot see (a trailing empty line,
/// a line that becomes the document's last, ...). Both scans are O(edited
/// region).
fn diff_edit(old: &Serialization, new: &Serialization) -> (Range<usize>, String) {
    let (prefix, suffix) = line_affix(&old.lines, &new.lines);
    // One boundary line per side, so the byte scan sees the separators
    // around the changed middle.
    let window_start = prefix.saturating_sub(1);
    let window_suffix = suffix.saturating_sub(1);
    let start = old.line_starts[window_start];
    let end = if window_suffix > 0 {
        old.line_starts[old.lines.len() - window_suffix]
    } else {
        old.text.len()
    };
    let new_start = new.line_starts[window_start];
    let new_end = if window_suffix > 0 {
        new.line_starts[new.lines.len() - window_suffix]
    } else {
        new.text.len()
    };
    let (byte_prefix, byte_suffix) =
        markdown_parser::parse::common_affix(&old.text[start..end], &new.text[new_start..new_end]);
    let range = start + byte_prefix..end - byte_suffix;
    let inserted = new.text[new_start + byte_prefix..new_end - byte_suffix].to_string();
    (range, inserted)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies one (range, inserted) replacement to `old`.
    fn apply(old: &str, range: Range<usize>, inserted: &str) -> String {
        format!("{}{}{}", &old[..range.start], inserted, &old[range.end..])
    }

    fn serialization(text: &str) -> Serialization {
        let lines: Vec<String> = text.split('\n').map(str::to_string).collect();
        Serialization::from_lines(&lines)
    }

    /// The golden property of the diff: applying the edit it derives
    /// reproduces the new serialization byte-for-byte.
    #[test]
    fn diff_reproduces_the_new_serialization() {
        let cases: &[(&str, &str)] = &[
            ("", "x"),
            ("a", ""),
            ("a", "ab"),
            ("ab", "a"),
            ("a\nb", "a\nXb"),
            ("a\nb", "a\nb\nc"), // append a line at EOF
            ("a\nb", "a\n"),     // delete the last line
            ("a\nb", "\nb"),     // insert a first line
            ("a\nb\nc", "a\nX\nc"),
            ("a\nb\nc", "a\nc"), // delete a middle line
            ("a\nb", "a\nb"),    // no change
            ("same", "same"),
            ("一\n二", "一\n二\n三"), // multibyte content
            ("x\ny", "x\ny2"),
            ("x\ny", "x2\ny"),
            ("l1\nl2\nl3\nl4", "l1\nl2\nl3b\nl4"),
            ("l1\nl2\nl3\nl4", "l1b\nl2\nl3\nl4"),
            ("l1\nl2\nl3\nl4", "l1\nl2\nl3\nl4b"),
            // Trailing empty line: its bytes are one final newline, which
            // line equality alone cannot see.
            ("a", "a\n"),
            ("a\n", "a"),
            ("a\n\nb", "a\n"),
            ("a\n", "a\n\nb"),
            ("a\n", "a\nb"),
            ("a\nb", "a\n"),
            ("a", "a\n\n"),
            ("a\n\n", "a"),
        ];
        for (old, new) in cases {
            let old = serialization(old);
            let new = serialization(new);
            let (range, inserted) = diff_edit(&old, &new);
            if range.is_empty() && inserted.is_empty() {
                assert_eq!(old.text, new.text, "no-op claimed for a real change");
                continue;
            }
            assert_eq!(
                apply(&old.text, range.clone(), &inserted),
                new.text,
                "diff of {:?} -> {:?} applied as ({range:?}, {inserted:?})",
                old.text,
                new.text
            );
        }
    }

    #[test]
    fn line_affix_finds_shared_edges() {
        let lines = |text: &str| text.lines().map(Arc::from).collect::<Vec<Arc<str>>>();
        assert_eq!(line_affix(&lines("a\nb\nc"), &lines("a\nb\nc")), (3, 0));
        assert_eq!(line_affix(&lines("a\nb\nc"), &lines("a\nX\nc")), (1, 1));
        assert_eq!(line_affix(&lines("a"), &lines("a\nb")), (1, 0));
        assert_eq!(line_affix(&lines("a\nb"), &lines("a")), (1, 0));
        assert_eq!(line_affix(&lines("x"), &lines("y")), (0, 0));
    }

    /// Randomized fuzz of the golden property: any sequence of line edits
    /// (insert/delete/replace, including whole lines and blank lines) must
    /// be captured by the diff such that applying it reproduces the new
    /// serialization.
    #[test]
    fn random_line_edits_always_reproduce_the_serialization() {
        struct Lcg(u64);
        impl Lcg {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }
        }
        let mut rng = Lcg(0x5eed_5eed_5eed_5eed);
        let tokens = ["a", "内容", "", "long line of text", "x\u{4E2E}y", " "];
        for step in 0..300 {
            let mut old_lines: Vec<Arc<str>> = Vec::new();
            for _ in 0..(rng.next() % 12) {
                old_lines.push(Arc::from(tokens[(rng.next() as usize) % tokens.len()]));
            }
            let mut new_lines = old_lines.clone();
            let edits = 1 + (rng.next() % 3) as usize;
            for _ in 0..edits {
                let count = new_lines.len();
                let at = if count == 0 {
                    0
                } else {
                    (rng.next() as usize) % count
                };
                let removed = (rng.next() as usize) % 3;
                let insert: Vec<Arc<str>> = (0..(rng.next() % 3))
                    .map(|_| Arc::from(tokens[(rng.next() as usize) % tokens.len()]))
                    .collect();
                new_lines.splice(at..(at + removed).min(count), insert);
            }
            let old_text = old_lines.join("\n");
            let new_text = new_lines.join("\n");
            let old = Serialization {
                text: old_text.clone(),
                line_starts: starts_of(&old_lines),
                lines: old_lines,
            };
            let new = Serialization {
                text: new_text.clone(),
                line_starts: starts_of(&new_lines),
                lines: new_lines,
            };
            let (range, inserted) = diff_edit(&old, &new);
            if range.is_empty() && inserted.is_empty() {
                assert_eq!(old_text, new_text, "step {step}: no-op for a real change");
                continue;
            }
            assert_eq!(
                apply(&old_text, range.clone(), &inserted),
                new_text,
                "step {step}: {old_text:?} -> {new_text:?} via ({range:?}, {inserted:?})"
            );
        }
    }

    fn starts_of(lines: &[Arc<str>]) -> Vec<usize> {
        let mut starts = Vec::with_capacity(lines.len() + 1);
        starts.push(0);
        for (index, line) in lines.iter().enumerate() {
            starts.push(starts[index] + line.len() + usize::from(index + 1 < lines.len()));
        }
        starts
    }
}
