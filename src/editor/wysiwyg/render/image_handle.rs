//! Rendered standalone image runtime state.

use std::path::Path;

use crate::editor::tree::block::{Block, ImageHandle};
use crate::model::block::BlockKind;
use crate::model::syntax::image::{ImageSyntax, parse_standalone_image, resolve_image_source};

impl Block {
    pub(crate) fn image_runtime(&self) -> Option<&ImageHandle> {
        self.image_runtime.as_ref()
    }

    pub(super) fn can_present_as_image(&self) -> bool {
        self.is_table_cell()
            || matches!(
                self.kind(),
                BlockKind::Paragraph
                    | BlockKind::BulletListItem
                    | BlockKind::NumberedListItem
                    | BlockKind::TaskListItem { .. }
            )
    }

    /// Whether this block's text is a lone image that renders as a
    /// self-contained image widget. Unlike `showing_rendered_image`, this is
    /// derived from the block text rather than the computed runtime, so it is
    /// valid before image runtimes are (re)built.
    pub(crate) fn renders_as_standalone_image(&self) -> bool {
        self.can_present_as_image() && self.standalone_image_markdown_for_runtime().is_some()
    }

    pub(super) fn compute_image_runtime(
        &self,
        base_dir: Option<&Path>,
        syntax: ImageSyntax,
    ) -> Option<ImageHandle> {
        let resolved_target = syntax.resolve_target(&self.image_reference_definitions)?;
        self.can_present_as_image().then(|| ImageHandle {
            alt: syntax.alt.clone(),
            src: resolved_target.src.clone(),
            title: resolved_target.title.clone(),
            resolved_source: resolve_image_source(&resolved_target.src, base_dir),
        })
    }

    pub(crate) fn image_runtime_for_syntax(&self, syntax: ImageSyntax) -> Option<ImageHandle> {
        self.compute_image_runtime(self.image_base_dir.as_deref(), syntax)
    }

    pub(crate) fn image_base_dir(&self) -> Option<&Path> {
        self.image_base_dir.as_deref()
    }

    pub(crate) fn sync_image_runtime(&mut self) -> bool {
        let next_runtime = if self.can_present_as_image() {
            self.standalone_image_markdown_for_runtime()
                .and_then(|markdown| parse_standalone_image(&markdown))
                .and_then(|syntax| {
                    self.compute_image_runtime(self.image_base_dir.as_deref(), syntax)
                })
        } else {
            None
        };

        if self.image_runtime == next_runtime {
            return false;
        }
        self.image_runtime = next_runtime;
        true
    }

    fn standalone_image_markdown_for_runtime(&self) -> Option<String> {
        let visible = self.record.text.visible_text();
        if parse_standalone_image(&visible).is_some() {
            return Some(visible);
        }

        let serialized = self.record.text.serialize_markdown();
        parse_standalone_image(&serialized)
            .is_some()
            .then_some(serialized)
    }

    pub(crate) fn sync_image_focus_state(&mut self, _focused: bool) -> bool {
        false
    }

    pub(crate) fn showing_rendered_image(&self) -> bool {
        self.image_runtime.is_some() && !self.is_source_raw_mode()
    }
}
