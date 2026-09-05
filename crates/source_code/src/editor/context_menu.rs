//! Context menu for the source code pane: state, lifecycle, rendering, and
//! action dispatch.
//!
//! Mirrors the pre-modularization design (commit f8c707c): one full-pane
//! overlay, a main panel with clipboard items and three submenus (text
//! format / paragraph settings / insert), hover-driven submenu switching
//! with a bridge element, and actions that insert plain Markdown text at
//! the caret.

use gpui::{
    AnyElement, ClipboardItem, Context, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Point, Size, StatefulInteractiveElement, Styled, Window, div, px, svg,
};
use theme::Theme;
use ui::menu_item::menu_item;
use ui::popover::{menu_panel, overlay};

use crate::editor::SourceCodeEditor;

/// Active secondary submenu in the source code context menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceContextSubmenu {
    TextFormat,
    ParagraphSettings,
    Insert,
}

/// Context menu currently open in the source code pane.
#[derive(Clone, Debug)]
pub struct SourceContextMenu {
    pub position: Point<Pixels>,
    pub active_submenu: Option<SourceContextSubmenu>,
}

type MenuAction =
    Box<dyn Fn(&mut SourceCodeEditor, &mut Window, &mut Context<SourceCodeEditor>) + 'static>;

/// Inline markup formatting variants and their Markdown templates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InlineFormat {
    Bold,
    Italic,
    Strikethrough,
    Highlight,
    InlineCode,
    InlineMath,
    Comment,
    ClearFormat,
}

impl InlineFormat {
    /// Markup template for empty caret insertion: (template, caret offset
    /// inside the template).
    fn empty_template(&self) -> (&'static str, usize) {
        match self {
            Self::Bold => ("****", 2),
            Self::Italic => ("**", 1),
            Self::Strikethrough => ("~~~~", 2),
            Self::Highlight => ("====", 2),
            Self::InlineCode => ("``", 1),
            Self::InlineMath => ("$$", 1),
            Self::Comment => ("<!---->", 4),
            Self::ClearFormat => ("", 0),
        }
    }

    /// Markup wrapper delimiters for an active selection: (prefix, suffix).
    fn wrap_delimiters(&self) -> (&'static str, &'static str) {
        match self {
            Self::Bold => ("**", "**"),
            Self::Italic => ("*", "*"),
            Self::Strikethrough => ("~~", "~~"),
            Self::Highlight => ("==", "=="),
            Self::InlineCode => ("`", "`"),
            Self::InlineMath => ("$", "$"),
            Self::Comment => ("<!--", "-->"),
            Self::ClearFormat => ("", ""),
        }
    }
}

/// Block-level structure kinds and their Markdown line prefixes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StructureKind {
    Heading(u8),
    Paragraph,
    BulletList,
    NumberedList,
    TaskList,
    Blockquote,
}

impl StructureKind {
    /// Source-mode Markdown line prefix.
    fn source_prefix(&self) -> &'static str {
        match self {
            Self::Heading(1) => "# ",
            Self::Heading(2) => "## ",
            Self::Heading(3) => "### ",
            Self::Heading(4) => "#### ",
            Self::Heading(5) => "##### ",
            Self::Heading(_) => "###### ",
            Self::Paragraph => "",
            Self::BulletList => "- ",
            Self::NumberedList => "1. ",
            Self::TaskList => "- [ ] ",
            Self::Blockquote => "> ",
        }
    }
}

/// Structural block insertions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InsertKind {
    Footnote,
    Callout,
    ThematicBreak,
    CodeBlock,
    MathBlock,
    Mermaid,
    Table,
}

/// Strips a leading Markdown block prefix (`#`, `-`, `>`, `1.`, task
/// markers) from a line, keeping the indentation.
pub fn strip_markdown_line_prefix(line: &str) -> &str {
    let trimmed = line.trim_start();
    let rest = [
        "###### ", "##### ", "#### ", "### ", "## ", "# ", "- [ ] ", "- [x] ", "- [X] ", "- ",
        "* ", "> ",
    ]
    .into_iter()
    .find_map(|prefix| trimmed.strip_prefix(prefix));
    if let Some(rest) = rest {
        return rest;
    }
    if let Some(idx) = trimmed.find(". ") {
        if !trimmed[..idx].is_empty() && trimmed[..idx].chars().all(|c| c.is_ascii_digit()) {
            return &trimmed[idx + 2..];
        }
    }
    line
}

impl SourceCodeEditor {
    // ── Lifecycle ─────────────────────────────────────────────────────────

    /// Opens the context menu at a pointer position. The caret moves to the
    /// clicked offset unless the click lands inside the current selection
    /// (so copy/cut act on it), mirroring Zed's context-menu behavior.
    pub fn open_context_menu(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(offset) = self.hit_test(position, window, cx) {
            let inside_selection = self
                .selections
                .primary_range()
                .is_some_and(|range| range.contains(&offset));
            if !inside_selection {
                self.move_to(offset, false);
            }
            self.start_cursor_blink();
        }
        self.context_menu = Some(SourceContextMenu {
            position,
            active_submenu: None,
        });
        cx.notify();
    }

    pub fn close_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.take().is_some() {
            cx.notify();
        }
    }

    pub fn set_context_menu_submenu(
        &mut self,
        submenu: Option<SourceContextSubmenu>,
        cx: &mut Context<Self>,
    ) {
        if let Some(menu) = self.context_menu.as_mut() {
            if menu.active_submenu != submenu {
                menu.active_submenu = submenu;
                cx.notify();
            }
        }
    }

    // ── Clipboard actions ─────────────────────────────────────────────────

    fn menu_cut(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.selected_text() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        let cursor_before = self.cursor_hint();
        self.delete_selection_local();
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.commit_local_edit(false, cursor_before, cx);
    }

    fn menu_copy(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn menu_paste(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let Some(text) = item.text() else {
            return;
        };
        self.insert_text_commit(&text.replace("\r\n", "\n"), cx);
    }

    fn menu_select_all(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.select_all(cx);
    }

    // ── Inline formatting ─────────────────────────────────────────────────

    /// Wraps the selection in Markdown delimiters, or inserts an empty
    /// template at the caret (caret inside the template).
    fn apply_inline_format(&mut self, kind: InlineFormat, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        if kind == InlineFormat::ClearFormat {
            if let Some(range) = self.selections.primary_range() {
                let selected = self.text.slice_owned(range.clone());
                let plain = selected
                    .trim_matches(|c| matches!(c, '*' | '_' | '~' | '`' | '=' | '$'))
                    .to_string();
                self.replace_local(range.clone(), &plain);
                self.record_edit_run(false, cursor_before, None);
                self.after_text_change();
                self.schedule_highlight(cx);
                self.selections.set_single_point(range.start + plain.len());
                self.commit_local_edit(false, cursor_before, cx);
            }
            return;
        }
        let (prefix, suffix) = kind.wrap_delimiters();
        let (template, caret_in_template) = kind.empty_template();
        match self.selections.primary_range() {
            Some(range) => {
                let text = self.text.slice_owned(range.clone());
                let wrapped = format!("{prefix}{text}{suffix}");
                self.replace_local(range.clone(), &wrapped);
                self.record_edit_run(false, cursor_before, None);
                self.after_text_change();
                self.schedule_highlight(cx);
                let inner_start = range.start + prefix.len();
                self.selections
                    .set_single_range(inner_start, inner_start + text.len());
                self.commit_local_edit(false, cursor_before, cx);
            }
            None => {
                let caret = self.cursor();
                self.replace_local(caret..caret, template);
                self.record_edit_run(false, cursor_before, None);
                self.after_text_change();
                self.schedule_highlight(cx);
                self.selections.set_single_point(caret + caret_in_template);
                self.commit_local_edit(false, cursor_before, cx);
            }
        }
    }

    // ── Block structure ───────────────────────────────────────────────────

    /// Replaces the cursor line's Markdown prefix (heading level, list
    /// marker, quote) with the kind's prefix, keeping the content.
    fn apply_line_prefix(&mut self, kind: StructureKind, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        let (row, _) = self.point_of(self.cursor());
        let start = self.line_start_offset(row);
        let end = self.line_end_offset(row);
        let stripped = strip_markdown_line_prefix(self.line_str(row));
        let prefix = kind.source_prefix();
        let new_line = format!("{prefix}{stripped}");
        let prefix_len = prefix.len();
        self.replace_local(start..end, &new_line);
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections.set_single_point(start + prefix_len);
        self.commit_local_edit(false, cursor_before, cx);
    }

    // ── Block insertion ───────────────────────────────────────────────────

    /// Inserts a Markdown snippet at the caret (replacing the selection),
    /// placing the caret at `caret_offset` inside the snippet.
    fn insert_snippet(&mut self, snippet: &str, caret_offset: usize, cx: &mut Context<Self>) {
        let cursor_before = self.cursor_hint();
        let range = self
            .selections
            .primary_range()
            .unwrap_or_else(|| self.cursor()..self.cursor());
        let offset = caret_offset.min(snippet.len());
        self.replace_local(range.clone(), snippet);
        self.record_edit_run(false, cursor_before, None);
        self.after_text_change();
        self.schedule_highlight(cx);
        self.selections.set_single_point(range.start + offset);
        self.commit_local_edit(false, cursor_before, cx);
    }

    fn insert_kind(&mut self, kind: InsertKind, cx: &mut Context<Self>) {
        match kind {
            InsertKind::Footnote => self.insert_snippet("[^1]", 3, cx),
            InsertKind::Callout => self.insert_snippet("> [!]", 4, cx),
            InsertKind::ThematicBreak => self.insert_snippet("---", 3, cx),
            InsertKind::CodeBlock => self.insert_snippet("``````", 3, cx),
            InsertKind::MathBlock => self.insert_snippet("$$$$", 2, cx),
            InsertKind::Mermaid => self.insert_snippet("```mermaid```", 10, cx),
            InsertKind::Table => self.insert_snippet(
                "\n|  |  |  |\n| --- | --- | --- |\n|  |  |  |\n|  |  |  |\n|  |  |  |\n",
                3,
                cx,
            ),
        }
    }

    /// Heading/list state of the cursor line, for the paragraph-settings
    /// checkmarks.
    fn cursor_line_structure(&self) -> (bool, bool, bool, bool, bool, bool, bool) {
        let (row, _) = self.point_of(self.cursor());
        let line = self.line_str(row).trim_start();
        let h1 = line.starts_with("# ");
        let h2 = line.starts_with("## ");
        let h3 = line.starts_with("### ");
        let h4 = line.starts_with("#### ");
        let h5 = line.starts_with("##### ");
        let h6 = line.starts_with("###### ");
        let p = !h1 && !h2 && !h3 && !h4 && !h5 && !h6;
        (h1, h2, h3, h4, h5, h6, p)
    }

    // ── Rendering ─────────────────────────────────────────────────────────

    /// Renders the open context menu inside the pane, positioned relative
    /// to `origin` (the scroll container's bounds origin).
    pub fn render_context_menu(
        &self,
        menu: &SourceContextMenu,
        origin: Point<Pixels>,
        pane_size: Size<Pixels>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;

        let is_zh = cx.has_global::<config::language::I18nManager>()
            && cx
                .global::<config::language::I18nManager>()
                .current_language_id()
                .starts_with("zh");
        let tr = |zh: &'static str, en: &'static str| if is_zh { zh } else { en };

        let pane_width = f32::from(pane_size.width);
        let pane_height = f32::from(pane_size.height);
        let panel_width = 185.0_f32;
        let panel_x = menu.position.x - origin.x;
        let panel_y = menu.position.y - origin.y;
        let max_x = (pane_width - panel_width - 16.0).max(8.0);
        let max_y = (pane_height - 290.0 - 16.0).max(8.0);
        let panel_left = px(f32::from(panel_x).clamp(8.0, max_x));
        let panel_top = px(f32::from(panel_y).clamp(8.0, max_y));

        let has_selection = self.selections.has_selection();

        let make_separator = || {
            div()
                .mx(px(d.menu_separator_margin_x))
                .my(px(d.menu_separator_margin_y))
                .h(px(1.0))
                .bg(c.dialog_border)
                .into_any_element()
        };

        // Items close any open submenu on hover (hovering the main menu
        // rows navigates back out of a submenu).
        let make_item = |id: &'static str,
                         label: &'static str,
                         shortcut: Option<&'static str>,
                         enabled: bool,
                         danger: bool,
                         on_click: MenuAction| {
            if enabled {
                let mut el = menu_item(id, c, d)
                    .justify_between()
                    .text_size(px(t.text_size * 0.85))
                    .text_color(if danger {
                        c.dialog_danger_button_bg
                    } else {
                        c.text_default
                    })
                    .child(label)
                    .children(shortcut.map(|s| {
                        div()
                            .text_size(px(t.text_size * 0.75))
                            .text_color(c.dialog_muted)
                            .child(s)
                    }));
                el = el.on_hover(cx.listener(|this, hovered, _window, cx| {
                    if *hovered {
                        this.set_context_menu_submenu(None, cx);
                    }
                }));
                el.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, window, cx| {
                        cx.stop_propagation();
                        on_click(this, window, cx);
                        this.close_context_menu(cx);
                    }),
                )
                .into_any_element()
            } else {
                menu_item(id, c, d)
                    .justify_between()
                    .text_size(px(t.text_size * 0.85))
                    .text_color(c.dialog_muted)
                    .child(label)
                    .children(shortcut.map(|s| {
                        div()
                            .text_size(px(t.text_size * 0.75))
                            .text_color(c.dialog_muted)
                            .child(s)
                    }))
                    .into_any_element()
            }
        };

        let make_check_item = |id: &'static str,
                               label: &'static str,
                               shortcut: Option<&'static str>,
                               is_checked: bool,
                               on_click: MenuAction| {
            menu_item(id, c, d)
                .justify_between()
                .text_size(px(t.text_size * 0.85))
                .text_color(c.text_default)
                .child(label)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .children(shortcut.map(|s| {
                            div()
                                .text_size(px(t.text_size * 0.75))
                                .text_color(c.dialog_muted)
                                .child(s)
                        }))
                        .children(is_checked.then(|| {
                            svg()
                                .path("icons/source_code/checkmark.svg")
                                .size(px(14.0))
                                .text_color(c.dialog_primary_button_bg)
                        })),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _event, window, cx| {
                        cx.stop_propagation();
                        on_click(this, window, cx);
                        this.close_context_menu(cx);
                    }),
                )
                .into_any_element()
        };

        let make_submenu_trigger = |id: &'static str,
                                    label: &'static str,
                                    submenu_kind: SourceContextSubmenu,
                                    is_active: bool| {
            menu_item(id, c, d)
                .justify_between()
                .bg(if is_active {
                    c.panel_row_hover
                } else {
                    c.dialog_surface
                })
                .text_size(px(t.text_size * 0.85))
                .text_color(c.text_default)
                .child(label)
                .child(
                    svg()
                        .path("icons/source_code/chevron-right.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_hover(cx.listener(move |this, hovered, _window, cx| {
                    if *hovered {
                        this.set_context_menu_submenu(Some(submenu_kind), cx);
                    }
                }))
                .into_any_element()
        };

        let (h1, h2, h3, h4, h5, h6, is_p) = self.cursor_line_structure();

        let submenu_rendered = menu.active_submenu.map(|sub| {
            let (items, y_offset) = match sub {
                SourceContextSubmenu::TextFormat => (
                    vec![
                        make_item(
                            "source-menu-fmt-bold",
                            tr("粗体", "Bold"),
                            Some("Ctrl+B"),
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::Bold, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-fmt-italic",
                            tr("斜体", "Italic"),
                            Some("Ctrl+I"),
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::Italic, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-fmt-strike",
                            tr("删除线", "Strikethrough"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::Strikethrough, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-fmt-highlight",
                            tr("高亮", "Highlight"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::Highlight, cx)
                            }),
                        ),
                        make_separator(),
                        make_item(
                            "source-menu-fmt-code",
                            tr("行内代码", "Inline Code"),
                            Some("Ctrl+E"),
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::InlineCode, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-fmt-math",
                            tr("行内公式", "Inline Math"),
                            Some("Ctrl+M"),
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::InlineMath, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-fmt-comment",
                            tr("注释", "Comment"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::Comment, cx)
                            }),
                        ),
                        make_separator(),
                        make_item(
                            "source-menu-fmt-clear",
                            tr("清除格式", "Clear Format"),
                            None,
                            has_selection,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_inline_format(InlineFormat::ClearFormat, cx)
                            }),
                        ),
                    ],
                    px(110.0),
                ),
                SourceContextSubmenu::ParagraphSettings => (
                    vec![
                        make_item(
                            "source-menu-para-bullet",
                            tr("无序列表", "Bullet List"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::BulletList, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-para-numbered",
                            tr("有序列表", "Numbered List"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::NumberedList, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-para-task",
                            tr("任务列表", "Task List"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::TaskList, cx)
                            }),
                        ),
                        make_separator(),
                        make_check_item(
                            "source-menu-para-h1",
                            tr("标题 1", "Heading 1"),
                            Some("Ctrl+1"),
                            h1,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(1), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-h2",
                            tr("标题 2", "Heading 2"),
                            Some("Ctrl+2"),
                            h2,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(2), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-h3",
                            tr("标题 3", "Heading 3"),
                            Some("Ctrl+3"),
                            h3,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(3), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-h4",
                            tr("标题 4", "Heading 4"),
                            None,
                            h4,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(4), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-h5",
                            tr("标题 5", "Heading 5"),
                            None,
                            h5,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(5), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-h6",
                            tr("标题 6", "Heading 6"),
                            None,
                            h6,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Heading(6), cx)
                            }),
                        ),
                        make_check_item(
                            "source-menu-para-p",
                            tr("正文", "Paragraph"),
                            Some("Ctrl+0"),
                            is_p,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Paragraph, cx)
                            }),
                        ),
                        make_separator(),
                        make_item(
                            "source-menu-para-quote",
                            tr("引用", "Quote"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.apply_line_prefix(StructureKind::Blockquote, cx)
                            }),
                        ),
                    ],
                    px(140.0),
                ),
                SourceContextSubmenu::Insert => (
                    vec![
                        make_item(
                            "source-menu-ins-footnote",
                            tr("脚注", "Footnote"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.insert_kind(InsertKind::Footnote, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-ins-table",
                            tr("表格", "Table"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| this.insert_kind(InsertKind::Table, cx)),
                        ),
                        make_item(
                            "source-menu-ins-callout",
                            tr("提示块", "Callout"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| this.insert_kind(InsertKind::Callout, cx)),
                        ),
                        make_item(
                            "source-menu-ins-break",
                            tr("分割线", "Thematic Break"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.insert_kind(InsertKind::ThematicBreak, cx)
                            }),
                        ),
                        make_separator(),
                        make_item(
                            "source-menu-ins-code",
                            tr("代码块", "Code Block"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.insert_kind(InsertKind::CodeBlock, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-ins-math",
                            tr("公式块", "Math Block"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| {
                                this.insert_kind(InsertKind::MathBlock, cx)
                            }),
                        ),
                        make_item(
                            "source-menu-ins-mermaid",
                            tr("Mermaid 图表", "Mermaid"),
                            None,
                            true,
                            false,
                            Box::new(|this, _window, cx| this.insert_kind(InsertKind::Mermaid, cx)),
                        ),
                    ],
                    px(168.0),
                ),
            };

            let submenu_width = 175.0_f32;
            let is_overflowing_right =
                f32::from(panel_left) + panel_width + submenu_width + 16.0 > pane_width;
            let submenu_left = if is_overflowing_right {
                (panel_left - px(submenu_width) - px(d.context_menu_submenu_gap.max(4.0)))
                    .max(px(8.0))
            } else {
                panel_left + px(panel_width) + px(d.context_menu_submenu_gap.max(4.0))
            };
            let submenu_top = (panel_top + y_offset)
                .max(px(8.0))
                .min(px((pane_height - 350.0).max(8.0)));

            let bridge_left = if is_overflowing_right {
                panel_left - px(d.context_menu_submenu_gap.max(4.0) + 4.0)
            } else {
                panel_left + px(panel_width) - px(4.0)
            };
            let bridge_top = (panel_top.min(submenu_top) - px(8.0)).max(px(0.0));
            let bridge_height =
                (panel_top.max(submenu_top) - bridge_top + px(350.0)).max(px(320.0));
            let bridge_el = div()
                .id("source-context-menu-submenu-bridge")
                .absolute()
                .left(bridge_left)
                .top(bridge_top)
                .w(px(d.context_menu_submenu_gap.max(4.0) + 8.0))
                .h(bridge_height)
                .occlude()
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation()
                });

            let panel_el = menu_panel(c, d)
                .id("source-context-menu-submenu")
                .absolute()
                .left(submenu_left)
                .top(submenu_top)
                .w(px(submenu_width))
                .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                    cx.stop_propagation()
                })
                .children(items);

            (panel_el, bridge_el)
        });

        let main_items = vec![
            make_item(
                "source-context-menu-cut",
                tr("剪切", "Cut"),
                Some("Ctrl+X"),
                has_selection,
                false,
                Box::new(|this, window, cx| this.menu_cut(window, cx)),
            ),
            make_item(
                "source-context-menu-copy",
                tr("复制", "Copy"),
                Some("Ctrl+C"),
                has_selection,
                false,
                Box::new(|this, window, cx| this.menu_copy(window, cx)),
            ),
            make_item(
                "source-context-menu-paste",
                tr("粘贴", "Paste"),
                Some("Ctrl+V"),
                true,
                false,
                Box::new(|this, window, cx| this.menu_paste(window, cx)),
            ),
            make_item(
                "source-context-menu-select-all",
                tr("全选", "Select All"),
                Some("Ctrl+A"),
                true,
                false,
                Box::new(|this, window, cx| this.menu_select_all(window, cx)),
            ),
            make_separator(),
            make_submenu_trigger(
                "source-context-menu-text-format",
                tr("文本格式", "Text Format"),
                SourceContextSubmenu::TextFormat,
                menu.active_submenu == Some(SourceContextSubmenu::TextFormat),
            ),
            make_submenu_trigger(
                "source-context-menu-paragraph-settings",
                tr("段落设置", "Paragraph Settings"),
                SourceContextSubmenu::ParagraphSettings,
                menu.active_submenu == Some(SourceContextSubmenu::ParagraphSettings),
            ),
            make_submenu_trigger(
                "source-context-menu-insert",
                tr("插入", "Insert"),
                SourceContextSubmenu::Insert,
                menu.active_submenu == Some(SourceContextSubmenu::Insert),
            ),
        ];

        let mut container = overlay()
            .id("source-context-menu-overlay")
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _event, _window, cx| {
                    this.close_context_menu(cx);
                }),
            )
            .child(
                menu_panel(c, d)
                    .id("source-context-menu-panel")
                    .absolute()
                    .left(panel_left)
                    .top(panel_top)
                    .w(px(panel_width))
                    .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                        cx.stop_propagation()
                    })
                    .children(main_items),
            );

        if let Some((sub_panel, bridge)) = submenu_rendered {
            container = container.child(bridge).child(sub_panel);
        }

        container.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::strip_markdown_line_prefix;

    #[test]
    fn strips_markdown_line_prefixes() {
        assert_eq!(strip_markdown_line_prefix("# Heading 1"), "Heading 1");
        assert_eq!(strip_markdown_line_prefix("### Heading 3"), "Heading 3");
        assert_eq!(strip_markdown_line_prefix("- Bullet item"), "Bullet item");
        assert_eq!(
            strip_markdown_line_prefix("1. Numbered item"),
            "Numbered item"
        );
        assert_eq!(strip_markdown_line_prefix("- [ ] Task item"), "Task item");
        assert_eq!(strip_markdown_line_prefix("- [x] Completed"), "Completed");
        assert_eq!(strip_markdown_line_prefix("> Quote line"), "Quote line");
        assert_eq!(strip_markdown_line_prefix("Plain line"), "Plain line");
    }
}
