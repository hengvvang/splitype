//! Rendered standalone image handle state.

use std::path::Path;

use crate::editor::tree::block::{Block, ImageHandle};
use crate::model::parse::BlockKind;
use crate::model::block::image::{ImageSyntax, parse_standalone_image, resolve_image_source};

impl Block {
    pub(crate) fn image_handle(&self) -> Option<&ImageHandle> {
        self.image_handle.as_ref()
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
    /// derived from the block text rather than the computed handle, so it is
    /// valid before image handles are (re)built.
    pub(crate) fn is_standalone_image(&self) -> bool {
        self.can_present_as_image() && self.standalone_image_markdown_for_handle().is_some()
    }

    pub(super) fn compute_image_handle(
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

    pub(crate) fn image_handle_for_syntax(&self, syntax: ImageSyntax) -> Option<ImageHandle> {
        self.compute_image_handle(self.image_base_dir.as_deref(), syntax)
    }

    pub(crate) fn image_base_dir(&self) -> Option<&Path> {
        self.image_base_dir.as_deref()
    }

    pub(crate) fn sync_image_handle(&mut self) -> bool {
        let next_runtime = if self.can_present_as_image() {
            self.standalone_image_markdown_for_handle()
                .and_then(|markdown| parse_standalone_image(&markdown))
                .and_then(|syntax| {
                    self.compute_image_handle(self.image_base_dir.as_deref(), syntax)
                })
        } else {
            None
        };

        if self.image_handle == next_runtime {
            return false;
        }
        self.image_handle = next_runtime;
        true
    }

    fn standalone_image_markdown_for_handle(&self) -> Option<String> {
        let visible = self.data.text.plain_text();
        if parse_standalone_image(&visible).is_some() {
            return Some(visible);
        }

        let serialized = self.data.text.serialize_markdown();
        parse_standalone_image(&serialized)
            .is_some()
            .then_some(serialized)
    }

    pub(crate) fn sync_image_focus_state(&mut self, _focused: bool) -> bool {
        false
    }

    pub(crate) fn is_showing_rendered_image(&self) -> bool {
        self.image_handle.is_some() && !self.is_verbatim_mode()
    }
}
