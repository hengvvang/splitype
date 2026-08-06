//! Editor inner panel layout — rendering for the `EditorInnerPanelKind`
//! split tree (welcome panel + Wysiwyg / Source Code / Preview / Outline
//! editing panels) inside each Edit area.
//!
//! The layout engine (tree, sessions, operations) lives in `crate::layout`;
//! the window-level area layout rendering lives in `crate::windows::layout`.

use crate::layout::{
    Axis, BorderMenuState, CornerDragModifier, EditingPanelKind, EditorInnerPanelKind,
    InnerPanelLocation, SplitTree, SplitterDragSession, WelcomePanelKind,
};
use crate::ui::components::popover::menu_panel;
use crate::ui::components::splitter::{splitter_bar_h, splitter_bar_v};

use gpui::*;

use crate::editor::controller::*;
use crate::infra::i18n::I18nStrings;
use crate::theme::Theme;

impl Editor {
    /// Render one Editor area's inner panel layout. While building, the
    /// routing hint (`current_tab_area`) points at this area so every panel
    /// reads THIS editor's tab list.
    pub(crate) fn render_editor_inner_panel_container(
        &mut self,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        self.tab_list_mut_for(area_id);
        let inner_tree = self
            .panels
            .layout
            .ensure_editor_session(area_id)
            .inner_panel_tree
            .clone();

        // Drop runtimes of panels that were closed or joined.
        self.source_panel_runtimes
            .retain(|panel_id, _| inner_tree.contains_leaf(*panel_id));

        let previous = self.current_tab_area;
        self.current_tab_area = Some(area_id);

        // Derive the focused panel from the keyboard focus when nothing is
        // focused yet — clicking inside a block or Tab navigation never
        // reaches the panel div. Explicit clicks take precedence. Only runs
        // for editing areas: a welcome area has no tabs, hence no edit
        // targets to derive from.
        if self.panels.layout.focused_editor_inner_panel.is_none()
            && self.area_mode(area_id).is_editing()
            && let Some(target_id) = self.focused_edit_target_entity_id(window, cx)
        {
            if let Some((panel_id, _)) = self.source_panel_runtimes.iter().find(|(_, runtime)| {
                runtime.area_id == area_id
                    && runtime
                        .block
                        .as_ref()
                        .is_some_and(|block| block.entity_id() == target_id)
            }) {
                // Keyboard focus sits in a source panel's own block.
                if inner_tree.contains_leaf(*panel_id) {
                    self.panels.layout.focused_editor_inner_panel =
                        Some(InnerPanelLocation { area_id, panel_id: *panel_id });
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
                    self.panels.layout.focused_editor_inner_panel =
                        Some(InnerPanelLocation { area_id, panel_id });
                }
            }
        }

        let inner_rendered = self.render_editor_inner_panel_node(
            &inner_tree,
            area_id,
            theme,
            strings,
            window,
            cx,
        );
        self.current_tab_area = previous;

        let dropdown = if let Some(loc) = self.panels.layout.open_editor_inner_panel_dropdown {
            if loc.area_id == area_id {
                let current_type = self
                    .panels
                    .layout
                    .ensure_editor_session(area_id)
                    .inner_panel_tree
                    .find_leaf_kind(loc.panel_id)
                    .unwrap_or(EditorInnerPanelKind::Welcome(WelcomePanelKind::Welcome(
                        None,
                    )));
                Some(self.render_editor_inner_panel_dropdown_menu(
                    area_id,
                    loc.panel_id,
                    current_type,
                    theme,
                    cx,
                ))
            } else {
                None
            }
        } else {
            None
        };

        let mut container = div()
            .w_full()
            .h_full()
            .relative()
            .p(px(2.0))
            .bg(c.editor_background)
            .child(inner_rendered);

        if let Some(dropdown) = dropdown {
            container = container.child(dropdown);
        }

        container.into_any_element()
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
                    let is_double = ed.chrome.welcome_last_click.is_some_and(|previous| {
                        now.duration_since(previous) < std::time::Duration::from_millis(500)
                    });
                    ed.chrome.welcome_last_click = Some(now);
                    if is_double {
                        // The clicked editor becomes the active editor and
                        // its routing context is set for the tab creation.
                        ed.panels.layout.activate_editor_area(area_id);
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
        node: &crate::layout::SplitTree<crate::layout::EditorInnerPanelKind>,
        area_id: usize,
        theme: &Theme,
        strings: &I18nStrings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;

        match node {
            SplitTree::Leaf {
                id: panel_id,
                kind,
            } => {
                let panel_id = *panel_id;
                let kind = *kind;
                let inner_editor = cx.entity().downgrade();

                // The panel kind carries the mode: welcome panels render
                // the guidance prompt, editing panels render their view.
                let inner_body: AnyElement = match kind {
                    EditorInnerPanelKind::Welcome(_) => {
                        self.render_welcome_prompt(area_id, panel_id, theme, cx)
                    }
                    EditorInnerPanelKind::Editing(kind) => match kind {
                        // WYSIWYG — this editor's own block editor view.
                        EditingPanelKind::Wysiwyg => self.render_document_view(
                            area_id,
                            panel_id,
                            window,
                            cx,
                        ),
                        // Source — interactive source code editor. Uses a
                        // cached block in source-document mode; edits sync to
                        // the shared document via the block's Changed event.
                        EditingPanelKind::SourceCode => {
                            self.refresh_source_panel_block(area_id, panel_id, cx);
                            self.render_source_editor_panel(area_id, panel_id, theme, cx)
                        }
                        EditingPanelKind::Preview => self.render_tiled_preview_panel(
                            area_id,
                            panel_id,
                            theme,
                            strings,
                            window,
                            cx,
                        ),
                        EditingPanelKind::Outline => self.render_tiled_outline_panel(
                            area_id,
                            panel_id,
                            theme,
                            strings,
                            cx,
                        ),
                    },
                };

                let make_inner_corner = |id_str: &'static str, top: bool, left: bool| {
                    let inner_editor = inner_editor.clone();
                    let mut corner_div = div()
                        .id((id_str, panel_id))
                        .absolute()
                        .occlude()
                        .size(px(10.0))
                        .cursor_crosshair()
                        .rounded(px(4.0));

                    if top {
                        corner_div = corner_div.top(px(2.0));
                    } else {
                        corner_div = corner_div.bottom(px(2.0));
                    }
                    if left {
                        corner_div = corner_div.left(px(2.0));
                    } else {
                        corner_div = corner_div.right(px(2.0));
                    }

                    corner_div.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let pos = event.position;
                        let modifier = if event.modifiers.control {
                            CornerDragModifier::Swap
                        } else if event.modifiers.shift {
                            CornerDragModifier::Duplicate
                        } else {
                            CornerDragModifier::None
                        };
                        let _ = inner_editor.update(cx, |ed, cx| {
                            ed.panels.layout.start_editor_inner_panel_corner_drag(
                                area_id,
                                panel_id,
                                pos,
                                modifier,
                            );
                            cx.notify();
                        });
                    })
                };

                // Auto-focus first inner panel if none is focused.
                if self.panels.layout.focused_editor_inner_panel.is_none() {
                    self.panels.layout.focused_editor_inner_panel =
                        Some(InnerPanelLocation { area_id, panel_id });
                    // The auto-focused editor becomes the active editor too.
                    self.panels.layout.activate_editor_area(area_id);
                }

                let focus_editor = cx.entity().downgrade();

                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .rounded(px(d.area_tile_radius))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .shadow_lg()
                    .child(div().w_full().flex_1().min_h(px(0.0)).child(inner_body))
                    .child(make_inner_corner("edit-sub-tl", true, true))
                    .child(make_inner_corner("edit-sub-tr", true, false))
                    .child(make_inner_corner("edit-sub-bl", false, true))
                    .child(make_inner_corner("edit-sub-br", false, false))
                    .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                        let _ = focus_editor.update(cx, |ed, cx| {
                            // Select this panel and move the keyboard edit
                            // focus to its editing target (source block or
                            // the shared document block).
                            ed.focus_editor_inner_panel(area_id, panel_id, window, cx);
                            cx.notify();
                        });
                    })
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
                let first_elem = self.render_editor_inner_panel_node(
                    first,
                    area_id,
                    theme,
                    strings,
                    window,
                    cx,
                );
                let second_elem = self.render_editor_inner_panel_node(
                    second,
                    area_id,
                    theme,
                    strings,
                    window,
                    cx,
                );

                let inner_editor = cx.entity().downgrade();

                match direction {
                    Axis::Horizontal => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_row()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
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
                                splitter_bar_h(("inner-splitter-bar-h", split_id), c)
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.x);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_editor_inner_panel_splitter_drag = Some((
                                                area_id,
                                                SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Horizontal,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 1000.0,
                                                },
                                            ));
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_editor_inner_panel_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
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
                            .into_any_element()
                    }
                    Axis::Vertical => {
                        let bar_editor = inner_editor.clone();
                        let menu_editor = inner_editor.clone();
                        div()
                            .w_full()
                            .h_full()
                            .flex()
                            .flex_col()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
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
                                splitter_bar_v(("inner-splitter-bar-v", split_id), c)
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.y);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.panels.layout.active_editor_inner_panel_splitter_drag = Some((
                                                area_id,
                                                SplitterDragSession {
                                                    split_id,
                                                    direction: Axis::Vertical,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 700.0,
                                                },
                                            ));
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.panels.layout.active_editor_inner_panel_border_menu =
                                                    Some(BorderMenuState {
                                                        split_id,
                                                        direction: dir,
                                                        position: pos,
                                                    });
                                                cx.notify();
                                            });
                                        },
                                    ),
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
        current_kind: crate::layout::EditorInnerPanelKind,
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
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(div().child(kind.name()))
                    .child(if is_current {
                        svg()
                            .path("icon/panel/check.svg")
                            .size(px(13.0))
                            .text_color(c.dialog_primary_button_bg)
                            .into_any_element()
                    } else {
                        div().w(px(13.0)).into_any_element()
                    })
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.panels.layout.change_editor_inner_panel_kind(
                                area_id,
                                panel_id,
                                kind,
                            );
                            cx.notify();
                        });
                    })
            }))
            .into_any_element()
    }
}
