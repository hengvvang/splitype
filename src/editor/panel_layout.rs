//! Editor inner panel layout — rendering for the `EditorInnerPanelKind`
//! split tree (welcome panel + Wysiwyg / Source Code / Preview / Outline
//! editing panels) inside each Edit area.
//!
//! The inner-panel state and operations live in
//! `crate::editor::session_ops`; the window-level area layout
//! rendering lives in `crate::editor::window_layout`.

use crate::editor::session::{
    EditingPanelKind, EditorInnerPanelKind, InnerPanelLocation, WelcomePanelKind,
};
use crate::splitter::Axis;
use crate::ui::popover::menu_panel;
use splitype_splitter::tree::SplitTree;

use gpui::*;

use crate::editor::controller::*;
use crate::infra::i18n::I18nStrings;
use crate::infra::theme::Theme;

impl Editor {
    /// Render one Editor area's inner panel layout. While building, the
    /// routing hint (`current_tab_area`) points at this area so every panel
    /// reads THIS editor's tab list.
    ///
    /// Side effects before rendering the tree:
    /// - drop runtimes of panels that were closed or joined;
    /// - derive `focused_editor_inner_panel` from the keyboard focus when
    ///   nothing was explicitly selected (projection fallback).
    pub(crate) fn render_editor_midcontainer(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        self.tab_list_mut_for(area_id);
        let inner_tree = self.ensure_editor_session(area_id).splitter.tree.clone();

        // Drop runtimes of panels that were closed or joined.
        self.source_code_panel_runtimes
            .retain(|(area, panel), _| *area == area_id && inner_tree.contains_leaf(*panel));

        let previous = self.current_tab_area;
        self.current_tab_area = Some(area_id);

        // Derive the focused panel from the keyboard focus when nothing is
        // focused yet — clicking inside a block or Tab navigation never
        // reaches the panel div. Explicit clicks take precedence. Only runs
        // for editing areas: a welcome area has no tabs, hence no edit
        // targets to derive from.
        if self.focused_editor_inner_panel.is_none()
            && self.area_mode(area_id).is_editing()
            && let Some(target_id) = self.focused_edit_target_entity_id(window, cx)
        {
            if let Some(((_, panel_id), _)) =
                self.source_code_panel_runtimes
                    .iter()
                    .find(|((area, _), runtime)| {
                        *area == area_id
                            && runtime
                                .block
                                .as_ref()
                                .is_some_and(|block| block.entity_id() == target_id)
                    })
            {
                // Keyboard focus sits in a source panel's own block.
                if inner_tree.contains_leaf(*panel_id) {
                    self.focused_editor_inner_panel = Some(InnerPanelLocation {
                        area_id,
                        panel_id: *panel_id,
                    });
                }
            } else if self.doc().block_entity_by_id(target_id).is_some() {
                // Keyboard focus sits in the shared document: point at the
                // area's first Wysiwyg panel.
                let mut ids = Vec::new();
                inner_tree.leaf_ids(&mut ids);
                if let Some(panel_id) = ids.into_iter().find(|id| {
                    inner_tree.find_leaf_kind(*id)
                        == Some(EditorInnerPanelKind::Editing(EditingPanelKind::Wysiwyg))
                }) {
                    self.focused_editor_inner_panel =
                        Some(InnerPanelLocation { area_id, panel_id });
                }
            }
        }

        let inner_rendered =
            self.render_editor_inner_panel_node(&inner_tree, area_id, theme, strings, window, cx);
        self.current_tab_area = previous;

        let dropdown = {
            let splitter = &self.ensure_editor_session(area_id).splitter;
            // The open dropdown lives on its panel (panel-level state).
            let mut ids = Vec::new();
            splitter.tree.leaf_ids(&mut ids);
            let open_panel = ids.into_iter().find(|id| {
                splitter
                    .tree
                    .find_leaf(*id)
                    .is_some_and(|p| p.open_dropdown)
            });
            if let Some(panel_id) = open_panel {
                let current_type = splitter
                    .tree
                    .find_leaf_kind(panel_id)
                    .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                        None,
                    )));
                Some(self.render_editor_inner_panel_dropdown_menu(
                    area_id,
                    panel_id,
                    current_type,
                    theme,
                    cx,
                ))
            } else {
                None
            }
        };

        let mut container = div()
            .w_full()
            .h_full()
            .relative()
            .bg(c.editor_background)
            .child(inner_rendered);

        if let Some(dropdown) = dropdown {
            container = container.child(dropdown);
        }

        // Inner corner-drag preview: rendered inside the midcontainer so
        // the normalized rects position with `relative()` against the
        // layout's initialization region (topbar/bottombar excluded). Host
        // policy: only plain (no-modifier) drags show an indicator.
        let d = &theme.dimensions;
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: d.area_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            ..Default::default()
        };
        let inner_size = {
            let viewport = window.viewport_size();
            let outer_rects = self.panels.layout.leaf_rects(viewport);
            self.panels
                .layout
                .leaf_rect(area_id, &outer_rects)
                .map(|rect| size(px(rect.width), px(rect.height)))
                .unwrap_or(viewport)
        };
        // The corner-drag session lives on the dragging panel itself;
        // find it via the root.
        if let Some(drag_panel) = self.ensure_editor_session(area_id).splitter.corner_drag_panel() {
            let drag = self
                .ensure_editor_session(area_id)
                .splitter
                .tree
                .find_leaf(drag_panel)
                .unwrap()
                .active_corner_drag
                .unwrap();
            if drag.modifier == splitype_splitter::sessions::CornerDragModifier::None {
                if let Some(preview) =
                    crate::editor::corner_drag_preview::render_corner_drag_preview(
                        &self.ensure_editor_session(area_id).splitter,
                        &drag,
                        inner_size,
                        &overlay_style,
                    )
                {
                    container = container.child(preview);
                }
            }
        }

        // Inner-panel border menu: same context menu as the outer window
        // areas, rendered by the layout crate and wired to the per-area
        // inner-panel operations. The split node id doubles as the id of
        // its second (right/bottom) leaf, matching the outer tree's
        // semantics: Split/Close act on that side, Swap flips the sides.
        if let Some(border_menu) = self
            .ensure_editor_session(area_id)
            .splitter
            .active_border_menu
        {
            let menu_overlay =
                self.render_editor_inner_panel_border_menu(border_menu, area_id, theme, cx);
            container = container.child(menu_overlay);
        }

        container.into_any_element()
    }

    pub(crate) fn render_editor_inner_panel_border_menu(
        &mut self,
        border_menu: crate::splitter::BorderMenuState,
        area_id: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;
        let menu_style = crate::editor::window_layout::border_menu_style(theme);

        let split_h_ed = editor.clone();
        let split_h: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_h_ed.update(app, |ed, cx| {
                ed.split_editor_inner_panel_with_ratio(area_id, split_id, Axis::Horizontal, 0.5);
                ed.editor_sessions
                    .get_mut(&area_id)
                    .unwrap()
                    .splitter
                    .active_border_menu = None;
                cx.notify();
            });
        });
        let split_v_ed = editor.clone();
        let split_v: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = split_v_ed.update(app, |ed, cx| {
                ed.split_editor_inner_panel_with_ratio(area_id, split_id, Axis::Vertical, 0.5);
                ed.editor_sessions
                    .get_mut(&area_id)
                    .unwrap()
                    .splitter
                    .active_border_menu = None;
                cx.notify();
            });
        });
        let swap_ed = editor.clone();
        let swap: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = swap_ed.update(app, |ed, cx| {
                ed.swap_editor_inner_panel_split_sides(area_id, split_id);
                cx.notify();
            });
        });
        let close_ed = editor.clone();
        let close: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = close_ed.update(app, |ed, cx| {
                ed.close_editor_inner_panel(area_id, split_id);
                ed.editor_sessions
                    .get_mut(&area_id)
                    .unwrap()
                    .splitter
                    .active_border_menu = None;
                cx.notify();
            });
        });
        let dismiss_ed = editor.clone();
        let dismiss: Box<dyn Fn(&mut App)> = Box::new(move |app| {
            let _ = dismiss_ed.update(app, |ed, cx| {
                ed.editor_sessions
                    .get_mut(&area_id)
                    .unwrap()
                    .splitter
                    .active_border_menu = None;
                cx.notify();
            });
        });

        crate::splitter::interaction::render_border_menu(
            border_menu.position,
            vec![
                crate::splitter::interaction::BorderMenuItem {
                    label: "Split Horizontally",
                    on_activate: split_h,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Split Vertically",
                    on_activate: split_v,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Swap Panels",
                    on_activate: swap,
                },
                crate::splitter::interaction::BorderMenuItem {
                    label: "Close Panel",
                    on_activate: close,
                },
            ],
            &menu_style,
            dismiss,
        )
    }
    /// Welcome prompt shown when the explorer is empty: double-click to
    /// start temporary editing in an Untitled tab, or open a file from the
    /// menus. `panel_id` scopes the element id so multiple split panels can
    /// each host their own prompt.
    pub(crate) fn render_welcome_prompt(
        &mut self,
        area_id: usize,
        panel_id: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        div()
            .id(ElementId::Name(
                format!("welcome-prompt-{area_id}-{panel_id}").into(),
            ))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .bg(c.editor_background)
            .cursor_pointer()
            // GPUI has no double-click event; track click timestamps in
            // editor state (closure-local state is rebuilt every frame).
            .on_click(move |_event, window, cx| {
                let now = std::time::Instant::now();
                let _ = editor.update(cx, |ed, cx| {
                    let is_double = ed.welcome_last_click.is_some_and(|previous| {
                        now.duration_since(previous) < std::time::Duration::from_millis(500)
                    });
                    ed.welcome_last_click = Some(now);
                    if is_double {
                        // The clicked editor becomes the active editor and
                        // its routing context is set for the tab creation.
                        ed.panels.layout.activate_area(area_id);
                        ed.current_tab_area = Some(area_id);
                        ed.new_untitled_tab(area_id, cx);
                        ed.current_tab_area = None;
                        // Focus the new source panel so typing works
                        // immediately after entering editing.
                        ed.focus_editor_inner_panel(area_id, panel_id, window, cx);
                    }
                });
            })
            .child(
                div()
                    .text_size(px(d.menu_text_size.max(13.0)))
                    .text_color(c.text_default)
                    .font_weight(FontWeight::MEDIUM)
                    .child("Double-click to start editing"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.dialog_muted)
                    .child("Or open a file from the explorer or menus"),
            )
            .into_any_element()
    }

    pub(crate) fn render_editor_inner_panel_node(
        &mut self,
        node: &splitype_splitter::tree::SplitTree<crate::editor::session::EditorInnerPanelKind>,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let overlay_style = splitype_splitter::interaction::OverlayStyle {
            accent: c.split_indicator,
            tile_radius: d.area_tile_radius,
            border: c.dialog_border,
            selection: c.selection,
            active: c.focus_accent,
            ..Default::default()
        };

        match node {
            SplitTree::Leaf(container) => {
                let panel_id = container.id;
                let kind = container.kind;
                let inner_editor = cx.entity().downgrade();

                // The panel kind carries the mode: welcome panels render
                // the guidance prompt, editing panels render their view.
                let inner_body: AnyElement = match kind {
                    EditorInnerPanelKind::Welcome(_) => {
                        self.render_welcome_prompt(area_id, panel_id, theme, cx)
                    }
                    EditorInnerPanelKind::Editing(kind) => match kind {
                        // WYSIWYG — this editor's own block editor view.
                        EditingPanelKind::Wysiwyg => {
                            self.render_document_view(area_id, panel_id, window, cx)
                        }
                        // Source — interactive source code editor. Uses a
                        // cached block in source-document mode; edits sync to
                        // the shared document via the block's Changed event.
                        EditingPanelKind::SourceCode => {
                            self.sync_source_code_panel(area_id, panel_id, cx);
                            self.render_source_code_panel(area_id, panel_id, theme, cx)
                        }
                        EditingPanelKind::Preview => self.render_tiled_preview_panel(
                            area_id, panel_id, theme, strings, window, cx,
                        ),
                        EditingPanelKind::Outline => {
                            self.render_tiled_outline_panel(area_id, panel_id, theme, strings, cx)
                        }
                    },
                };

                let corner_handles = splitype_splitter::interaction::corner_drag_handles(
                    "inner-corner",
                    panel_id,
                    d.inner_panel_gap,
                    20.0,
                    false,
                    false,
                    move |modifier, pos, cx| {
                        let _ = inner_editor.update(cx, |ed, cx| {
                            ed.editor_sessions
                                .get_mut(&area_id)
                                .unwrap()
                                .splitter
                                .start_corner_drag(panel_id, pos, modifier);
                            cx.notify();
                        });
                    },
                );

                // Auto-focus first inner panel if none is focused.
                if self.focused_editor_inner_panel.is_none() {
                    self.focused_editor_inner_panel =
                        Some(InnerPanelLocation { area_id, panel_id });
                    // The auto-focused editor becomes the active editor too.
                    self.panels.layout.activate_area(area_id);
                }

                let focus_editor = cx.entity().downgrade();
                let panel_gap = d.inner_panel_gap;

                // The leaf container is the split-out area: unadorned and
                // seamless, it only partitions the initialized region. The
                // panel floats inside it with a uniform inset on all four
                // sides and carries the content.
                div()
                    .w_full()
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .inset(px(panel_gap))
                            .flex()
                            .flex_col()
                            .rounded(px(d.area_tile_radius))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .shadow_lg()
                            .child(div().w_full().flex_1().min_h(px(0.0)).child(inner_body))
                            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                                let _ = focus_editor.update(cx, |ed, cx| {
                                    // Select this panel and move the keyboard edit
                                    // focus to its editing target (source block or
                                    // the shared document block).
                                    ed.focus_editor_inner_panel(area_id, panel_id, window, cx);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(corner_handles)
                    .into_any_element()
            }
            SplitTree::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let dir = *direction;
                let r = *ratio;
                let first_elem =
                    self.render_editor_inner_panel_node(first, area_id, theme, strings, window, cx);
                let second_elem = self
                    .render_editor_inner_panel_node(second, area_id, theme, strings, window, cx);

                let inner_editor = cx.entity().downgrade();

                match direction {
                    Axis::Horizontal => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active =
                            self.editor_sessions.get(&area_id).is_some_and(|session| {
                                session
                                    .splitter
                                    .active_splitter_drag
                                    .is_some_and(|drag| drag.split_id == split_id)
                            });
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
                            .child(
                                div()
                                    .w(relative(r))
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .h_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                splitype_splitter::interaction::splitter_bar_h(
                                    ("inner-splitter-bar-h", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                                    let start_pos = f32::from(event.position.x);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        if let Some(session) = ed.editor_sessions.get_mut(&area_id)
                                        {
                                            // The move handler tracks the pointer in the
                                            // area's local space, so rebase the start
                                            // position the same way.
                                            let local_start = ed
                                                .panels
                                                .layout
                                                .leaf_rect(
                                                    area_id,
                                                    &ed.panels
                                                        .layout
                                                        .leaf_rects(window.viewport_size()),
                                                )
                                                .map(|rect| start_pos - rect.x)
                                                .unwrap_or(start_pos);
                                            splitype_splitter::interaction::start_splitter_drag(
                                                &mut session.splitter,
                                                split_id,
                                                Axis::Horizontal,
                                                local_start,
                                                r,
                                            );
                                            cx.notify();
                                        }
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            if let Some(session) =
                                                ed.editor_sessions.get_mut(&area_id)
                                            {
                                                splitype_splitter::interaction::open_border_menu(
                                                    &mut session.splitter,
                                                    split_id,
                                                    dir,
                                                    pos,
                                                );
                                                cx.notify();
                                            }
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                    Axis::Vertical => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        let bar_active =
                            self.editor_sessions.get(&area_id).is_some_and(|session| {
                                session
                                    .splitter
                                    .active_splitter_drag
                                    .is_some_and(|drag| drag.split_id == split_id)
                            });
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .relative()
                            .child(
                                div()
                                    .h(relative(r))
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_shrink_0()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(first_elem),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .overflow_hidden()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .child(second_elem),
                            )
                            .child(
                                splitype_splitter::interaction::splitter_bar_v(
                                    ("inner-splitter-bar-v", split_id),
                                    r,
                                    bar_active,
                                    &overlay_style,
                                )
                                .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                                    let start_pos = f32::from(event.position.y);
                                    let _ = bar_editor.update(cx, |ed, cx| {
                                        if let Some(session) = ed.editor_sessions.get_mut(&area_id)
                                        {
                                            // Rebase the start position into the area's
                                            // local space, matching the move handler.
                                            let local_start = ed
                                                .panels
                                                .layout
                                                .leaf_rect(
                                                    area_id,
                                                    &ed.panels
                                                        .layout
                                                        .leaf_rects(window.viewport_size()),
                                                )
                                                .map(|rect| start_pos - rect.y)
                                                .unwrap_or(start_pos);
                                            splitype_splitter::interaction::start_splitter_drag(
                                                &mut session.splitter,
                                                split_id,
                                                Axis::Vertical,
                                                local_start,
                                                r,
                                            );
                                            cx.notify();
                                        }
                                    });
                                })
                                .on_mouse_down(
                                    MouseButton::Right,
                                    move |event, _window, cx| {
                                        let pos = event.position;
                                        let _ = menu_editor.update(cx, |ed, cx| {
                                            if let Some(session) =
                                                ed.editor_sessions.get_mut(&area_id)
                                            {
                                                splitype_splitter::interaction::open_border_menu(
                                                    &mut session.splitter,
                                                    split_id,
                                                    dir,
                                                    pos,
                                                );
                                                cx.notify();
                                            }
                                        });
                                    },
                                ),
                            )
                            .into_any_element()
                    }
                }
            }
        }
    }
    pub(crate) fn render_editor_inner_panel_dropdown_menu(
        &mut self,
        area_id: usize,
        panel_id: usize,
        current_kind: crate::editor::session::EditorInnerPanelKind,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let available_types = EditingPanelKind::all();

        menu_panel(c, d)
            .id(("inner-area-dropdown-overlay", panel_id))
            .absolute()
            .occlude()
            .left(px(0.0))
            // Anchor to the bottom of the editor area (directly above the
            // status bar) and grow upward, so the menu never overflows past
            // the window bottom edge.
            .bottom(px(0.0))
            .w(px(d.menu_panel_width))
            .children(available_types.iter().enumerate().map(|(idx, kind)| {
                let kind = *kind;
                let is_current = kind == current_kind.editing_kind();
                let option_editor = editor.clone();
                div()
                    .id(("inner-area-type-opt", idx))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_current {
                        c.panel_row_selected
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.panel_row_hover))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(div().child(kind.name()))
                    .child(if is_current {
                        svg()
                            .path("icons/editor/bottombar/checkmark.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.change_editor_inner_panel_kind(area_id, panel_id, kind);
                            cx.notify();
                        });
                    })
            }))
            .into_any_element()
    }
}
