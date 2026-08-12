//! Pointer interaction handlers on a focused block: mouse-down/up/move,
//! rendered-link hits, footnote backrefs, and task checkboxes.

use gpui::*;

use crate::editor::block_protocol::BlockAction;
use crate::editor::tree::block::Block;
impl Block {
    pub(crate) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.showing_rendered_image() {
            let offset = self.index_for_mouse_position(event.position);
            let was_focused = self.focus_handle.is_focused(window);

            if was_focused {
                self.is_selecting = true;
                if event.modifiers.shift {
                    self.select_to(offset, cx);
                } else {
                    self.move_to(offset, cx);
                }
            } else {
                self.is_selecting = false;
                self.move_to(offset, cx);
                cx.emit(BlockAction::RequestFocus);
            }
            cx.stop_propagation();
            return;
        }

        let offset = self.index_for_mouse_position(event.position);
        let was_focused = self.focus_handle.is_focused(window);

        // Cmd/Ctrl+click follows a rendered link instead of editing it, so the
        // block is neither focused nor selected; the link opens on mouse-up.
        if event.modifiers.secondary() && self.pointer_link_hit(event.position).is_some() {
            self.is_selecting = false;
            cx.stop_propagation();
            return;
        }

        if was_focused {
            self.is_selecting = true;
            if event.modifiers.shift {
                self.select_to(offset, cx);
            } else {
                self.move_to(offset, cx);
            }
        } else {
            self.is_selecting = false;
            self.move_to(offset, cx);
            cx.emit(BlockAction::RequestFocus);
        }
    }

    /// Resolve the inline link under a pointer position against the most recent
    /// rendered text layout, if any. Returns `None` while the block shows raw
    /// source or when the pointer is not over a link.
    pub(crate) fn pointer_link_hit(
        &self,
        position: Point<Pixels>,
    ) -> Option<crate::model::inline::link::InlineLinkHit> {
        self.last_paint_at(position)
            .and_then(|paint| {
                crate::editor::geometry::text_layout::link_at_position(
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
    pub(crate) fn on_rendered_link_mouse_down(
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
    pub(crate) fn open_rendered_link(
        &mut self,
        link: &crate::model::inline::link::InlineLinkHit,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        cx.emit(BlockAction::RequestOpenLink {
            prompt_target: link.prompt_target.clone(),
            open_target: link.open_target.clone(),
        });
    }

    pub(crate) fn on_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;

        // Cmd/Ctrl+click follows a rendered link, using the same open-link
        // prompt as the double-click gesture below.
        if event.modifiers.secondary()
            && let Some(link) = self.pointer_link_hit(event.position)
        {
            self.open_rendered_link(&link, cx);
            return;
        }

        if event.click_count >= 2 {
            let footnote = self
                .last_paint_at(event.position)
                .and_then(|paint| {
                    crate::editor::geometry::text_layout::footnote_at_position(
                        self,
                        &paint.layout,
                        paint.bounds,
                        paint.line_height,
                        event.position,
                    )
                })
                .cloned();
            if let Some(footnote) = footnote {
                cx.stop_propagation();
                cx.emit(BlockAction::RequestJumpToFootnoteDefinition { id: footnote.id });
                return;
            }

            if let Some(link) = self.pointer_link_hit(event.position) {
                self.open_rendered_link(&link, cx);
            }
        }
    }

    pub(crate) fn on_footnote_backref_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if !self.focus_handle.is_focused(window) {
            cx.emit(BlockAction::RequestFocus);
        }
    }

    pub(crate) fn on_footnote_backref_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.footnote_definition_id() else {
            return;
        };
        cx.stop_propagation();
        cx.emit(BlockAction::RequestJumpToFootnoteBackref { id });
    }

    pub(crate) fn on_mouse_move(
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
        let hovered = self
            .last_paint_at(event.position)
            .and_then(|paint| {
                crate::editor::geometry::text_layout::footnote_at_position(
                    self,
                    &paint.layout,
                    paint.bounds,
                    paint.line_height,
                    event.position,
                )
            })
            .map(|footnote| footnote.id.clone());
        if hovered != self.hovered_footnote_id {
            let show = hovered.is_some();
            self.hovered_footnote_id = hovered.clone();
            cx.emit(BlockAction::RequestFootnoteTooltip {
                id: hovered.unwrap_or_default(),
                content: None,
                position: event.position,
                show,
            });
        }
    }

    /// Show the definition content tooltip while the pointer hovers a
    /// footnote definition header; hide it when the pointer leaves.
    pub(crate) fn on_footnote_header_hover(
        &mut self,
        hovered: &bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.hovered_footnote_id.is_some() {
            self.hovered_footnote_id = None;
        }
        cx.emit(BlockAction::RequestFootnoteTooltip {
            id: self.footnote_definition_id().unwrap_or_default(),
            content: (*hovered).then(|| self.data.text.plain_text().into()),
            position: window.mouse_position(),
            show: *hovered,
        });
    }

    pub(crate) fn on_task_checkbox_mouse_down(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        if !self.focus_handle.is_focused(window) {
            cx.emit(BlockAction::RequestFocus);
        }
    }

    pub(crate) fn on_task_checkbox_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.kind().is_task_list_item() || self.is_verbatim_mode() {
            return;
        }

        cx.stop_propagation();
        cx.emit(BlockAction::ToggleTaskChecked);
    }
}
