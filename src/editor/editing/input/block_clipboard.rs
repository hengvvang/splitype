//! Clipboard action handlers on a focused block: copy, cut, and paste,
//! including local-image-path routing from the clipboard text.

use gpui::*;

use crate::editor::block_protocol::{BlockAction, PastedImageSource, UndoCaptureKind};
use crate::editor::editing::input::actions::{Copy, Cut, Paste};
use crate::editor::editing::input::paste::should_split_plain_multiline_paste;
use crate::editor::tree::block::Block;
use crate::model::inline::text::RichText;
impl Block {
    fn pasted_image_source_from_clipboard(item: &ClipboardItem) -> Option<PastedImageSource> {
        item.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(PastedImageSource::ClipboardImage(image.clone())),
            ClipboardEntry::String(_) => None,
        })
    }

    fn pasted_image_source_from_text(text: &str) -> Option<PastedImageSource> {
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
    fn pasted_image_path_from_text_item(text: &str) -> Option<std::path::PathBuf> {
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

    fn is_supported_local_image_path(path: &std::path::Path) -> bool {
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

    fn paste_image_split(&self) -> (RichText, RichText) {
        let plain_selected = self.selection_plain_range();
        let (leading, tail) = self.data.text.split_at(plain_selected.start);
        let (_, trailing) = tail.split_at(plain_selected.end.saturating_sub(plain_selected.start));
        (leading, trailing)
    }
    pub(crate) fn on_copy(&mut self, _: &Copy, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.display_text()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub(crate) fn on_cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.prepare_undo_capture(UndoCaptureKind::NonCoalescible, cx);
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.display_text()[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    pub(crate) fn on_paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.kind().is_thematic_break() && !self.edits_verbatim_text() {
            return;
        }

        if let Some(item) = cx.read_from_clipboard() {
            if let Some(source) = Self::pasted_image_source_from_clipboard(&item) {
                let (leading, trailing) = self.paste_image_split();
                cx.emit(BlockAction::RequestPasteImage {
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
                cx.emit(BlockAction::RequestPasteImage {
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
                cx.emit(BlockAction::RequestReplaceCrossBlockSelection {
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
                cx.emit(BlockAction::RequestPasteMultiline {
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

#[cfg(test)]
mod tests {
    use super::Block;
    use crate::editor::block_protocol::PastedImageSource;
    use std::fs;

    fn temp_image_path(name: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "splitype-paste-image-path-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&root).expect("temp image dir should exist");
        let path = root.join(name);
        fs::write(
            &path,
            b"not a real image; extension is enough for paste routing",
        )
        .expect("temp image should be written");
        path
    }

    fn remove_temp_image(path: &std::path::Path) {
        let _ = path.parent().map(|parent| fs::remove_dir_all(parent));
    }

    #[test]
    fn paste_image_text_accepts_plain_local_image_path() {
        let path = temp_image_path("copied.png");
        let text = path.to_string_lossy().to_string();
        #[cfg(target_os = "windows")]
        assert!(
            text.contains(':'),
            "test should exercise Windows drive-letter paths"
        );

        let source = Block::pasted_image_source_from_text(&text);

        assert_eq!(source, Some(PastedImageSource::LocalPath(path.clone())));
        remove_temp_image(&path);
    }

    #[test]
    fn paste_image_text_accepts_quoted_local_image_path() {
        let path = temp_image_path("quoted image.png");
        let text = format!("\"{}\"", path.display());

        let source = Block::pasted_image_source_from_text(&text);

        assert_eq!(source, Some(PastedImageSource::LocalPath(path.clone())));
        remove_temp_image(&path);
    }

    #[test]
    fn paste_image_text_accepts_file_url() {
        let path = temp_image_path("url image.png");
        let url = url::Url::from_file_path(&path).expect("temp image path should form file URL");

        let source = Block::pasted_image_source_from_text(url.as_str());

        assert_eq!(source, Some(PastedImageSource::LocalPath(path.clone())));
        remove_temp_image(&path);
    }

    #[test]
    fn paste_image_text_rejects_non_image_path() {
        let path = temp_image_path("notes.txt");
        let text = path.to_string_lossy().to_string();

        let source = Block::pasted_image_source_from_text(&text);

        assert_eq!(source, None);
        remove_temp_image(&path);
    }
}
