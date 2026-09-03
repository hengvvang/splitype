//! Pointer interaction handlers on a focused block: mouse-down/up/move,
//! rendered-link hits, footnote backrefs, and task checkboxes.

use gpui::*;

use crate::model::block::Block;
use crate::model::protocol::BlockEvent;
impl Block {
    pub fn focus_and_select_footnote_reference(
        &mut self,
        occurrence_index: usize,
        footnote_id: String,
        anchor_pos: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;
        self.sync_inline_projection_for_focus(true);
        if let Some(range) = self.display_range_for_footnote(&footnote_id, occurrence_index) {
            let plain_selected = self.display_to_plain_range(range.clone());
            let supports_projection = self.edit_mode.supports_inline_projection();
            let kind_key = match self.kind() {
                markdown_parser::parse::BlockKind::Heading { level } => Some(level),
                markdown_parser::parse::BlockKind::Callout(variant) => Some(10 + variant as u8),
                _ => None,
            };
            self.projection_cache_key = Some((supports_projection, kind_key, plain_selected, None));
            self.selected_range = range;
            self.selection_reversed = false;
            self.marked_range = None;
        }
        self.focus_handle.focus(window, cx);
        self.start_cursor_blink(cx);
        cx.emit(BlockEvent::RequestFocus);
        cx.emit(BlockEvent::RequestFootnoteTooltip {
            id: footnote_id,
            content: None,
            position: anchor_pos,
            show: false,
        });
        cx.notify();
    }

    pub fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let was_focused = self.focus_handle.is_focused(window);
        let offset = self.index_for_mouse_position(event.position);

        // Thematic break: clicking on the divider places cursor at the end to facilitate editing.
        if self.kind() == markdown_parser::parse::BlockKind::ThematicBreak && (!was_focused || offset == 0) {
            self.is_selecting = false;
            let end_offset = self.display_text().len();
            self.move_to(end_offset, cx);
            self.focus_handle.focus(window, cx);
            self.start_cursor_blink(cx);
            cx.emit(BlockEvent::RequestFocus);
            return;
        }

        // Footnote superscript: clicking enters edit mode and selects the footnote id (e.g. "ref" in "[^ref]").
        if !was_focused
            && let Some((footnote, _)) = self.last_paint_at(event.position).and_then(|paint| {
                crate::render::text_layout::footnote_at_position(
                    self,
                    &paint.layout,
                    paint.bounds,
                    paint.line_height,
                    event.position,
                )
            })
        {
            self.focus_and_select_footnote_reference(
                footnote.occurrence_index,
                footnote.id.clone(),
                event.position,
                window,
                cx,
            );
            cx.stop_propagation();
            return;
        }

        // Cmd/Ctrl+click follows a rendered link instead of editing it, so the
        // block is neither focused nor selected; the link opens on mouse-up.
        if event.modifiers.secondary() && self.pointer_link_hit(event.position).is_some() {
            self.is_selecting = false;
            cx.stop_propagation();
            return;
        }

        self.is_selecting = true;
        if was_focused && event.modifiers.shift {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }

        self.focus_handle.focus(window, cx);
        cx.emit(BlockEvent::RequestFocus);
    }

    /// Resolve the inline link under a pointer position against the most recent
    /// rendered text layout, if any. Returns `None` while the block shows raw
    /// source or when the pointer is not over a link.
    pub fn pointer_link_hit(
        &self,
        position: Point<Pixels>,
    ) -> Option<markdown_parser::inline::link::InlineLinkHit> {
        self.last_paint_at(position)
            .and_then(|paint| {
                crate::render::text_layout::link_at_position(
                    self,
                    &paint.layout,
                    paint.bounds,
                    paint.line_height,
                    position,
                )
            })
            .cloned()
    }

    /// Handle mouse-down on a rendered inline link (in a mixed inline-visual
    /// block). A Cmd/Ctrl+click is claimed here so it follows the link instead
    /// of focusing the block; the destination opens on the matching mouse-up.
    pub fn on_wysiwyg_link_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Only Cmd/Ctrl+click follows the link; a plain click falls through so
        // the block focuses for editing like any other inline text.
        if event.modifiers.secondary() {
            cx.stop_propagation();
        }
    }

    /// Open a rendered inline link's destination through the editor prompt.
    pub fn open_wysiwyg_link(
        &mut self,
        link: &markdown_parser::inline::link::InlineLinkHit,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        cx.emit(BlockEvent::RequestOpenLink {
            prompt_target: link.prompt_target.clone(),
            open_target: link.open_target.clone(),
        });
    }

    pub fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;

        // Cmd/Ctrl+click follows a rendered link.
        if event.modifiers.secondary()
            && let Some(link) = self.pointer_link_hit(event.position)
        {
            self.open_wysiwyg_link(&link, cx);
        }
    }

    pub fn on_footnote_backref_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if !self.focus_handle.is_focused(window) {
            cx.emit(BlockEvent::RequestFocus);
        }
    }

    pub fn on_footnote_backref_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.footnote_definition_id() else {
            return;
        };
        cx.stop_propagation();
        cx.emit(BlockEvent::RequestJumpToFootnoteBackref { id });
    }

    pub fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            // A stale selecting flag can survive a missed mouse-up. Only extend
            // the selection while the platform still reports an active drag.
            if !event.dragging() {
                self.is_selecting = false;
                cx.notify();
                return;
            }
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }

        // Footnote reference hover: surface the definition content in a
        // tooltip while the pointer rests on a rendered reference.
        let hit = self
            .last_paint_at(event.position)
            .and_then(|paint| {
                crate::render::text_layout::footnote_at_position(
                    self,
                    &paint.layout,
                    paint.bounds,
                    paint.line_height,
                    event.position,
                )
            })
            .map(|(footnote, bounds)| (footnote.id.clone(), point(bounds.left(), bounds.bottom())));
        let hovered = hit.as_ref().map(|(id, _)| id.clone());
        if hovered != self.hovered_footnote_id {
            let show = hovered.is_some();
            self.hovered_footnote_id = hovered.clone();
            let anchor_pos = hit.as_ref().map(|(_, pos)| *pos).unwrap_or(event.position);
            cx.emit(BlockEvent::RequestFootnoteTooltip {
                id: hovered.unwrap_or_default(),
                content: None,
                position: anchor_pos,
                show,
            });
        }
    }

    pub fn on_task_checkbox_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if !self.focus_handle.is_focused(window) {
            cx.emit(BlockEvent::RequestFocus);
        }
    }

    pub fn on_task_checkbox_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.kind().is_task_list_item() || self.is_verbatim_mode() {
            return;
        }

        cx.stop_propagation();
        cx.emit(BlockEvent::RequestToggleTaskChecked);
    }
}
