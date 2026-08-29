//! Clipboard action handlers on a focused block: copy, cut, and paste,
//! including local-image-path routing from the clipboard text.

use gpui::*;

use crate::document::protocol::{BlockEvent, PastedImageSource, UndoCaptureKind};
use workspace::actions::{Copy, Cut, Paste};
use crate::paste_plain::should_split_plain_multiline_paste;
use crate::document::block::Block;
use crate::markdown::inline::text::BlockText;
impl Block {
    pub fn pasted_image_source_from_clipboard(item: &ClipboardItem) -> Option<PastedImageSource> {
        item.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(PastedImageSource::ClipboardImage(image.clone())),
            ClipboardEntry::String(_) | ClipboardEntry::ExternalPaths(_) => None,
        })
    }

    pub fn pasted_image_source_from_text(text: &str) -> Option<PastedImageSource> {
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed.contains('\n') || trimmed.contains('\r') {
            return None;
        }

        Self::pasted_image_path_from_text_item(trimmed).map(PastedImageSource::LocalPath)
    }

    /// Parses a single clipboard text item as a local image path.
    ///
    /// Windows file-copy paste reaches GPUI as a plain drive-letter path; that
    /// must be tested as a path before URL parsing, because `url::Url` treats
    /// the drive letter as a URL scheme.
    pub fn pasted_image_path_from_text_item(text: &str) -> Option<std::path::PathBuf> {
        let unquoted = text
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .unwrap_or(text);
        let direct_path = std::path::PathBuf::from(unquoted);
        let path = if Self::is_supported_local_image_path(&direct_path) {
            direct_path
        } else if let Ok(url) = url::Url::parse(unquoted) {
            if url.scheme() == "file" {
                url.to_file_path().ok()?
            } else {
                return None;
            }
        } else {
            return None;
        };
        if !Self::is_supported_local_image_path(&path) {
            return None;
        }
        Some(path)
    }

    pub fn is_supported_local_image_path(path: &std::path::Path) -> bool {
        if !path.is_file() {
            return false;
        }
        let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
            return false;
        };
        matches!(
            ext.to_ascii_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tif" | "tiff"
        )
    }

    pub fn paste_image_split(&self) -> (BlockText, BlockText) {
        let plain_selected = self.selection_plain_range();
        let (leading, tail) = self.data.text.split_at(plain_selected.start);
        let (_, trailing) = tail.split_at(plain_selected.end.saturating_sub(plain_selected.start));
        (leading, trailing)
    }
    pub fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.display_text()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.display_text()[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    pub fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.kind().is_thematic_break() && !self.edits_verbatim_text() {
            return;
        }

        if let Some(item) = cx.read_from_clipboard() {
            if let Some(source) = Self::pasted_image_source_from_clipboard(&item) {
                let (leading, trailing) = self.paste_image_split();
                cx.emit(BlockEvent::RequestPasteImage {
                    leading,
                    source,
                    trailing,
                });
                return;
            }

            let Some(text) = item.text() else {
                return;
            };
            if let Some(source) = Self::pasted_image_source_from_text(&text) {
                let (leading, trailing) = self.paste_image_split();
                cx.emit(BlockEvent::RequestPasteImage {
                    leading,
                    source,
                    trailing,
                });
                return;
            }

            // Only rendered rich-text blocks apply paste correction. Raw/code
            // contexts preserve bytes, and table cells flatten newlines so the
            // surrounding table structure is not accidentally split.
            if self.editor_selection_range.is_some() {
                cx.emit(BlockEvent::RequestReplaceCrossBlockSelection {
                    text,
                    selected_range_relative: None,
                    mark_inserted_text: false,
                    undo_kind: UndoCaptureKind::NonCoalescible,
                });
                return;
            }

            if self.is_table_cell() {
                let flattened = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
                self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
                self.replace_text_in_range(None, &flattened, window, cx);
                return;
            }

            if self.edits_verbatim_text() {
                self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
                self.replace_text_in_range(None, &text, window, cx);
                return;
            }

            if text.contains('\n') || text.contains('\r') {
                let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
                if self.quote_depth > 0 {
                    self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
                    self.replace_text_in_range(None, &normalized, window, cx);
                    return;
                }
                let plain_selected = self.selection_plain_range();
                let (leading, tail) = self.data.text.split_at(plain_selected.start);
                let (_, trailing) =
                    tail.split_at(plain_selected.end.saturating_sub(plain_selected.start));
                let lines = normalized
                    .split('\n')
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                let split_physical_lines = should_split_plain_multiline_paste(&lines);
                cx.emit(BlockEvent::RequestPasteMultiline {
                    leading,
                    lines,
                    trailing,
                    split_physical_lines,
                });
                return;
            }

            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            self.replace_text_in_range(None, &text, window, cx);
        }
    }
}

