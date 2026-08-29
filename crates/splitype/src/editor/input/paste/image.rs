//! Clipboard image paste flow: storage resolution, file materialization,
//! and markdown insertion.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use gpui::*;

use editor_wysiwyg::document::protocol::PastedImageSource;
use crate::editor::engine::controller::*;
use config::settings::{ImagePasteBehavior, read_app_settings};
use markdown::parse::BlockKind;

impl Editor {
    pub(crate) fn current_image_paste_behavior() -> ImagePasteBehavior {
        read_app_settings()
            .map(|preferences| preferences.markdown.image_paste_behavior)
            .unwrap_or(ImagePasteBehavior::None)
    }

    pub(crate) fn image_paste_root_dir(&self) -> anyhow::Result<PathBuf> {
        if let Some(parent) = self.tab().file.path.as_ref().and_then(|path| path.parent()) {
            return Ok(parent.to_path_buf());
        }
        std::env::current_dir().context("failed to resolve current working directory")
    }

    pub(crate) fn clipboard_image_extension(format: ImageFormat) -> &'static str {
        match format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
            ImageFormat::Gif => "gif",
            ImageFormat::Svg => "svg",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
            ImageFormat::Ico => "ico",
            ImageFormat::Pnm => "pnm",
        }
    }

    pub(crate) fn image_target_dir(
        &self,
        behavior: ImagePasteBehavior,
        root_dir: &Path,
        source: &PastedImageSource,
    ) -> anyhow::Result<PathBuf> {
        match behavior {
            ImagePasteBehavior::None | ImagePasteBehavior::CopyToDocumentFolder => {
                Ok(root_dir.to_path_buf())
            }
            ImagePasteBehavior::CopyToAssetsFolder => Ok(root_dir.join("assets")),
            ImagePasteBehavior::CopyToNamedAssetsFolder => {
                let base = self
                    .tab()
                    .file
                    .path
                    .as_ref()
                    .and_then(|path| path.file_stem())
                    .and_then(|stem| stem.to_str())
                    .filter(|stem| !stem.trim().is_empty())
                    .unwrap_or("untitle");
                if self.tab().file.path.is_some() {
                    return Ok(root_dir.join(format!("{base}.assets")));
                }

                for index in 0.. {
                    let folder = if index == 0 {
                        "untitle.assets".to_string()
                    } else {
                        format!("untitle{index}.assets")
                    };
                    let path = root_dir.join(folder);
                    if !path.exists() {
                        return Ok(path);
                    }
                    if matches!(source, PastedImageSource::LocalPath(_)) {
                        continue;
                    }
                }
                unreachable!("unbounded search should always return");
            }
        }
    }

    pub(crate) fn unique_file_path(dir: &Path, preferred_name: &str) -> PathBuf {
        let preferred = Path::new(preferred_name);
        let stem = preferred
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("image");
        let extension = preferred.extension().and_then(|ext| ext.to_str());
        for index in 0.. {
            let file_name = if index == 0 {
                preferred_name.to_string()
            } else if let Some(extension) = extension {
                format!("{stem}{index}.{extension}")
            } else {
                format!("{stem}{index}")
            };
            let candidate = dir.join(file_name);
            if !candidate.exists() {
                return candidate;
            }
        }
        unreachable!("unbounded search should always return");
    }

    pub(crate) fn path_parent_eq(left: &Path, right: &Path) -> bool {
        let Some(parent) = left.parent() else {
            return false;
        };
        let left = parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_path_buf());
        let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
        left == right
    }

    pub(crate) fn materialize_pasted_image(
        &self,
        source: &PastedImageSource,
    ) -> anyhow::Result<(PathBuf, bool)> {
        let behavior = Self::current_image_paste_behavior();
        let root_dir = self.image_paste_root_dir()?;

        if matches!(behavior, ImagePasteBehavior::None)
            && let PastedImageSource::LocalPath(path) = source
        {
            return Ok((path.clone(), false));
        }

        let target_dir = self.image_target_dir(behavior, &root_dir, source)?;
        fs::create_dir_all(&target_dir)
            .with_context(|| format!("failed to create '{}'", target_dir.display()))?;

        match source {
            PastedImageSource::LocalPath(path) => {
                if Self::path_parent_eq(path, &target_dir) {
                    return Ok((path.clone(), behavior != ImagePasteBehavior::None));
                }
                let file_name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("image");
                let target = Self::unique_file_path(&target_dir, file_name);
                fs::copy(path, &target).with_context(|| {
                    format!(
                        "failed to copy '{}' to '{}'",
                        path.display(),
                        target.display()
                    )
                })?;
                Ok((target, behavior != ImagePasteBehavior::None))
            }
            PastedImageSource::ClipboardImage(image) => {
                let file_name = format!(
                    "pasted-image.{}",
                    Self::clipboard_image_extension(image.format)
                );
                let target = Self::unique_file_path(&target_dir, &file_name);
                fs::write(&target, &image.bytes)
                    .with_context(|| format!("failed to write '{}'", target.display()))?;
                Ok((target, behavior != ImagePasteBehavior::None))
            }
        }
    }

    pub(crate) fn markdown_path_string(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    pub(crate) fn markdown_image_target(path: &str) -> String {
        path.chars()
            .flat_map(|ch| match ch {
                '\\' | '(' | ')' | '"' => ['\\', ch].into_iter().collect::<Vec<_>>(),
                _ => [ch].into_iter().collect::<Vec<_>>(),
            })
            .collect()
    }

    pub(crate) fn markdown_image_alt(path: &Path) -> String {
        let alt = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|stem| !stem.is_empty())
            .unwrap_or("image");
        alt.chars()
            .flat_map(|ch| match ch {
                '\\' | ']' => ['\\', ch].into_iter().collect::<Vec<_>>(),
                _ => [ch].into_iter().collect::<Vec<_>>(),
            })
            .collect()
    }

    pub(crate) fn relative_markdown_path(root_dir: &Path, path: &Path) -> Option<String> {
        let relative = path.strip_prefix(root_dir).ok()?;
        Some(format!("./{}", Self::markdown_path_string(relative)))
    }

    pub(crate) fn pasted_image_markdown(
        &self,
        source: &PastedImageSource,
    ) -> anyhow::Result<String> {
        let root_dir = self.image_paste_root_dir()?;
        let (path, relative) = self.materialize_pasted_image(source)?;
        let path_text = if relative {
            Self::relative_markdown_path(&root_dir, &path)
                .ok_or_else(|| anyhow!("failed to create a relative image path"))?
        } else {
            Self::markdown_path_string(&path)
        };
        Ok(format!(
            "![{}]({})",
            Self::markdown_image_alt(&path),
            Self::markdown_image_target(&path_text)
        ))
    }

    pub(crate) fn show_image_paste_error(&self, err: anyhow::Error, cx: &mut Context<Self>) {
        let strings = cx
            .global::<i18n::I18nManager>()
            .strings()
            .clone();
        if let Some(window) = cx.active_window() {
            let ok = strings.info_dialog_ok.clone();
            let title = strings.image_paste_failed_title.clone();
            let detail = err.to_string();
            let _ = window.update(cx, |_view, window, cx| {
                let buttons = [ok.as_str()];
                let _ = window.prompt(PromptLevel::Critical, &title, Some(&detail), &buttons, cx);
            });
        } else {
            tracing::error!(error = %err, "image paste failed");
        }
    }

    pub(crate) fn inserted_image_tree_for_block(
        block: &editor_wysiwyg::document::block::Block,
        markdown: &str,
    ) -> BlockText {
        if block.edits_verbatim_text() || block.kind().is_code_block() {
            BlockText::plain(markdown.to_string())
        } else {
            BlockText::from_markdown(markdown)
        }
    }

    pub(crate) fn replace_current_block_selection_with_image_text(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        leading: &BlockText,
        markdown: &str,
        trailing: &BlockText,
        cx: &mut Context<Self>,
    ) {
        let (kind, text, cursor) = block.read_with(cx, |block, _cx| {
            let mut text = leading.clone();
            text.append(Self::inserted_image_tree_for_block(block, markdown));
            let cursor = text.plain_len();
            text.append(trailing.clone());
            (block.kind(), text, cursor)
        });
        Self::set_block_text_and_kind(block, kind, text, cursor, cx);
        if let Some(binding) = self.table_cell_binding(block.entity_id()) {
            self.sync_table_data_from_grid(&binding.table_block, cx);
        }
        self.focus_block(block.entity_id());
        self.rebuild_reference_registries(cx);
    }

    pub(crate) fn insert_image_block_after_paragraph(
        &mut self,
        block: &Entity<editor_wysiwyg::document::block::Block>,
        leading: &BlockText,
        markdown: &str,
        trailing: &BlockText,
        cx: &mut Context<Self>,
    ) {
        let Some(location) = self.doc().find_block_location(block.entity_id()) else {
            return;
        };
        let leading_empty = leading.plain_len() == 0;
        let trailing_empty = trailing.plain_len() == 0;

        if leading_empty {
            Self::set_block_text_and_kind(
                block,
                BlockKind::Paragraph,
                BlockText::plain(markdown.to_string()),
                markdown.len(),
                cx,
            );
            let image_block = block.clone();
            if !trailing_empty {
                let trailing_block =
                    Self::new_block(cx, BlockData::new(BlockKind::Paragraph, trailing.clone()));
                self.doc_mut().insert_blocks_at(
                    location.parent,
                    location.index + 1,
                    vec![trailing_block],
                    cx,
                );
            }
            self.focus_block(image_block.entity_id());
            self.rebuild_reference_registries(cx);
            return;
        }

        Self::set_block_text_and_kind(
            block,
            BlockKind::Paragraph,
            leading.clone(),
            leading.plain_len(),
            cx,
        );
        let image_block = Self::new_block(cx, BlockData::paragraph(markdown.to_string()));
        let mut inserted = vec![image_block.clone()];
        if !trailing_empty {
            inserted.push(Self::new_block(
                cx,
                BlockData::new(BlockKind::Paragraph, trailing.clone()),
            ));
        }
        self.doc_mut()
            .insert_blocks_at(location.parent, location.index + 1, inserted, cx);
        self.focus_block(image_block.entity_id());
        self.rebuild_reference_registries(cx);
    }

    pub(crate) fn on_paste_image_request(
        &mut self,
        block: Entity<editor_wysiwyg::document::block::Block>,
        leading: &BlockText,
        source: &PastedImageSource,
        trailing: &BlockText,
        cx: &mut Context<Self>,
    ) {
        let markdown = match self.pasted_image_markdown(source) {
            Ok(markdown) => markdown,
            Err(err) => {
                self.show_image_paste_error(err, cx);
                return;
            }
        };

        if self.replace_cross_block_selection_with_text(
            &markdown,
            None,
            false,
            editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        ) {
            return;
        }

        self.prepare_undo_capture(
            editor_wysiwyg::document::protocol::UndoCaptureKind::NonCoalescible,
            cx,
        );
        let can_insert_image_block = self.is_wysiwyg()
            && block.read(cx).kind() == BlockKind::Paragraph
            && self.table_cell_binding(block.entity_id()).is_none()
            && !block.read(cx).edits_verbatim_text();

        if can_insert_image_block {
            self.insert_image_block_after_paragraph(&block, leading, &markdown, trailing, cx);
        } else {
            self.replace_current_block_selection_with_image_text(
                &block, leading, &markdown, trailing, cx,
            );
        }

        self.mark_dirty(cx);
        self.finalize_pending_undo_capture(cx);
        cx.notify();
    }
}
