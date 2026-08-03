//! Editor window rendering: centered scrollable block column,
//! unsaved-changes overlay dialog, custom scrollbar, and deferred
//! operations (focus, scroll, save, window title).

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{Editor, InfoDialogKind};
use crate::app_menu::dispatch_menu_action_for_editor;
use crate::components::CalloutVariant;
use crate::components::switch::Switch;
use crate::components::{
    AddLanguageConfig, AddThemeConfig, Block, NoRecentFiles, SelectLanguage, SelectTheme,
};
use crate::i18n::{I18nManager, I18nStrings};
use crate::theme::{Theme, ThemeDimensions, ThemeManager};
use crate::window_chrome::{custom_titlebar_height, render_custom_titlebar};

pub(crate) const ABOUT_GITHUB_URL: &str = "https://github.com/manyougz/velotype";

/// Rows within this many pixels of the viewport stay mounted, so a fast flick
/// paints them before they scroll in instead of showing a blank edge.
const RENDER_OVERDRAW_PX: f32 = 800.0;

pub(crate) fn open_about_github_url(cx: &mut App) {
    cx.open_url(ABOUT_GITHUB_URL);
}

fn editor_text_font() -> Font {
    // FontFallbacks is internally `Arc<Vec<String>>` — building it once
    // per process and Arc-cloning per render is the right shape, since
    // editor_text_font() is called from Editor::render on every frame.
    static FALLBACKS: std::sync::OnceLock<FontFallbacks> = std::sync::OnceLock::new();
    let fallbacks = FALLBACKS
        .get_or_init(|| {
            FontFallbacks::from_fonts(tibetan_font_fallbacks_for_target_os(std::env::consts::OS))
        })
        .clone();
    let mut font = font(".SystemUIFont");
    font.fallbacks = Some(fallbacks);
    font
}

fn tibetan_font_fallbacks_for_target_os(target_os: &str) -> Vec<String> {
    let families = match target_os {
        "windows" => &[
            "Microsoft Himalaya",
            "Noto Serif Tibetan",
            "Noto Sans Tibetan",
            "BabelStone Tibetan",
        ][..],
        "macos" => &["Kailasa", "Noto Serif Tibetan", "Noto Sans Tibetan"][..],
        _ => &[
            "Noto Serif Tibetan",
            "Noto Sans Tibetan",
            "Microsoft Himalaya",
            "Kailasa",
            "BabelStone Tibetan",
        ][..],
    };
    families
        .iter()
        .map(|family| (*family).to_string())
        .collect()
}

/// Adjacent-row metadata used to collapse spacing inside visual groups.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RenderedRowSpacingInfo {
    quote_group_anchor: Option<uuid::Uuid>,
    visible_quote_group_anchor: Option<uuid::Uuid>,
    callout_anchor: Option<uuid::Uuid>,
    callout_variant: Option<CalloutVariant>,
    is_callout_header: bool,
    footnote_anchor: Option<uuid::Uuid>,
    is_footnote_header: bool,
}

impl RenderedRowSpacingInfo {
    fn from_block(block: &Block) -> Self {
        Self {
            quote_group_anchor: block.quote_group_anchor,
            visible_quote_group_anchor: block.visible_quote_group_anchor,
            callout_anchor: block.callout_anchor,
            callout_variant: block.callout_variant,
            is_callout_header: block.kind().is_callout(),
            footnote_anchor: block.footnote_anchor,
            is_footnote_header: block.kind().is_footnote_definition(),
        }
    }
}

fn rendered_row_top_gap(
    previous: Option<RenderedRowSpacingInfo>,
    current: RenderedRowSpacingInfo,
    default_gap: f32,
) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };

    if previous.quote_group_anchor.is_some()
        && previous.quote_group_anchor == current.quote_group_anchor
    {
        0.0
    } else {
        default_gap
    }
}

fn callout_colors(variant: CalloutVariant, theme: &Theme) -> (Hsla, Hsla) {
    let c = &theme.colors;
    match variant {
        CalloutVariant::Note => (c.callout_note_border, c.callout_note_bg),
        CalloutVariant::Tip => (c.callout_tip_border, c.callout_tip_bg),
        CalloutVariant::Important => (c.callout_important_border, c.callout_important_bg),
        CalloutVariant::Warning => (c.callout_warning_border, c.callout_warning_bg),
        CalloutVariant::Caution => (c.callout_caution_border, c.callout_caution_bg),
    }
}

fn callout_row_top_gap(
    previous: Option<RenderedRowSpacingInfo>,
    current: RenderedRowSpacingInfo,
    dimensions: &ThemeDimensions,
) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };

    if previous.visible_quote_group_anchor.is_some()
        && previous.visible_quote_group_anchor == current.visible_quote_group_anchor
    {
        return 0.0;
    }

    if previous.is_callout_header {
        dimensions.callout_header_margin_bottom
    } else {
        dimensions.callout_body_gap
    }
}

fn footnote_row_top_gap(previous: Option<RenderedRowSpacingInfo>, default_gap: f32) -> f32 {
    let Some(previous) = previous else {
        return 0.0;
    };

    if previous.is_footnote_header {
        default_gap * 0.75
    } else {
        default_gap
    }
}

fn is_wide_menu_char(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x11ff
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe6f
            | 0xff00..=0xff60
            | 0xffe0..=0xffe6
    )
}

fn estimated_menu_label_width(label: &str, text_size: f32) -> f32 {
    label
        .chars()
        .map(|ch| {
            if ch.is_ascii_whitespace() {
                text_size * 0.35
            } else if ch.is_ascii_punctuation() {
                text_size * 0.45
            } else if ch.is_ascii() {
                text_size * 0.54
            } else if is_wide_menu_char(ch) {
                text_size
            } else {
                text_size * 0.85
            }
        })
        .sum()
}

const TITLEBAR_MENU_BUTTON_PADDING_X: f32 = 5.0;
const TITLEBAR_MENU_BUTTON_GAP: f32 = 2.0;
const TITLEBAR_MENU_START_X: f32 = 32.0;

fn menu_bar_button_width(label: &str, dimensions: &ThemeDimensions) -> f32 {
    let content_width = estimated_menu_label_width(label, dimensions.menu_text_size)
        + TITLEBAR_MENU_BUTTON_PADDING_X * 2.0;
    content_width.ceil().max(20.0)
}

fn supports_in_window_menu_for_target_os(target_os: &str) -> bool {
    target_os != "macos"
}

fn supports_in_window_menu() -> bool {
    supports_in_window_menu_for_target_os(std::env::consts::OS)
}

fn in_window_menu_bar_height_for_target_os(
    _target_os: &str,
    _has_menus: bool,
    _dimensions: &ThemeDimensions,
) -> f32 {
    0.0
}

fn menu_panel_left<S: AsRef<str>>(
    open_index: usize,
    menu_labels: &[S],
    dimensions: &ThemeDimensions,
) -> f32 {
    let prior_width: f32 = menu_labels
        .iter()
        .take(open_index)
        .map(|label| menu_bar_button_width(label.as_ref(), dimensions))
        .sum();
    TITLEBAR_MENU_START_X + prior_width + TITLEBAR_MENU_BUTTON_GAP * open_index as f32
}

fn menu_panel_width_for_labels<S: AsRef<str>>(labels: &[S], dimensions: &ThemeDimensions) -> f32 {
    let widest_label = labels
        .iter()
        .map(|label| estimated_menu_label_width(label.as_ref(), dimensions.menu_text_size))
        .fold(0.0, f32::max);
    let content_width = widest_label + dimensions.menu_item_padding_x * 2.0;
    dimensions.menu_panel_width.max(content_width.ceil())
}

fn owned_menu_item_labels(items: &[OwnedMenuItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
            OwnedMenuItem::Submenu(menu) => Some(menu.name.to_string()),
            OwnedMenuItem::SystemMenu(menu) => Some(menu.name.to_string()),
            OwnedMenuItem::Separator => None,
        })
        .collect()
}

fn menu_item_visual_height(item: &OwnedMenuItem, dimensions: &ThemeDimensions) -> f32 {
    match item {
        OwnedMenuItem::Separator => {
            dimensions.menu_separator_height + dimensions.menu_separator_margin_y * 2.0
        }
        OwnedMenuItem::Action { .. } | OwnedMenuItem::Submenu(_) | OwnedMenuItem::SystemMenu(_) => {
            dimensions.menu_item_height
        }
    }
}

const SCROLLABLE_IMPORT_MENU_VISIBLE_ITEMS: usize = 12;

fn menu_items_visual_height_with_gaps(
    items: &[OwnedMenuItem],
    dimensions: &ThemeDimensions,
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }

    let items_height: f32 = items
        .iter()
        .map(|item| menu_item_visual_height(item, dimensions))
        .sum();
    items_height + dimensions.menu_panel_gap * items.len().saturating_sub(1) as f32
}

fn import_menu_split_index(items: &[OwnedMenuItem]) -> Option<usize> {
    let [
        prefix @ ..,
        OwnedMenuItem::Separator,
        OwnedMenuItem::Action { action, .. },
    ] = items
    else {
        return None;
    };

    if action.as_ref().as_any().is::<AddThemeConfig>()
        || action.as_ref().as_any().is::<AddLanguageConfig>()
    {
        Some(prefix.len())
    } else {
        None
    }
}

fn scrollable_import_menu_scroll_height(
    scroll_items: &[OwnedMenuItem],
    footer_items: &[OwnedMenuItem],
    viewport_height: f32,
    top_offset: f32,
    dimensions: &ThemeDimensions,
) -> f32 {
    let visible_count = scroll_items.len().min(SCROLLABLE_IMPORT_MENU_VISIBLE_ITEMS);
    if visible_count == 0 {
        return 0.0;
    }

    let default_height =
        menu_items_visual_height_with_gaps(&scroll_items[..visible_count], dimensions);
    let footer_height = menu_items_visual_height_with_gaps(footer_items, dimensions);
    let footer_gap = if footer_items.is_empty() {
        0.0
    } else {
        dimensions.menu_panel_gap
    };
    let available_height = viewport_height
        - top_offset
        - dimensions.menu_panel_top
        - dimensions.menu_panel_padding * 2.0
        - footer_height
        - footer_gap
        - 8.0;
    let min_height = dimensions.menu_item_height.min(default_height).max(1.0);

    default_height.min(available_height.max(min_height))
}

fn submenu_panel_top(
    items: &[OwnedMenuItem],
    item_index: usize,
    dimensions: &ThemeDimensions,
) -> f32 {
    let prior_items_height: f32 = items
        .iter()
        .take(item_index)
        .map(|item| menu_item_visual_height(item, dimensions))
        .sum();
    let prior_gaps = dimensions.menu_panel_gap * item_index as f32;
    dimensions.menu_panel_top + dimensions.menu_panel_padding + prior_items_height + prior_gaps
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MenuSubmenuBridgeGeometry {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

fn submenu_bridge_geometry<S: AsRef<str>, T: AsRef<str>>(
    open_index: usize,
    menu_labels: &[S],
    items: &[OwnedMenuItem],
    item_index: usize,
    submenu_labels: &[T],
    dimensions: &ThemeDimensions,
) -> Option<MenuSubmenuBridgeGeometry> {
    let item = items.get(item_index)?;
    let main_panel_left = menu_panel_left(open_index, menu_labels, dimensions);
    let main_panel_width = menu_panel_width_for_labels(&owned_menu_item_labels(items), dimensions);
    let submenu_width = menu_panel_width_for_labels(submenu_labels, dimensions);
    let vertical_tolerance = dimensions.menu_panel_padding + dimensions.menu_panel_gap;
    let item_top = submenu_panel_top(items, item_index, dimensions);
    let top = (item_top - vertical_tolerance).max(dimensions.menu_panel_top);
    Some(MenuSubmenuBridgeGeometry {
        left: main_panel_left + main_panel_width,
        top,
        width: dimensions.menu_panel_gap + submenu_width,
        height: menu_item_visual_height(item, dimensions) + vertical_tolerance * 2.0,
    })
}

fn footnote_group_shell(
    children: Vec<AnyElement>,
    theme: &Theme,
    dimensions: &ThemeDimensions,
) -> AnyElement {
    div()
        .w_full()
        .flex_shrink_0()
        .flex()
        .flex_col()
        .gap(px(0.0))
        .px(px(dimensions.footnote_padding_x))
        .py(px(dimensions.footnote_padding_y))
        .rounded(px(dimensions.footnote_radius))
        .border(px(1.0))
        .border_color(theme.colors.footnote_border)
        .bg(theme.colors.footnote_bg)
        .children(children)
        .into_any_element()
}

impl Editor {
    fn on_titlebar_close(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.standard_click() {
            self.request_close_current_window(window, cx);
        }
    }

    pub(crate) fn install_close_guard(&mut self, cx: &mut Context<Self>, window: &mut Window) {
        if self.close_guard_installed {
            return;
        }

        self.force_install_close_guard(cx, window);
    }

    pub(crate) fn force_install_close_guard(
        &mut self,
        cx: &mut Context<Self>,
        window: &mut Window,
    ) {
        let editor = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            editor
                .update(cx, |this, cx| this.on_window_should_close(window, cx))
                .unwrap_or(true)
        });
        self.close_guard_installed = true;
    }

    fn apply_pending_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entity_id) = self.pending_focus.take()
            && let Some(block) = self.focusable_entity_by_id(entity_id)
        {
            block.read(cx).focus_handle.focus(window);
        }
    }

    fn ensure_focused_caret_visible(&mut self, window: &Window, cx: &App) -> bool {
        let Some(focused_block) = self.focused_edit_target(window, cx) else {
            return false;
        };
        let Some(active_bounds) =
            focused_block.read_with(cx, |block, _cx| block.active_range_or_cursor_bounds())
        else {
            return false;
        };

        let viewport = self.scroll_handle.bounds();
        let padding = px(20.0);
        let top_limit = viewport.top() + padding;
        let bottom_limit = viewport.bottom() - padding;
        let mut offset = self.scroll_handle.offset();
        let mut changed = false;

        if active_bounds.top() < top_limit {
            offset.y += top_limit - active_bounds.top();
            changed = true;
        } else if active_bounds.bottom() > bottom_limit {
            offset.y -= active_bounds.bottom() - bottom_limit;
            changed = true;
        }

        if changed {
            let max_offset_y = self.scroll_handle.max_offset().height.max(px(0.0));
            offset.y = offset.y.min(px(0.0)).max(-max_offset_y);
            self.scroll_handle.set_offset(offset);
        }

        true
    }

    fn apply_pending_scroll_into_view(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.scrollbar_drag.is_some() {
            return;
        }

        if !self.pending_scroll_active_block_into_view {
            return;
        }

        // scroll_to_item indexed children by position, which the spacers break;
        // the focused block is always mounted, so pixel math on its bounds works.
        let has_bounds = self.ensure_focused_caret_visible(window, cx);
        if self.pending_scroll_recheck_after_layout {
            self.pending_scroll_recheck_after_layout = false;
            self.schedule_scroll_recheck(cx);
            return;
        }

        if !has_bounds {
            self.schedule_scroll_recheck(cx);
            return;
        }

        self.pending_scroll_active_block_into_view = false;
        self.scroll_recheck_task = None;
    }

    /// Requests a repaint one frame out so a still-pending scroll-into-view can
    /// retry once the target block has been laid out. `cx.notify()` is swallowed
    /// when called from within `render`, so without this the retry would wait
    /// for the next external notify (e.g. the cursor blink, ~0.5s later).
    fn schedule_scroll_recheck(&mut self, cx: &mut Context<Self>) {
        self.scroll_recheck_task = Some(cx.spawn(async move |this: WeakEntity<Self>, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let _ = this.update(cx, |_this, cx| cx.notify());
        }));
    }

    fn sync_pending_save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_save {
            self.pending_save = false;
            self.save_document(window, cx);
        }
    }

    fn sync_pending_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.pending_save_as {
            self.pending_save_as = false;
            self.save_document_as(window, cx);
        }
    }

    fn sync_pending_open_link(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(link) = self.pending_open_link.take() else {
            return;
        };

        let strings = cx.global::<I18nManager>().strings_arc();
        let buttons = [
            strings.open_link_open.as_str(),
            strings.open_link_cancel.as_str(),
        ];
        let prompt = window.prompt(
            PromptLevel::Info,
            &strings.open_link_title,
            Some(&link.prompt_target),
            &buttons,
            cx,
        );
        let window_handle = window.window_handle();
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let Ok(choice) = prompt.await else {
                return;
            };
            if choice == 0 {
                let _ = cx.update_window(window_handle, |_view: AnyView, _window, cx| {
                    cx.open_url(&link.open_target);
                });
            }
        })
        .detach();
    }

    fn sync_window_edited_state(&mut self, window: &mut Window) {
        if self.pending_window_edited {
            self.pending_window_edited = false;
            window.set_window_edited(true);
        }
    }

    fn sync_scroll_viewport(&mut self, viewport_size: Size<Pixels>, cx: &mut Context<Self>) {
        match self.last_scroll_viewport_size {
            Some(previous) if Self::viewport_size_changed(previous, viewport_size) => {
                self.last_scroll_viewport_size = Some(viewport_size);
                self.request_active_block_scroll_into_view(cx);
            }
            Some(_) => {}
            None => {
                self.last_scroll_viewport_size = Some(viewport_size);
            }
        }
    }

    fn sync_window_title(&mut self, window: &mut Window, strings: &I18nStrings) {
        if self.pending_window_title_refresh {
            self.pending_window_title_refresh = false;
            let title = Self::window_title(self.file_path.as_deref(), self.document_dirty, strings);
            window.set_window_title(&title);
        }
    }

    fn render_inline_titlebar_menu(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        menus: Option<&[gpui::OwnedMenu]>,
        menu_labels: &[SharedString],
    ) -> Option<AnyElement> {
        let menus = menus?;
        if menus.is_empty() {
            return None;
        }

        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        let is_expanded = self.menu_bar_expanded || self.menu_bar_open.is_some();

        let mut row = div()
            .id("titlebar-menu-inline")
            .h_full()
            .flex()
            .items_center()
            .gap(px(TITLEBAR_MENU_BUTTON_GAP))
            .px(px(6.0));

        let app_button_editor = editor.clone();
        let app_button = div()
            .id("titlebar-app-icon-button")
            .size(px(22.0))
            .mr(px(2.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(if is_expanded {
                c.dialog_secondary_button_hover
            } else {
                hsla(0.0, 0.0, 0.0, 0.0)
            })
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .active(|this| this.opacity(0.88))
            .cursor_pointer()
            .child(
                div()
                    .size(px(10.0))
                    .rounded_full()
                    .border(px(1.5))
                    .border_color(if is_expanded {
                        c.dialog_title
                    } else {
                        c.dialog_secondary_button_text
                    })
                    .bg(if is_expanded {
                        c.dialog_title
                    } else {
                        hsla(0.0, 0.0, 0.0, 0.0)
                    }),
            )
            .on_click(move |_, _window, cx| {
                let _ = app_button_editor.update(cx, |ed, cx| {
                    ed.toggle_menu_bar_expanded(cx);
                });
            });

        row = row.child(app_button);

        if is_expanded && !menu_labels.is_empty() {
            let button_widths = menu_labels
                .iter()
                .map(|label| menu_bar_button_width(label, d))
                .collect::<Vec<_>>();

            for (index, label) in menu_labels.iter().enumerate() {
                let label = label.clone();
                let is_open = self.menu_bar_open == Some(index);
                let button_editor = editor.clone();
                let click_editor = editor.clone();
                let button_width = button_widths[index];

                row = row.child(
                    div()
                        .id(("app-menu-button", index))
                        .h(px(d.menu_bar_button_height))
                        .w(px(button_width))
                        .px(px(TITLEBAR_MENU_BUTTON_PADDING_X))
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .justify_center()
                        .rounded(px(d.menu_bar_button_radius))
                        .bg(if is_open {
                            c.dialog_secondary_button_hover
                        } else {
                            hsla(0.0, 0.0, 0.0, 0.0)
                        })
                        .hover(|this| this.bg(c.dialog_secondary_button_hover))
                        .active(|this| this.opacity(0.92))
                        .cursor_pointer()
                        .text_size(px(d.menu_text_size))
                        .font_weight(t.dialog_button_weight.to_font_weight())
                        .text_color(c.dialog_secondary_button_text)
                        .whitespace_nowrap()
                        .child(label)
                        .on_hover(move |hovered, _window, cx| {
                            if *hovered {
                                let _ = button_editor
                                    .update(cx, |editor, cx| editor.open_menu_bar(index, cx));
                            }
                        })
                        .on_click(move |_, _window, cx| {
                            let _ = click_editor
                                .update(cx, |editor, cx| editor.open_menu_bar(index, cx));
                        }),
                );
            }
        }

        Some(row.into_any_element())
    }

    fn render_in_window_menu_item(
        &self,
        item: OwnedMenuItem,
        item_index: usize,
        theme: &Theme,
        editor: WeakEntity<Self>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;

        match item {
            OwnedMenuItem::Separator => div()
                .id(("app-menu-separator", item_index))
                .flex_shrink_0()
                .mx(px(d.menu_separator_margin_x))
                .my(px(d.menu_separator_margin_y))
                .h(px(d.menu_separator_height))
                .bg(c.dialog_border)
                .into_any_element(),
            OwnedMenuItem::Action { name, action, .. } => {
                let is_disabled = action.as_ref().as_any().is::<NoRecentFiles>();
                let click_editor = editor.clone();
                let hover_editor = editor.clone();

                let mut is_selected = false;
                let mut left_elem: Option<AnyElement> = None;

                if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectTheme>() {
                    let current_theme_id = cx.global::<ThemeManager>().current_theme_id();
                    is_selected = act.theme_id == current_theme_id;
                    let item_icon = if name == "Light" {
                        "icon/panel/sun.svg"
                    } else {
                        "icon/panel/moon.svg"
                    };
                    left_elem = Some(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                svg()
                                    .path(item_icon)
                                    .size(px(13.0))
                                    .text_color(c.text_default),
                            )
                            .child(name.clone())
                            .into_any_element(),
                    );
                } else if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectLanguage>() {
                    let current_language_id = cx.global::<I18nManager>().current_language_id();
                    is_selected = act.language_id == current_language_id;
                }

                let is_theme_or_lang = action.as_ref().as_any().is::<SelectTheme>()
                    || action.as_ref().as_any().is::<SelectLanguage>();

                let base = div()
                    .id(("app-menu-item", item_index))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .flex_shrink_0()
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .when(is_theme_or_lang, |this| this.justify_between())
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_selected {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(if is_disabled {
                        c.dialog_muted
                    } else {
                        c.dialog_secondary_button_text
                    })
                    .child(left_elem.unwrap_or_else(|| div().child(name.clone()).into_any_element()))
                    .when(is_theme_or_lang, |this| {
                        this.child(if is_selected {
                            svg()
                                .path("icon/panel/check.svg")
                                .size(px(13.0))
                                .text_color(c.dialog_primary_button_bg)
                                .into_any_element()
                        } else {
                            div().w(px(13.0)).into_any_element()
                        })
                    })
                    .on_hover(move |hovered, _window, cx| {
                        if *hovered {
                            let _ =
                                hover_editor.update(cx, |editor, cx| editor.close_menu_submenu(cx));
                        }
                    });

                if is_disabled {
                    base.into_any_element()
                } else {
                    base.hover(|this| this.bg(c.dialog_secondary_button_hover))
                        .active(|this| this.opacity(0.92))
                        .cursor_pointer()
                        .on_click(move |_, window, cx| {
                            let _ = click_editor.update(cx, |editor, cx| editor.close_menu_bar(cx));
                            dispatch_menu_action_for_editor(
                                action.as_ref(),
                                &click_editor,
                                window,
                                cx,
                            );
                        })
                        .into_any_element()
                }
            }
            OwnedMenuItem::Submenu(submenu) => {
                let is_open = self.menu_submenu_open == Some(item_index);
                let hover_editor = editor.clone();
                div()
                    .id(("app-menu-submenu", item_index))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .flex_shrink_0()
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(d.menu_item_radius))
                    .bg(if is_open {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_body_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(submenu.name.to_string())
                    .child(
                        svg()
                            .path("icon/panel/chevron-right.svg")
                            .size(px(14.0))
                            .text_color(c.dialog_secondary_button_text),
                    )
                    .on_hover(move |hovered, _window, cx| {
                        if *hovered {
                            let _ = hover_editor
                                .update(cx, |editor, cx| editor.open_menu_submenu(item_index, cx));
                        }
                    })
                    .into_any_element()
            }
            OwnedMenuItem::SystemMenu(os_menu) => div()
                .id(("app-menu-system", item_index))
                .w_full()
                .h(px(d.menu_item_height))
                .flex_shrink_0()
                .px(px(d.menu_item_padding_x))
                .flex()
                .items_center()
                .rounded(px(d.menu_item_radius))
                .bg(c.dialog_surface)
                .text_size(px(d.menu_text_size))
                .text_color(c.dialog_muted)
                .child(os_menu.name.to_string())
                .into_any_element(),
        }
    }

    /// Renders the currently open in-window fallback menu as a floating
    /// panel. `menus` and `menu_labels` are fetched and computed once at
    /// the caller and shared with [`Self::render_in_window_menu_bar`].
    fn render_in_window_menu_panel(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
        menus: Option<&[gpui::OwnedMenu]>,
        menu_labels: &[SharedString],
        top_offset: f32,
        viewport_height: f32,
    ) -> Option<AnyElement> {
        let open_index = self.menu_bar_open?;
        let menus = menus?;
        let menu = menus.get(open_index)?.clone();
        let menu_items = menu.items.clone();
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();
        let menu_item_labels = owned_menu_item_labels(&menu_items);
        let menu_panel_width = menu_panel_width_for_labels(&menu_item_labels, d);
        let submenu_bridge = self.menu_submenu_open.and_then(|submenu_index| {
            match menu_items.get(submenu_index)? {
                OwnedMenuItem::Submenu(submenu) => {
                    let submenu_labels = owned_menu_item_labels(&submenu.items);
                    let geometry = submenu_bridge_geometry(
                        open_index,
                        menu_labels,
                        &menu_items,
                        submenu_index,
                        &submenu_labels,
                        d,
                    )?;
                    Some(
                        div()
                            .id(("app-submenu-bridge", open_index * 1000 + submenu_index))
                            .absolute()
                            .occlude()
                            .top(px(top_offset + geometry.top))
                            .left(px(geometry.left))
                            .w(px(geometry.width))
                            .h(px(geometry.height))
                            .bg(hsla(0.0, 0.0, 0.0, 0.0))
                            .on_hover(cx.listener(Self::on_menu_submenu_bridge_hover))
                            .into_any_element(),
                    )
                }
                _ => None,
            }
        });
        let submenu_panel =
            self.menu_submenu_open.and_then(|submenu_index| {
                match menu_items.get(submenu_index)? {
                    OwnedMenuItem::Submenu(submenu) => {
                        let submenu_labels = owned_menu_item_labels(&submenu.items);
                        let left = menu_panel_left(open_index, menu_labels, d)
                            + menu_panel_width
                            + d.menu_panel_gap;
                        let top = submenu_panel_top(&menu_items, submenu_index, d);
                        let submenu_width = menu_panel_width_for_labels(&submenu_labels, d);
                        let submenu_items = submenu.items.clone().into_iter().enumerate().map(
                            |(item_index, item)| match item {
                                OwnedMenuItem::Separator => div()
                                    .id((
                                        "app-submenu-separator",
                                        submenu_index * 1000 + item_index,
                                    ))
                                    .mx(px(d.menu_separator_margin_x))
                                    .my(px(d.menu_separator_margin_y))
                                    .h(px(d.menu_separator_height))
                                    .bg(c.dialog_border)
                                    .into_any_element(),
                                OwnedMenuItem::Action { name, action, .. } => {
                                    let is_disabled =
                                        action.as_ref().as_any().is::<NoRecentFiles>();
                                    let editor = editor.clone();

                                    let mut is_selected = false;
                                    let mut left_elem: Option<AnyElement> = None;

                                    if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectTheme>() {
                                        let current_theme_id = cx.global::<ThemeManager>().current_theme_id();
                                        is_selected = act.theme_id == current_theme_id;
                                        let item_icon = if name == "Light" {
                                            "icon/panel/sun.svg"
                                        } else {
                                            "icon/panel/moon.svg"
                                        };
                                        left_elem = Some(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(6.0))
                                                .child(
                                                    svg()
                                                        .path(item_icon)
                                                        .size(px(13.0))
                                                        .text_color(c.text_default),
                                                )
                                                .child(name.clone())
                                                .into_any_element(),
                                        );
                                    } else if let Some(act) = action.as_ref().as_any().downcast_ref::<SelectLanguage>() {
                                        let current_language_id = cx.global::<I18nManager>().current_language_id();
                                        is_selected = act.language_id == current_language_id;
                                    }

                                    let is_theme_or_lang = action.as_ref().as_any().is::<SelectTheme>()
                                        || action.as_ref().as_any().is::<SelectLanguage>();

                                    let base = div()
                                        .id(("app-submenu-item", submenu_index * 1000 + item_index))
                                        .w_full()
                                        .h(px(d.menu_item_height))
                                        .px(px(d.menu_item_padding_x))
                                        .flex()
                                        .items_center()
                                        .when(is_theme_or_lang, |this| this.justify_between())
                                        .rounded(px(d.menu_item_radius))
                                        .bg(if is_selected {
                                            c.dialog_secondary_button_hover
                                        } else {
                                            c.dialog_surface
                                        })
                                        .text_size(px(d.menu_text_size))
                                        .font_weight(t.dialog_body_weight.to_font_weight())
                                        .text_color(if is_disabled {
                                            c.dialog_muted
                                        } else {
                                            c.dialog_secondary_button_text
                                        })
                                        .child(left_elem.unwrap_or_else(|| div().child(name.clone()).into_any_element()))
                                        .when(is_theme_or_lang, |this| {
                                            this.child(if is_selected {
                                                svg()
                                                    .path("icon/panel/check.svg")
                                                    .size(px(13.0))
                                                    .text_color(c.dialog_primary_button_bg)
                                                    .into_any_element()
                                            } else {
                                                div().w(px(13.0)).into_any_element()
                                            })
                                        });

                                    if is_disabled {
                                        base.into_any_element()
                                    } else {
                                        base.hover(|this| this.bg(c.dialog_secondary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .on_click(move |_, window, cx| {
                                                let _ = editor.update(cx, |editor, cx| {
                                                    editor.close_menu_bar(cx)
                                                });
                                                dispatch_menu_action_for_editor(
                                                    action.as_ref(),
                                                    &editor,
                                                    window,
                                                    cx,
                                                );
                                            })
                                            .into_any_element()
                                    }
                                }
                                OwnedMenuItem::Submenu(submenu) => div()
                                    .id(("app-submenu-nested", submenu_index * 1000 + item_index))
                                    .w_full()
                                    .h(px(d.menu_item_height))
                                    .px(px(d.menu_item_padding_x))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .rounded(px(d.menu_item_radius))
                                    .bg(c.dialog_surface)
                                    .text_size(px(d.menu_text_size))
                                    .text_color(c.dialog_muted)
                                    .child(submenu.name.to_string())
                                    .child(
                                        svg()
                                            .path("icon/panel/chevron-right.svg")
                                            .size(px(14.0))
                                            .text_color(c.dialog_muted),
                                    )
                                    .into_any_element(),
                                OwnedMenuItem::SystemMenu(os_menu) => div()
                                    .id(("app-submenu-system", submenu_index * 1000 + item_index))
                                    .w_full()
                                    .h(px(d.menu_item_height))
                                    .px(px(d.menu_item_padding_x))
                                    .flex()
                                    .items_center()
                                    .rounded(px(d.menu_item_radius))
                                    .bg(c.dialog_surface)
                                    .text_size(px(d.menu_text_size))
                                    .text_color(c.dialog_muted)
                                    .child(os_menu.name.to_string())
                                    .into_any_element(),
                            },
                        );

                        Some(
                            div()
                                .id(("app-submenu-panel", open_index * 1000 + submenu_index))
                                .absolute()
                                .occlude()
                                .top(px(top_offset + top))
                                .left(px(left))
                                .w(px(submenu_width))
                                .p(px(d.menu_panel_padding))
                                .flex()
                                .flex_col()
                                .gap(px(d.menu_panel_gap))
                                .bg(c.dialog_surface)
                                .border(px(d.dialog_border_width))
                                .border_color(c.dialog_border)
                                .rounded(px(d.menu_panel_radius))
                                .shadow_lg()
                                .on_hover(cx.listener(Self::on_menu_submenu_panel_hover))
                                .children(submenu_items)
                                .into_any_element(),
                        )
                    }
                    _ => None,
                }
            });

        let main_panel = div()
            .id(("app-menu-panel", open_index))
            .absolute()
            .occlude()
            .top(px(top_offset + d.menu_panel_top))
            .left(px(menu_panel_left(open_index, menu_labels, d)))
            .w(px(menu_panel_width))
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .on_hover(cx.listener(Self::on_menu_panel_hover));
        let main_panel = if let Some(split_index) = import_menu_split_index(&menu_items) {
            let scroll_items = &menu_items[..split_index];
            let footer_items = &menu_items[split_index..];
            let scroll_height = scrollable_import_menu_scroll_height(
                scroll_items,
                footer_items,
                viewport_height,
                top_offset,
                d,
            );
            let scroll_area = (!scroll_items.is_empty()).then(|| {
                div()
                    .id(("app-menu-scroll-area", open_index))
                    .w_full()
                    .h(px(scroll_height))
                    .flex_shrink_0()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(d.menu_panel_gap))
                            .children(scroll_items.iter().cloned().enumerate().map(
                                |(item_index, item)| {
                                    self.render_in_window_menu_item(
                                        item,
                                        item_index,
                                        theme,
                                        editor.clone(),
                                        cx,
                                    )
                                },
                            )),
                    )
                    .into_any_element()
            });
            let footer_elements =
                footer_items
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(footer_index, item)| {
                        self.render_in_window_menu_item(
                            item,
                            split_index + footer_index,
                            theme,
                            editor.clone(),
                            cx,
                        )
                    });

            main_panel
                .children(scroll_area)
                .children(footer_elements)
                .into_any_element()
        } else {
            let items = menu_items
                .iter()
                .cloned()
                .enumerate()
                .map(|(item_index, item)| {
                    self.render_in_window_menu_item(item, item_index, theme, editor.clone(), cx)
                });

            main_panel.children(items).into_any_element()
        };

        let layer = div()
            .id(("app-menu-panel-layer", open_index))
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .child(main_panel);
        let layer = if let Some(submenu_bridge) = submenu_bridge {
            layer.child(submenu_bridge)
        } else {
            layer
        };
        let layer = if let Some(submenu_panel) = submenu_panel {
            layer.child(submenu_panel)
        } else {
            layer
        };

        Some(layer.into_any_element())
    }

    /// Builds the unsaved-changes dialog with backdrop, message, and three
    /// action buttons (save-and-close, discard, cancel).
    fn render_unsaved_changes_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        div()
            .id("unsaved-changes-overlay")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_close_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("unsaved-changes-dialog")
                            .w(px(d.dialog_width))
                            .max_w(relative(1.0))
                            .flex()
                            .flex_col()
                            .gap(px(d.dialog_gap))
                            .p(px(d.dialog_padding))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_, _, _| {})
                            .child(
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(strings.unsaved_changes_title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(t.dialog_body_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .line_height(rems(t.text_line_height))
                                    .text_color(c.dialog_body)
                                    .child(strings.unsaved_changes_message.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(d.dialog_button_gap))
                                    .child(
                                        div()
                                            .id("save-and-close-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .bg(c.dialog_primary_button_bg)
                                            .hover(|this| this.bg(c.dialog_primary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.unsaved_changes_save_and_close.clone())
                                            .on_click(cx.listener(Self::on_save_and_close)),
                                    )
                                    .child(
                                        div()
                                            .id("discard-and-close-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .border(px(d.dialog_border_width))
                                            .border_color(c.dialog_border)
                                            .bg(c.dialog_danger_button_bg)
                                            .hover(|this| this.bg(c.dialog_danger_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(
                                                strings.unsaved_changes_discard_and_close.clone(),
                                            )
                                            .on_click(cx.listener(Self::on_discard_and_close)),
                                    )
                                    .child(
                                        div()
                                            .id("cancel-close-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .border(px(d.dialog_border_width))
                                            .border_color(c.dialog_border)
                                            .bg(c.dialog_secondary_button_bg)
                                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.unsaved_changes_cancel.clone())
                                            .on_click(cx.listener(Self::on_cancel_close_dialog)),
                                    ),
                            ),
                    ),
            )
    }

    /// Builds the dropped-file replacement dialog shown when the current
    /// document has unsaved changes.
    fn render_drop_replace_overlay(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        div()
            .id("drop-replace-overlay")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(Self::on_cancel_drop_replace_dialog))
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("drop-replace-dialog")
                            .w(px(d.dialog_width))
                            .max_w(relative(1.0))
                            .flex()
                            .flex_col()
                            .gap(px(d.dialog_gap))
                            .p(px(d.dialog_padding))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.menu_panel_radius))
                            .shadow_lg()
                            .occlude()
                            .on_click(|_, _, _| {})
                            .child(
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(strings.drop_replace_title.clone()),
                            )
                            .child(
                                div()
                                    .text_size(px(t.dialog_body_size))
                                    .font_weight(t.dialog_body_weight.to_font_weight())
                                    .line_height(rems(t.text_line_height))
                                    .text_color(c.dialog_body)
                                    .child(strings.drop_replace_message.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(d.dialog_button_gap))
                                    .child(
                                        div()
                                            .id("save-and-replace-drop-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .bg(c.dialog_primary_button_bg)
                                            .hover(|this| this.bg(c.dialog_primary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.drop_replace_save_and_replace.clone())
                                            .on_click(
                                                cx.listener(Self::on_save_and_replace_drop),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("discard-and-replace-drop-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .border(px(d.dialog_border_width))
                                            .border_color(c.dialog_border)
                                            .bg(c.dialog_danger_button_bg)
                                            .hover(|this| this.bg(c.dialog_danger_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_danger_button_text)
                                            .child(
                                                strings
                                                    .drop_replace_discard_and_replace
                                                    .clone(),
                                            )
                                            .on_click(
                                                cx.listener(Self::on_discard_and_replace_drop),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("cancel-drop-replace-dialog")
                                            .h(px(32.0))
                                            .px(px(14.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(d.menu_item_radius))
                                            .border(px(d.dialog_border_width))
                                            .border_color(c.dialog_border)
                                            .bg(c.dialog_secondary_button_bg)
                                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(13.0))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_secondary_button_text)
                                            .child(strings.drop_replace_cancel.clone())
                                            .on_click(
                                                cx.listener(Self::on_cancel_drop_replace_dialog),
                                            ),
                                    ),
                            ),
                    ),
            )
    }

    fn info_dialog_title<'a>(&self, strings: &'a I18nStrings, kind: InfoDialogKind) -> &'a str {
        match kind {
            InfoDialogKind::CheckForUpdates => &strings.help_check_updates_title,
            InfoDialogKind::About => &strings.help_about_title,
        }
    }

    pub(crate) fn about_dialog_body_lines(strings: &I18nStrings) -> Vec<String> {
        vec![
            format!("Velotype {}", env!("CARGO_PKG_VERSION")),
            strings.help_about_message.clone(),
            format!("{}: {}", strings.help_about_github_label, ABOUT_GITHUB_URL),
            strings.help_about_star_message.clone(),
        ]
    }

    fn info_dialog_body(&self, strings: &I18nStrings, kind: InfoDialogKind) -> String {
        match kind {
            InfoDialogKind::CheckForUpdates => strings.help_check_updates_message.clone(),
            InfoDialogKind::About => Self::about_dialog_body_lines(strings).join("\n"),
        }
    }

    fn render_info_dialog_body(
        &self,
        theme: &Theme,
        strings: &I18nStrings,
        kind: InfoDialogKind,
    ) -> AnyElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let body_style = |this: Div| {
            this.text_size(px(t.dialog_body_size))
                .font_weight(t.dialog_body_weight.to_font_weight())
                .line_height(rems(t.text_line_height))
                .text_color(c.dialog_body)
        };

        match kind {
            InfoDialogKind::CheckForUpdates => div()
                .flex()
                .flex_col()
                .gap(px(d.dialog_gap * 0.5))
                .child(
                    body_style(div()).children(
                        self.info_dialog_body(strings, kind)
                            .lines()
                            .map(|line| div().child(line.to_string())),
                    ),
                )
                .into_any_element(),
            InfoDialogKind::About => div()
                .flex()
                .flex_col()
                .gap(px(d.dialog_gap * 0.5))
                .child(body_style(div()).child(format!("Velotype {}", env!("CARGO_PKG_VERSION"))))
                .child(body_style(div()).child(strings.help_about_message.clone()))
                .child(
                    body_style(div())
                        .flex()
                        .flex_wrap()
                        .gap(px(4.0))
                        .child(format!("{}:", strings.help_about_github_label))
                        .child(
                            div()
                                .id("about-github-link")
                                .cursor_pointer()
                                .text_color(c.text_link)
                                .underline()
                                .child(ABOUT_GITHUB_URL)
                                .on_click(move |_, _, cx| {
                                    open_about_github_url(cx);
                                }),
                        ),
                )
                .child(body_style(div()).child(strings.help_about_star_message.clone()))
                .into_any_element(),
        }
    }

    fn on_dismiss_info_dialog(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.hide_info_dialog(cx);
    }

    fn render_info_dialog_overlay(
        &self,
        theme: &Theme,
        kind: InfoDialogKind,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let strings = cx.global::<I18nManager>().strings();

        div()
            .id("info-dialog-overlay")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .bg(c.dialog_backdrop)
            .child(
                div()
                    .w_full()
                    .px(px(d.editor_padding))
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .id("info-dialog")
                            .w(px(d.dialog_width))
                            .max_w(relative(1.0))
                            .flex()
                            .flex_col()
                            .gap(px(d.dialog_gap))
                            .p(px(d.dialog_padding))
                            .bg(c.dialog_surface)
                            .border(px(d.dialog_border_width))
                            .border_color(c.dialog_border)
                            .rounded(px(d.dialog_radius))
                            .shadow_lg()
                            .child(
                                div()
                                    .text_size(px(t.dialog_title_size))
                                    .font_weight(t.dialog_title_weight.to_font_weight())
                                    .text_color(c.dialog_title)
                                    .child(self.info_dialog_title(strings, kind).to_string()),
                            )
                            .child(self.render_info_dialog_body(theme, strings, kind))
                            .child(
                                div()
                                    .flex()
                                    .justify_end()
                                    .gap(px(d.dialog_button_gap))
                                    .child(
                                        div()
                                            .id("dismiss-info-dialog")
                                            .h(px(d.dialog_button_height))
                                            .px(px(d.dialog_button_padding_x))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px((d.dialog_radius - 4.0).max(0.0)))
                                            .bg(c.dialog_primary_button_bg)
                                            .hover(|this| this.bg(c.dialog_primary_button_hover))
                                            .active(|this| this.opacity(0.92))
                                            .cursor_pointer()
                                            .text_size(px(t.dialog_button_size))
                                            .font_weight(t.dialog_button_weight.to_font_weight())
                                            .text_color(c.dialog_primary_button_text)
                                            .child(strings.info_dialog_ok.clone())
                                            .on_click(cx.listener(Self::on_dismiss_info_dialog)),
                                    ),
                            ),
                    ),
            )
    }

    pub(crate) fn render_tiled_layout(
        &mut self,
        content_area: AnyElement,
        theme: &Theme,
        strings: &I18nStrings,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let root = self.area_layout.root.clone();
        let leaf_count = root.count_leaves();
        let mut primary_content = Some(content_area);

        let layout_tree = if let Some(maximized_id) = self.area_layout.maximized_leaf {
            if let Some(area_type) = root.find_leaf_area(maximized_id) {
                self.render_area_tile(
                    maximized_id,
                    area_type,
                    &mut primary_content,
                    theme,
                    strings,
                    leaf_count,
                    true,
                    cx,
                )
            } else {
                self.render_tiled_layout_node(
                    &root,
                    &mut primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                )
            }
        } else {
            self.render_tiled_layout_node(
                &root,
                &mut primary_content,
                theme,
                strings,
                leaf_count,
                cx,
            )
        };

        let root_editor_move = cx.entity().downgrade();
        let root_editor_up = cx.entity().downgrade();

        let container = div()
            .id("tiled-layout-root")
            .w_full()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .relative()
            .on_mouse_move(move |event, window, cx| {
                let pos = event.position;
                let _ = root_editor_move.update(cx, |ed, cx| {
                    let mut changed = false;
                    if let Some(drag) = ed.area_layout.active_splitter_drag {
                        let current_pos = match drag.direction {
                            SplitDirection::Horizontal => f32::from(pos.x),
                            SplitDirection::Vertical => f32::from(pos.y),
                        };
                        let viewport = window.viewport_size();
                        let span = ed
                            .area_layout
                            .get_split_pixel_span(drag.split_id, viewport)
                            .unwrap_or_else(|| match drag.direction {
                                SplitDirection::Horizontal => f32::from(viewport.width),
                                SplitDirection::Vertical => f32::from(viewport.height),
                            });

                        if span > 1.0 {
                            let mut session = drag;
                            session.total_span = span;
                            ed.area_layout.active_splitter_drag = Some(session);
                        }
                        ed.area_layout.update_splitter_drag(current_pos);
                        changed = true;
                    } else if ed.area_layout.active_corner_drag.is_some() {
                        let viewport = window.viewport_size();
                        let action = ed.area_layout.update_corner_drag(pos, viewport);
                        // Modifier actions still execute immediately.
                        if let Some(action) = action {
                            match action {
                                CornerDragAction::Swap { from, to } => {
                                    ed.area_layout.end_corner_drag();
                                    ed.area_layout.swap_area_types(from, to);
                                }
                                CornerDragAction::Duplicate { .. } => {
                                    ed.area_layout.end_corner_drag();
                                }
                                CornerDragAction::Cancel => {
                                    ed.area_layout.end_corner_drag();
                                }
                                _ => {} // Split/Join handled on mouse_up
                            }
                        }
                        changed = true;
                    }
                    if changed {
                        cx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                let _ = root_editor_up.update(cx, |ed, cx| {
                    if ed.area_layout.active_splitter_drag.is_some() {
                        ed.area_layout.end_splitter_drag();
                        cx.notify();
                    }
                    if ed.area_layout.active_corner_drag.is_some() {
                        match ed.area_layout.finish_corner_drag() {
                            Some(CornerDragAction::Split {
                                leaf_id,
                                direction,
                                ratio,
                            }) => {
                                ed.area_layout
                                    .split_area_with_ratio(leaf_id, direction, ratio);
                            }
                            Some(CornerDragAction::Join { from, into }) => {
                                ed.area_layout.join_area(into, from);
                            }
                            Some(CornerDragAction::Swap { from, to }) => {
                                ed.area_layout.swap_area_types(from, to);
                            }
                            _ => {}
                        }
                        cx.notify();
                    }
                });
            })
            .child(layout_tree);

        // Build the preview overlay for corner drag gestures.
        let preview_overlay = if let Some(drag) = &self.area_layout.active_corner_drag {
            match drag.preview {
                CornerDragPreview::SplitPreview { direction, ratio } => {
                    // Calculate the pixel rect of the leaf being split.
                    let viewport = _window.viewport_size();
                    let leaf_rects = self.area_layout.collect_leaf_rects(viewport);
                    let leaf_rect = self
                        .area_layout
                        .get_leaf_pixel_rect(drag.leaf_id, &leaf_rects);

                    if let Some((_, lx, ly, lw, lh)) = leaf_rect {
                        // Horizontal split = left|right → draw a VERTICAL line
                        // Vertical split = top|bottom → draw a HORIZONTAL line
                        let line = match direction {
                            SplitDirection::Horizontal => div()
                                .absolute()
                                .left(px(lx + lw * ratio))
                                .top(px(ly))
                                .w(px(3.0))
                                .h(px(lh))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                            SplitDirection::Vertical => div()
                                .absolute()
                                .top(px(ly + lh * ratio))
                                .left(px(lx))
                                .h(px(3.0))
                                .w(px(lw))
                                .bg(hsla(0.36, 0.73, 0.57, 0.8)),
                        };

                        // Also draw a semi-transparent highlight over the leaf
                        Some(
                            div()
                                .absolute()
                                .inset(px(0.0))
                                .child(
                                    div()
                                        .absolute()
                                        .left(px(lx))
                                        .top(px(ly))
                                        .w(px(lw))
                                        .h(px(lh))
                                        .rounded(px(theme.dimensions.area_tile_radius))
                                        .bg(hsla(0.36, 0.73, 0.57, 0.08)),
                                )
                                .child(line),
                        )
                    } else {
                        None
                    }
                }
                CornerDragPreview::JoinPreview {
                    target_leaf_id,
                    direction,
                } => {
                    let viewport = _window.viewport_size();
                    let leaf_rects = self.area_layout.collect_leaf_rects(viewport);
                    let target_rect = self
                        .area_layout
                        .get_leaf_pixel_rect(target_leaf_id, &leaf_rects);

                    if let Some((_, rx, ry, rw, rh)) = target_rect {
                        let arrow_symbol = match direction {
                            ScreenDirection::North => "▲",
                            ScreenDirection::South => "▼",
                            ScreenDirection::East => "▶",
                            ScreenDirection::West => "◀",
                        };

                        Some(
                            div()
                                .absolute()
                                .left(px(rx))
                                .top(px(ry))
                                .w(px(rw))
                                .h(px(rh))
                                .rounded(px(theme.dimensions.area_tile_radius))
                                .bg(hsla(0.36, 0.73, 0.57, 0.25))
                                .border(px(2.0))
                                .border_color(hsla(0.36, 0.73, 0.57, 0.8))
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .px(px(12.0))
                                        .py(px(6.0))
                                        .rounded_md()
                                        .bg(hsla(0.0, 0.0, 0.0, 0.75))
                                        .text_color(hsla(0.0, 0.0, 1.0, 0.95))
                                        .text_size(px(15.0))
                                        .font_weight(FontWeight::BOLD)
                                        .child(format!("{} Join Area", arrow_symbol)),
                                ),
                        )
                    } else {
                        Some(
                            div()
                                .absolute()
                                .inset(px(0.0))
                                .bg(hsla(0.36, 0.73, 0.57, 0.15)),
                        )
                    }
                }
                CornerDragPreview::Dragging => None,
            }
        } else {
            None
        };
        let container = container.children(preview_overlay);

        if let Some(border_menu) = self.area_layout.active_border_menu {
            let menu_overlay = self.render_border_context_menu_overlay(border_menu, theme, cx);
            container.child(menu_overlay).into_any_element()
        } else {
            container.into_any_element()
        }
    }

    fn render_tiled_layout_node(
        &mut self,
        node: &crate::editor::area_layout::LayoutNode,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let editor = cx.entity().downgrade();

        match node {
            LayoutNode::Leaf { id, area_type } => self.render_area_tile(
                *id,
                *area_type,
                primary_content,
                theme,
                strings,
                leaf_count,
                false,
                cx,
            ),
            LayoutNode::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let split_id = *id;
                let dir = *direction;
                let r = *ratio;

                let first_elem = self.render_tiled_layout_node(
                    first,
                    primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                );
                let second_elem = self.render_tiled_layout_node(
                    second,
                    primary_content,
                    theme,
                    strings,
                    leaf_count,
                    cx,
                );

                match direction {
                    SplitDirection::Horizontal => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();

                        div()
                            .id(("tiled-split-h", split_id))
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
                                // Splitter bar between the two padded tiles.
                                div()
                                    .id(("tiled-splitter-bar-h", split_id))
                                    .w(px(2.0))
                                    .h_full()
                                    .flex_shrink_0()
                                    .cursor_col_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.x);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.area_layout.active_splitter_drag =
                                                Some(SplitterDragSession {
                                                    split_id,
                                                    direction: SplitDirection::Horizontal,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 1000.0,
                                                });
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.area_layout.active_border_menu =
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
                    SplitDirection::Vertical => {
                        let bar_editor = editor.clone();
                        let menu_editor = editor.clone();

                        div()
                            .id(("tiled-split-v", split_id))
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
                                // Splitter bar between the two padded tiles.
                                div()
                                    .id(("tiled-splitter-bar-v", split_id))
                                    .h(px(2.0))
                                    .w_full()
                                    .flex_shrink_0()
                                    .cursor_row_resize()
                                    .bg(c.dialog_border)
                                    .hover(|this| this.bg(c.selection))
                                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                                        let start_pos = f32::from(event.position.y);
                                        let _ = bar_editor.update(cx, |ed, cx| {
                                            ed.area_layout.active_splitter_drag =
                                                Some(SplitterDragSession {
                                                    split_id,
                                                    direction: SplitDirection::Vertical,
                                                    start_pointer_pos: start_pos,
                                                    start_ratio: r,
                                                    total_span: 700.0,
                                                });
                                            cx.notify();
                                        });
                                    })
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        move |event, _window, cx| {
                                            let pos = event.position;
                                            let _ = menu_editor.update(cx, |ed, cx| {
                                                ed.area_layout.active_border_menu =
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

    fn render_area_tile(
        &mut self,
        leaf_id: usize,
        area_type: crate::editor::area_layout::AreaType,
        primary_content: &mut Option<AnyElement>,
        theme: &Theme,
        strings: &I18nStrings,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let d = &theme.dimensions;
        let gap = d.area_tile_gap;
        let radius = d.area_tile_radius;

        let header =
            self.render_area_header(leaf_id, area_type, theme, leaf_count, is_maximized, cx);

        let body: AnyElement = match area_type {
            AreaType::Block | AreaType::Source => {
                if let Some(content) = primary_content.take() {
                    content
                } else {
                    div()
                        .w_full()
                        .h_full()
                        .p(px(16.0))
                        .flex()
                        .flex_col()
                        .justify_center()
                        .bg(c.editor_background)
                        .child(
                            div()
                                .w_full()
                                .text_align(TextAlign::Center)
                                .text_size(px(14.0))
                                .text_color(c.dialog_muted)
                                .child(format!("{} (Editor View)", area_type.name())),
                        )
                        .into_any_element()
                }
            }
            AreaType::Explorer => self.render_tiled_workspace_files_panel(theme, strings, cx),
            AreaType::Outline => self.render_tiled_outline_panel(theme, strings, cx),
            AreaType::Settings => self.render_tiled_settings_panel(theme, strings, cx),
        };

        let dropdown_open = self.area_layout.active_dropdown_leaf == Some(leaf_id);

        // Tile card with overflow hidden (no corner handles inside, to avoid clipping).
        let tile_card = div()
            .id(("tiled-area-card", leaf_id))
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .rounded(px(radius))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .shadow_lg()
            .overflow_hidden()
            .child(header)
            .child(
                div()
                    .w_full()
                    .flex_1()
                    .min_h(px(0.0))
                    .relative()
                    .child(body),
            );

        // Corner drag handles positioned at the four corners of the tile card.
        // Attached only to the 4 specific corner hot-zones (top-left, top-right, bottom-left, bottom-right).
        let editor_corner = cx.entity().downgrade();
        let make_corner = |id_str: &'static str, top: bool, left: bool| {
            let editor_corner = editor_corner.clone();
            let mut corner_div = div()
                .id((id_str, leaf_id))
                .absolute()
                .size(px(24.0))
                .cursor_crosshair();

            if top {
                corner_div = corner_div.top(px(gap));
            } else {
                corner_div = corner_div.bottom(px(gap));
            }
            if left {
                corner_div = corner_div.left(px(gap));
            } else {
                corner_div = corner_div.right(px(gap));
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
                let _ = editor_corner.update(cx, |ed, cx| {
                    ed.area_layout.start_corner_drag(leaf_id, pos, modifier);
                    cx.notify();
                });
            })
        };

        let corner_handles = div()
            .id(("area-corners", leaf_id))
            .absolute()
            .inset(px(-gap))
            .child(make_corner("area-corner-tl", true, true))
            .child(make_corner("area-corner-tr", true, false))
            .child(make_corner("area-corner-bl", false, true))
            .child(make_corner("area-corner-br", false, false));

        // Wrap in a padded container so the gap is uniform.
        let wrapped = div()
            .id(("tiled-area-wrapper", leaf_id))
            .w_full()
            .h_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .p(px(gap))
            .relative()
            .child(tile_card)
            .child(corner_handles);

        if dropdown_open {
            let menu = self.render_area_dropdown_menu(leaf_id, theme, cx);
            wrapped.child(menu).into_any_element()
        } else {
            wrapped.into_any_element()
        }
    }

    fn render_area_header(
        &mut self,
        leaf_id: usize,
        area_type: crate::editor::area_layout::AreaType,
        theme: &Theme,
        leaf_count: usize,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let d = &theme.dimensions;
        let editor = cx.entity().downgrade();

        let type_editor = editor.clone();
        let type_button = div()
            .id(("area-header-type", leaf_id))
            .h(px(22.0))
            .px(px(8.0))
            .flex()
            .items_center()
            .gap(px(4.0))
            .rounded(px(d.menu_item_radius))
            .bg(c.dialog_secondary_button_bg)
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .text_size(px(12.0))
            .text_color(c.text_default)
            .child(format!("{} ▼", area_type.name()))
            .on_click(move |_event, _window, cx| {
                let _ = type_editor.update(cx, |ed, cx| {
                    ed.area_layout.toggle_dropdown(leaf_id);
                    cx.notify();
                });
            });

        let split_h_editor = editor.clone();
        let split_h_button = div()
            .id(("area-btn-split-h", leaf_id))
            .p(px(4.0))
            .rounded(px(d.menu_item_radius))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icon/panel/split-h.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_h_editor.update(cx, |ed, cx| {
                    ed.area_layout
                        .split_area(leaf_id, SplitDirection::Horizontal);
                    cx.notify();
                });
            });

        let split_v_editor = editor.clone();
        let split_v_button = div()
            .id(("area-btn-split-v", leaf_id))
            .p(px(4.0))
            .rounded(px(d.menu_item_radius))
            .hover(|this| this.bg(c.dialog_secondary_button_hover))
            .cursor_pointer()
            .child(
                svg()
                    .path("icon/panel/split-v.svg")
                    .size(px(14.0))
                    .text_color(c.dialog_muted),
            )
            .on_click(move |_event, _window, cx| {
                let _ = split_v_editor.update(cx, |ed, cx| {
                    ed.area_layout.split_area(leaf_id, SplitDirection::Vertical);
                    cx.notify();
                });
            });

        let mut actions = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(split_h_button)
            .child(split_v_button);

        if leaf_count > 1 {
            let max_editor = editor.clone();
            let max_button = div()
                .id(("area-btn-max", leaf_id))
                .p(px(4.0))
                .rounded(px(d.menu_item_radius))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path(if is_maximized {
                            "icon/titlebar/chrome-restore.svg"
                        } else {
                            "icon/titlebar/chrome-maximize.svg"
                        })
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = max_editor.update(cx, |ed, cx| {
                        ed.area_layout.toggle_maximize(leaf_id);
                        cx.notify();
                    });
                });

            let close_editor = editor.clone();
            let close_button = div()
                .id(("area-btn-close", leaf_id))
                .p(px(4.0))
                .rounded(px(d.menu_item_radius))
                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                .cursor_pointer()
                .child(
                    svg()
                        .path("icon/titlebar/chrome-close.svg")
                        .size(px(14.0))
                        .text_color(c.dialog_muted),
                )
                .on_click(move |_event, _window, cx| {
                    let _ = close_editor.update(cx, |ed, cx| {
                        ed.area_layout.close_area(leaf_id);
                        cx.notify();
                    });
                });

            actions = actions.child(max_button).child(close_button);
        }

        // Corner drag is now handled by corner hot-zones on the tile card,
        // so the header only contains the type button and action buttons.
        div()
            .id(("area-header", leaf_id))
            .h(px(28.0))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .border_b(px(1.0))
            .border_color(c.dialog_border)
            .child(div().flex().items_center().gap(px(8.0)).child(type_button))
            .child(div().flex().items_center().gap(px(6.0)).child(actions))
            .into_any_element()
    }

    fn render_area_dropdown_menu(
        &mut self,
        leaf_id: usize,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();

        div()
            .id(("area-dropdown-overlay", leaf_id))
            .absolute()
            .occlude()
            .top(px(30.0))
            .left(px(8.0))
            .w(px(d.menu_panel_width))
            .p(px(d.menu_panel_padding))
            .flex()
            .flex_col()
            .gap(px(d.menu_panel_gap))
            .bg(c.dialog_surface)
            .border(px(d.dialog_border_width))
            .border_color(c.dialog_border)
            .rounded(px(d.menu_panel_radius))
            .shadow_lg()
            .children(AreaType::all().iter().enumerate().map(|(idx, area_type)| {
                let area_type = *area_type;
                let option_editor = editor.clone();
                div()
                    .id(("area-type-opt", idx))
                    .w_full()
                    .h(px(d.menu_item_height))
                    .px(px(d.menu_item_padding_x))
                    .flex()
                    .items_center()
                    .rounded(px(d.menu_item_radius))
                    .bg(c.dialog_surface)
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .active(|this| this.opacity(0.92))
                    .cursor_pointer()
                    .text_size(px(d.menu_text_size))
                    .font_weight(t.dialog_button_weight.to_font_weight())
                    .text_color(c.dialog_secondary_button_text)
                    .child(area_type.name())
                    .on_click(move |_event, _window, cx| {
                        let _ = option_editor.update(cx, |ed, cx| {
                            ed.area_layout.change_area_type(leaf_id, area_type);
                            match area_type {
                                AreaType::Source => ed.set_view_mode(super::ViewMode::Source, cx),
                                AreaType::Block => {
                                    ed.set_view_mode(super::ViewMode::Rendered, cx)
                                }
                                _ => {}
                            }
                            cx.notify();
                        });
                    })
                    .into_any_element()
            }))
            .into_any_element()
    }

    fn render_border_context_menu_overlay(
        &mut self,
        border_menu: crate::editor::area_layout::BorderMenuState,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let d = &theme.dimensions;
        let t = &theme.typography;
        let editor = cx.entity().downgrade();
        let split_id = border_menu.split_id;

        let left_pos = f32::from(border_menu.position.x);
        let top_pos = f32::from(border_menu.position.y);

        let split_h_ed = editor.clone();
        let split_v_ed = editor.clone();
        let swap_ed = editor.clone();
        let close_ed = editor.clone();
        let dismiss_ed = editor.clone();

        div()
            .id("tiled-border-context-menu-wrapper")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                let _ = dismiss_ed.update(cx, |ed, cx| {
                    ed.area_layout.active_border_menu = None;
                    cx.notify();
                });
            })
            .child(
                div()
                    .id("tiled-border-context-menu")
                    .absolute()
                    .occlude()
                    .top(px(top_pos))
                    .left(px(left_pos))
                    .w(px(d.menu_panel_width))
                    .p(px(d.menu_panel_padding))
                    .flex()
                    .flex_col()
                    .gap(px(d.menu_panel_gap))
                    .bg(c.dialog_surface)
                    .border(px(d.dialog_border_width))
                    .border_color(c.dialog_border)
                    .rounded(px(d.menu_panel_radius))
                    .shadow_lg()
                    .child(
                        div()
                            .id("border-menu-split-h")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Horizontally")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_h_ed.update(cx, |ed, cx| {
                                    ed.area_layout
                                        .split_area(split_id, SplitDirection::Horizontal);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-split-v")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Split Vertically")
                            .on_click(move |_event, _window, cx| {
                                let _ = split_v_ed.update(cx, |ed, cx| {
                                    ed.area_layout
                                        .split_area(split_id, SplitDirection::Vertical);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-sep-1")
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border),
                    )
                    .child(
                        div()
                            .id("border-menu-swap")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Swap Panels")
                            .on_click(move |_event, _window, cx| {
                                let _ = swap_ed.update(cx, |ed, cx| {
                                    ed.area_layout.swap_split_children(split_id);
                                    cx.notify();
                                });
                            }),
                    )
                    .child(
                        div()
                            .id("border-menu-sep-2")
                            .mx(px(d.menu_separator_margin_x))
                            .my(px(d.menu_separator_margin_y))
                            .h(px(d.menu_separator_height))
                            .bg(c.dialog_border),
                    )
                    .child(
                        div()
                            .id("border-menu-close")
                            .w_full()
                            .h(px(d.menu_item_height))
                            .px(px(d.menu_item_padding_x))
                            .flex()
                            .items_center()
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_surface)
                            .hover(|this| this.bg(c.dialog_secondary_button_hover))
                            .active(|this| this.opacity(0.92))
                            .cursor_pointer()
                            .text_size(px(d.menu_text_size))
                            .font_weight(t.dialog_body_weight.to_font_weight())
                            .text_color(c.dialog_secondary_button_text)
                            .child("Close Area")
                            .on_click(move |_event, _window, cx| {
                                let _ = close_ed.update(cx, |ed, cx| {
                                    ed.area_layout.close_area(split_id);
                                    cx.notify();
                                });
                            }),
                    ),
            )
            .into_any_element()
    }

    fn render_tiled_workspace_files_panel(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_workspace_models(cx);
        let editor = cx.entity().downgrade();
        self.render_workspace_files_tree(theme, strings, &editor)
    }

    fn render_tiled_outline_panel(
        &mut self,
        theme: &Theme,
        strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.sync_workspace_models(cx);
        let editor = cx.entity().downgrade();
        self.render_workspace_outline_tree(theme, strings, &editor)
    }

    fn render_tiled_settings_panel(
        &mut self,
        theme: &Theme,
        _strings: &I18nStrings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::editor::area_layout::*;

        let c = &theme.colors;
        let d = &theme.dimensions;
        let active_tab = self.area_layout.settings_tab;

        let mut inner_border_color = c.dialog_border;
        inner_border_color.a *= 0.4;

        // --- Left Sidebar (3 Main Tabs: Interface, Editing, Keymap) ---
        let mut left_nav_items = Vec::new();
        for (tab_idx, tab) in SettingsTab::all().iter().enumerate() {
            let is_active = active_tab == *tab;
            let editor = cx.entity().downgrade();
            let tab_item = *tab;

            left_nav_items.push(
                div()
                    .id(("pref-tab", tab_idx))
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(d.menu_item_radius))
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .bg(if is_active {
                        c.dialog_secondary_button_hover
                    } else {
                        c.dialog_surface
                    })
                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(if is_active {
                                gpui::FontWeight::BOLD
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if is_active {
                                c.text_default
                            } else {
                                c.dialog_muted
                            })
                            .child(tab.name()),
                    )
                    .on_click(move |_event, _window, cx| {
                        let _ = editor.update(cx, |ed, cx| {
                            ed.area_layout.settings_tab = tab_item;
                            cx.notify();
                        });
                    })
                    .into_any_element(),
            );
        }

        let left_nav = div()
            .w(px(160.0))
            .h_full()
            .flex_shrink_0()
            .p(px(8.0))
            .border_r_1()
            .border_color(c.dialog_border)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .children(left_nav_items);

        // --- Right Content Area ---
        let mut sections: Vec<AnyElement> = Vec::new();

        // Helper closures / local constructors to produce shallow type-erased elements
        let make_row = |title: &'static str,
                        desc: &'static str,
                        control: AnyElement,
                        theme: &Theme,
                        border_col: Hsla|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;
            div()
                .w_full()
                .h(px(56.0))
                .px(px(16.0))
                .rounded(px(td.menu_panel_radius))
                .bg(tc.dialog_surface)
                .border_1()
                .border_color(border_col)
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(
                            div()
                                .text_size(px(12.5))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(tc.text_default)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(tc.dialog_muted)
                                .child(desc),
                        ),
                )
                .child(control)
                .into_any_element()
        };

        let render_zed_stepper = |id_dec: &'static str,
                                  id_inc: &'static str,
                                  val_num: String,
                                  unit_str: &'static str,
                                  is_editing: bool,
                                  on_dec: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
                                  on_inc: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
                                  on_click_center: Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
                                  theme: &Theme|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;

            let mut center_box = div()
                .id(ElementId::Name(format!("{}-center", id_dec).into()))
                .cursor_pointer()
                .h_full()
                .flex_1()
                .min_w(px(0.0))
                .px(px(4.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(3.0))
                .bg(if is_editing { tc.dialog_surface } else { tc.dialog_secondary_button_bg })
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(tc.text_default)
                        .child(val_num),
                );

            if is_editing {
                center_box = center_box
                    .border_1()
                    .border_color(tc.dialog_primary_button_bg)
                    .child(div().w(px(1.5)).h(px(12.0)).bg(tc.dialog_primary_button_bg));
            }

            if !unit_str.is_empty() {
                center_box = center_box.child(
                    div()
                        .text_size(px(11.0))
                        .text_color(tc.dialog_muted)
                        .child(unit_str),
                );
            }

            let center_box = center_box.on_click(on_click_center);

            div()
                .flex()
                .items_center()
                .w(px(145.0))
                .h(px(28.0))
                .rounded(px(td.menu_item_radius))
                .border_1()
                .border_color(tc.dialog_border)
                .bg(tc.dialog_secondary_button_bg)
                .child(
                    div()
                        .id(id_dec)
                        .cursor_pointer()
                        .h_full()
                        .w(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|this| this.bg(tc.dialog_secondary_button_hover))
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(tc.text_default)
                        .child("-")
                        .on_click(on_dec),
                )
                .child(div().w(px(1.0)).h_full().bg(tc.dialog_border))
                .child(center_box)
                .child(div().w(px(1.0)).h_full().bg(tc.dialog_border))
                .child(
                    div()
                        .id(id_inc)
                        .cursor_pointer()
                        .h_full()
                        .w(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|this| this.bg(tc.dialog_secondary_button_hover))
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(tc.text_default)
                        .child("+")
                        .on_click(on_inc),
                )
                .into_any_element()
        };

        let make_section = |sec_id: &'static str,
                            title: &'static str,
                            is_expanded: bool,
                            toggle_fn: Box<
            dyn Fn(&gpui::ClickEvent, &mut Window, &mut App),
        >,
                            items: Vec<AnyElement>,
                            theme: &Theme|
         -> AnyElement {
            let tc = &theme.colors;
            let td = &theme.dimensions;

            let header = div()
                .id(sec_id)
                .w_full()
                .px(px(14.0))
                .py(px(10.0))
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    svg()
                        .path(if is_expanded {
                            "icon/panel/chevron-down.svg"
                        } else {
                            "icon/panel/chevron-right.svg"
                        })
                        .size(px(14.0))
                        .text_color(tc.text_default),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .text_color(tc.text_default)
                        .child(title),
                )
                .on_click(move |ev, window, cx| toggle_fn(ev, window, cx));

            let mut card = div()
                .relative()
                .w_full()
                .rounded(px(td.menu_panel_radius))
                .bg(tc.dialog_surface)
                .border_1()
                .border_color(tc.dialog_border)
                .flex()
                .flex_col()
                .child(header);

            if is_expanded && !items.is_empty() {
                let body = div()
                    .w_full()
                    .px(px(10.0))
                    .pb(px(10.0))
                    .pt(px(2.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(items);

                card = card.child(body);
            }

            card.into_any_element()
        };

        match active_tab {
            SettingsTab::Interface => {
                // Section 1: Visual Theme & Language
                let sec1_key = "theme";
                let is_sec1_expanded = self.area_layout.settings_expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                let theme_ed = cx.entity().downgrade();
                let available_themes = cx.global::<ThemeManager>().available_themes();
                let raw_theme_name = theme.name.clone();
                let current_theme_name: String = match raw_theme_name.as_str() {
                    "Velotype" => "Dark".to_string(),
                    "Velotype Light" => "Light".to_string(),
                    other => other.to_string(),
                };

                let lang_ed = cx.entity().downgrade();
                let lang_options = [("en-US", "English (en-US)"), ("zh-CN", "简体中文 (zh-CN)")];
                let current_lang = "English (en-US)";

                let is_theme_open = self.area_layout.open_settings_dropdown.as_deref() == Some("theme");
                let is_lang_open = self.area_layout.open_settings_dropdown.as_deref() == Some("lang");

                if is_sec1_expanded {
                    let theme_icon_path = if current_theme_name == "Light" {
                        "icon/panel/sun.svg"
                    } else {
                        "icon/panel/moon.svg"
                    };

                    let mut theme_btn_wrap = div()
                        .relative()
                        .child(
                            div()
                                .id("pref-btn-theme")
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .w(px(145.0))
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(d.menu_item_radius))
                                .bg(c.dialog_secondary_button_bg)
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .border_1()
                                .border_color(c.dialog_border)
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            svg()
                                                .path(theme_icon_path)
                                                .size(px(13.0))
                                                .text_color(c.text_default),
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w(px(0.0))
                                                .truncate()
                                                .child(current_theme_name.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .pl(px(4.0))
                                        .child(
                                            svg()
                                                .path("icon/panel/select-chevron.svg")
                                                .size(px(14.0))
                                                .text_color(c.dialog_muted),
                                        ),
                                )
                                .on_click({
                                    let theme_ed = theme_ed.clone();
                                    move |_ev, _win, cx| {
                                        let _ = theme_ed.update(cx, |ed, cx| {
                                            if ed.area_layout.open_settings_dropdown.as_deref() == Some("theme") {
                                                ed.area_layout.open_settings_dropdown = None;
                                            } else {
                                                ed.area_layout.open_settings_dropdown = Some("theme".to_string());
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                        );

                    if is_theme_open {
                        let mut menu_items = Vec::new();
                        for t_entry in available_themes {
                            let t_id = t_entry.id.clone();
                            let display_label: String = match t_entry.name.as_str() {
                                "Velotype" | "Dark" => "Dark".to_string(),
                                "Velotype Light" | "Light" => "Light".to_string(),
                                other => other.to_string(),
                            };
                            let is_selected = display_label == current_theme_name;
                            let item_ed = theme_ed.clone();
                            let item_icon = if display_label == "Light" {
                                "icon/panel/sun.svg"
                            } else {
                                "icon/panel/moon.svg"
                            };

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("theme-item-{}", t_id).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(6.0))
                                            .child(
                                                svg()
                                                    .path(item_icon)
                                                    .size(px(13.0))
                                                    .text_color(c.text_default),
                                            )
                                            .child(display_label),
                                    )
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            cx.update_global::<ThemeManager, _>(|manager, _cx| {
                                                let _ = manager.set_theme_by_id(&t_id);
                                            });
                                            ed.area_layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        theme_btn_wrap = theme_btn_wrap.child(
                            gpui::deferred(
                                div()
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt(px(4.0))
                                    .w(px(160.0))
                                    .occlude()
                                    .bg(c.dialog_surface)
                                    .border_1()
                                    .border_color(c.dialog_border)
                                    .rounded(px(6.0))
                                    .shadow_lg()
                                    .p(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .children(menu_items),
                            ),
                        );
                    }

                    sec1_items.push(make_row(
                        "Interface Theme",
                        "Customize overall application color scheme and appearance",
                        theme_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));

                    let mut lang_btn_wrap = div()
                        .relative()
                        .child(
                            div()
                                .id("pref-btn-lang")
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .w(px(145.0))
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(d.menu_item_radius))
                                .bg(c.dialog_secondary_button_bg)
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .border_1()
                                .border_color(c.dialog_border)
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .truncate()
                                        .child(current_lang),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .pl(px(4.0))
                                        .child(
                                            svg()
                                                .path("icon/panel/select-chevron.svg")
                                                .size(px(14.0))
                                                .text_color(c.dialog_muted),
                                        ),
                                )
                                .on_click({
                                    let lang_ed = lang_ed.clone();
                                    move |_ev, _win, cx| {
                                        let _ = lang_ed.update(cx, |ed, cx| {
                                            if ed.area_layout.open_settings_dropdown.as_deref() == Some("lang") {
                                                ed.area_layout.open_settings_dropdown = None;
                                            } else {
                                                ed.area_layout.open_settings_dropdown = Some("lang".to_string());
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                        );

                    if is_lang_open {
                        let mut menu_items = Vec::new();
                        for (code, label) in lang_options {
                            let is_selected = label == current_lang;
                            let item_ed = lang_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("lang-item-{}", code).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.area_layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        lang_btn_wrap = lang_btn_wrap.child(
                            gpui::deferred(
                                div()
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt(px(4.0))
                                    .w(px(160.0))
                                    .occlude()
                                    .bg(c.dialog_surface)
                                    .border_1()
                                    .border_color(c.dialog_border)
                                    .rounded(px(6.0))
                                    .shadow_lg()
                                    .p(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .children(menu_items),
                            ),
                        );
                    }

                    sec1_items.push(make_row(
                        "Display Language",
                        "Select preferred language for editor UI and dialogs",
                        lang_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-theme",
                    "Visual Theme & Language",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Status Bar Options
                let sec2_key = "status_bar";
                let is_sec2_expanded = self.area_layout.settings_expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let sub1_ed = cx.entity().downgrade();
                    let ctrl_sb_main = Switch::new("switch-sb-main")
                        .checked(self.area_layout.pref_show_status_bar)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub1_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_status_bar = !ed.area_layout.pref_show_status_bar;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Status Bar Visibility",
                        "Show or hide the persistent bottom status bar across window",
                        ctrl_sb_main,
                        theme,
                        inner_border_color,
                    ));

                    let sub2_ed = cx.entity().downgrade();
                    let ctrl_sb_words = Switch::new("switch-sb-words")
                        .checked(self.area_layout.pref_show_word_count)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub2_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_word_count = !ed.area_layout.pref_show_word_count;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Word Count Badge",
                        "Display real-time document word count in status bar",
                        ctrl_sb_words,
                        theme,
                        inner_border_color,
                    ));

                    let sub3_ed = cx.entity().downgrade();
                    let ctrl_sb_pos = Switch::new("switch-sb-pos")
                        .checked(self.area_layout.pref_show_cursor_pos)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub3_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_cursor_pos = !ed.area_layout.pref_show_cursor_pos;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Cursor Position Badge",
                        "Display line and column coordinates in status bar",
                        ctrl_sb_pos,
                        theme,
                        inner_border_color,
                    ));

                    let sub4_ed = cx.entity().downgrade();
                    let ctrl_sb_sidebar = Switch::new("switch-sb-sidebar")
                        .checked(self.area_layout.pref_show_sidebar_toggle)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub4_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_sidebar_toggle = !ed.area_layout.pref_show_sidebar_toggle;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Sidebar Toggle Button",
                        "Display button to toggle file tree sidebar in status bar",
                        ctrl_sb_sidebar,
                        theme,
                        inner_border_color,
                    ));

                    let sub5_ed = cx.entity().downgrade();
                    let ctrl_sb_mode = Switch::new("switch-sb-mode")
                        .checked(self.area_layout.pref_show_mode_switch)
                        .on_click(move |_ev, _win, cx| {
                            let _ = sub5_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_mode_switch = !ed.area_layout.pref_show_mode_switch;
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Mode Switch Button",
                        "Display button to switch Edit/Preview modes in status bar",
                        ctrl_sb_mode,
                        theme,
                        inner_border_color,
                    ));
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-sb",
                    "Status Bar Options",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
            SettingsTab::Editing => {
                // Section 1: Typography & Formatting
                let sec1_key = "typography";
                let is_sec1_expanded = self.area_layout.settings_expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let font_dec = cx.entity().downgrade();
                    let font_inc = cx.entity().downgrade();
                    let font_ctr = cx.entity().downgrade();
                    let curr_size = self.area_layout.pref_font_size;
                    let is_editing_font = self.area_layout.editing_settings_stepper.as_deref() == Some("font");

                    let ctrl_font = render_zed_stepper(
                        "font-dec",
                        "font-inc",
                        format!("{}", curr_size),
                        "px",
                        is_editing_font,
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_dec.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = None;
                                if ed.area_layout.pref_font_size > 8 {
                                    ed.area_layout.pref_font_size -= 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_inc.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = None;
                                if ed.area_layout.pref_font_size < 48 {
                                    ed.area_layout.pref_font_size += 1;
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = font_ctr.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = Some("font".to_string());
                                ed.area_layout.pref_font_size = match ed.area_layout.pref_font_size {
                                    12 => 14,
                                    14 => 16,
                                    16 => 18,
                                    18 => 20,
                                    20 => 24,
                                    24 => 12,
                                    _ => 14,
                                };
                                cx.notify();
                            });
                        }),
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Editor Font Size",
                        "Baseline font size in pixels for text editor content",
                        ctrl_font,
                        theme,
                        inner_border_color,
                    ));

                    let lh_dec = cx.entity().downgrade();
                    let lh_inc = cx.entity().downgrade();
                    let lh_ctr = cx.entity().downgrade();
                    let curr_lh = self.area_layout.pref_line_height;
                    let is_editing_lh = self.area_layout.editing_settings_stepper.as_deref() == Some("line_height");

                    let ctrl_lh = render_zed_stepper(
                        "lh-dec",
                        "lh-inc",
                        format!("{:.1}", curr_lh),
                        "",
                        is_editing_lh,
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_dec.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = None;
                                if ed.area_layout.pref_line_height > 1.05 {
                                    ed.area_layout.pref_line_height =
                                        (ed.area_layout.pref_line_height - 0.1).max(1.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_inc.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = None;
                                if ed.area_layout.pref_line_height < 3.0 {
                                    ed.area_layout.pref_line_height =
                                        (ed.area_layout.pref_line_height + 0.1).min(3.0);
                                    cx.notify();
                                }
                            });
                        }),
                        Box::new(move |_ev, _win, cx| {
                            let _ = lh_ctr.update(cx, |ed, cx| {
                                ed.area_layout.editing_settings_stepper = Some("line_height".to_string());
                                ed.area_layout.pref_line_height = if (ed.area_layout.pref_line_height - 1.2).abs() < 0.05 {
                                    1.4
                                } else if (ed.area_layout.pref_line_height - 1.4).abs() < 0.05 {
                                    1.6
                                } else if (ed.area_layout.pref_line_height - 1.6).abs() < 0.05 {
                                    1.8
                                } else if (ed.area_layout.pref_line_height - 1.8).abs() < 0.05 {
                                    2.0
                                } else {
                                    1.2
                                };
                                cx.notify();
                            });
                        }),
                        theme,
                    );

                    sec1_items.push(make_row(
                        "Line Height Multiplier",
                        "Adjust vertical line spacing ratio for reading comfort",
                        ctrl_lh,
                        theme,
                        inner_border_color,
                    ));
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-typo",
                    "Typography & Formatting",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Markdown & Assets
                let sec2_key = "markdown";
                let is_sec2_expanded = self.area_layout.settings_expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                let img_ed = cx.entity().downgrade();
                let img_options = [
                    (0, "Save to Local Assets"),
                    (1, "Copy to Document Folder"),
                    (2, "Insert Direct Link"),
                ];
                let curr_img_idx = self.area_layout.pref_image_paste_action % img_options.len();
                let curr_img_label = img_options[curr_img_idx].1;
                let is_img_open = self.area_layout.open_settings_dropdown.as_deref() == Some("image");

                if is_sec2_expanded {
                    let tbl_ed = cx.entity().downgrade();
                    let ctrl_tbl = Switch::new("switch-table-headers")
                        .checked(self.area_layout.pref_show_table_headers)
                        .on_click(move |_ev, _win, cx| {
                            let _ = tbl_ed.update(cx, |ed, cx| {
                                ed.area_layout.pref_show_table_headers =
                                    !ed.area_layout.pref_show_table_headers;
                                crate::config::settings::EditorSettings::set_show_table_headers(
                                    cx,
                                    ed.area_layout.pref_show_table_headers,
                                );
                                cx.notify();
                            });
                        })
                        .into_any_element();

                    sec2_items.push(make_row(
                        "Table Column Headers",
                        "Automatically render header row when formatting markdown tables",
                        ctrl_tbl,
                        theme,
                        inner_border_color,
                    ));

                    let mut img_btn_wrap = div()
                        .relative()
                        .child(
                            div()
                                .id("pref-btn-img")
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .w(px(145.0))
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(d.menu_item_radius))
                                .bg(c.dialog_secondary_button_bg)
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .border_1()
                                .border_color(c.dialog_border)
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .truncate()
                                        .child(curr_img_label),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .pl(px(4.0))
                                        .child(
                                            svg()
                                                .path("icon/panel/select-chevron.svg")
                                                .size(px(14.0))
                                                .text_color(c.dialog_muted),
                                        ),
                                )
                                .on_click({
                                    let img_ed = img_ed.clone();
                                    move |_ev, _win, cx| {
                                        let _ = img_ed.update(cx, |ed, cx| {
                                            if ed.area_layout.open_settings_dropdown.as_deref() == Some("image") {
                                                ed.area_layout.open_settings_dropdown = None;
                                            } else {
                                                ed.area_layout.open_settings_dropdown = Some("image".to_string());
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                        );

                    if is_img_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in img_options {
                            let is_selected = idx == curr_img_idx;
                            let item_ed = img_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("img-item-{}", idx).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.area_layout.pref_image_paste_action = idx;
                                            ed.area_layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        img_btn_wrap = img_btn_wrap.child(
                            gpui::deferred(
                                div()
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt(px(4.0))
                                    .w(px(160.0))
                                    .occlude()
                                    .bg(c.dialog_surface)
                                    .border_1()
                                    .border_color(c.dialog_border)
                                    .rounded(px(6.0))
                                    .shadow_lg()
                                    .p(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .children(menu_items),
                            ),
                        );
                    }

                    sec2_items.push(make_row(
                        "Image Paste Action",
                        "Default storage location when pasting images into document",
                        img_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-md",
                    "Markdown & Assets",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));

                // Section 3: Startup Behavior
                let sec3_key = "startup";
                let is_sec3_expanded = self.area_layout.settings_expanded_sections.contains(sec3_key);
                let mut sec3_items = Vec::new();

                let startup_ed = cx.entity().downgrade();
                let startup_options = [
                    (0, "New Blank Document"),
                    (1, "Open Last Opened File"),
                ];
                let curr_startup_idx = self.area_layout.pref_startup_option % startup_options.len();
                let curr_startup_label = startup_options[curr_startup_idx].1;
                let is_startup_open = self.area_layout.open_settings_dropdown.as_deref() == Some("startup");

                if is_sec3_expanded {
                    let mut startup_btn_wrap = div()
                        .relative()
                        .child(
                            div()
                                .id("pref-btn-startup")
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .justify_between()
                                .w(px(145.0))
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(d.menu_item_radius))
                                .bg(c.dialog_secondary_button_bg)
                                .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                .border_1()
                                .border_color(c.dialog_border)
                                .text_size(px(12.0))
                                .text_color(c.text_default)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .truncate()
                                        .child(curr_startup_label),
                                )
                                .child(
                                    div()
                                        .flex_shrink_0()
                                        .pl(px(4.0))
                                        .child(
                                            svg()
                                                .path("icon/panel/select-chevron.svg")
                                                .size(px(14.0))
                                                .text_color(c.dialog_muted),
                                        ),
                                )
                                .on_click({
                                    let startup_ed = startup_ed.clone();
                                    move |_ev, _win, cx| {
                                        let _ = startup_ed.update(cx, |ed, cx| {
                                            if ed.area_layout.open_settings_dropdown.as_deref() == Some("startup") {
                                                ed.area_layout.open_settings_dropdown = None;
                                            } else {
                                                ed.area_layout.open_settings_dropdown = Some("startup".to_string());
                                            }
                                            cx.notify();
                                        });
                                    }
                                }),
                        );

                    if is_startup_open {
                        let mut menu_items = Vec::new();
                        for (idx, label) in startup_options {
                            let is_selected = idx == curr_startup_idx;
                            let item_ed = startup_ed.clone();

                            menu_items.push(
                                div()
                                    .id(ElementId::Name(format!("startup-item-{}", idx).into()))
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .rounded(px(4.0))
                                    .bg(if is_selected { c.dialog_secondary_button_hover } else { c.dialog_surface })
                                    .hover(|this| this.bg(c.dialog_secondary_button_hover))
                                    .text_size(px(12.0))
                                    .text_color(c.text_default)
                                    .child(label)
                                    .child(if is_selected {
                                        svg()
                                            .path("icon/panel/check.svg")
                                            .size(px(13.0))
                                            .text_color(c.dialog_primary_button_bg)
                                            .into_any_element()
                                    } else {
                                        div().w(px(13.0)).into_any_element()
                                    })
                                    .on_click(move |_ev, _win, cx| {
                                        let _ = item_ed.update(cx, |ed, cx| {
                                            ed.area_layout.pref_startup_option = idx;
                                            ed.area_layout.open_settings_dropdown = None;
                                            cx.notify();
                                        });
                                    })
                                    .into_any_element(),
                            );
                        }

                        startup_btn_wrap = startup_btn_wrap.child(
                            gpui::deferred(
                                div()
                                    .absolute()
                                    .top_full()
                                    .right_0()
                                    .mt(px(4.0))
                                    .w(px(160.0))
                                    .occlude()
                                    .bg(c.dialog_surface)
                                    .border_1()
                                    .border_color(c.dialog_border)
                                    .rounded(px(6.0))
                                    .shadow_lg()
                                    .p(px(4.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .children(menu_items),
                            ),
                        );
                    }

                    sec3_items.push(make_row(
                        "On Startup",
                        "Choose default document state when launching Velotype editor",
                        startup_btn_wrap.into_any_element(),
                        theme,
                        inner_border_color,
                    ));
                }

                let sec3_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-startup",
                    "Startup Behavior",
                    is_sec3_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec3_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec3_key);
                            cx.notify();
                        });
                    }),
                    sec3_items,
                    theme,
                ));
            }
            SettingsTab::Keymap => {
                // Section 1: Document Actions
                let sec1_key = "doc_actions";
                let is_sec1_expanded = self.area_layout.settings_expanded_sections.contains(sec1_key);
                let mut sec1_items = Vec::new();

                if is_sec1_expanded {
                    let doc_shortcuts = [
                        ("Save Document", "Save active file changes to disk", "Ctrl + S"),
                        ("Save Document As", "Save active document with a new name", "Ctrl + Shift + S"),
                        ("New Window", "Open a new editor window instance", "Ctrl + N"),
                        ("Close Window", "Close the currently focused editor window", "Ctrl + W"),
                    ];

                    for (name, desc, sc) in doc_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec1_items.push(make_row(
                            *name,
                            *desc,
                            ctrl_sc,
                            theme,
                            inner_border_color,
                        ));
                    }
                }

                let sec1_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-doc-actions",
                    "Document Actions",
                    is_sec1_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec1_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec1_key);
                            cx.notify();
                        });
                    }),
                    sec1_items,
                    theme,
                ));

                // Section 2: Interface & View Controls
                let sec2_key = "view_controls";
                let is_sec2_expanded = self.area_layout.settings_expanded_sections.contains(sec2_key);
                let mut sec2_items = Vec::new();

                if is_sec2_expanded {
                    let view_shortcuts = [
                        ("Toggle View Mode", "Switch between Edit, Preview, and Dual view layouts", "Ctrl + M"),
                        ("Toggle Workspace Tree", "Show or collapse the left file navigation sidebar", "Ctrl + E"),
                        ("Quit Application", "Safely exit application and save session", "Ctrl + Q"),
                    ];

                    for (name, desc, sc) in view_shortcuts.iter() {
                        let ctrl_sc = div()
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(d.menu_item_radius))
                            .bg(c.dialog_secondary_button_hover)
                            .text_size(px(11.0))
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(c.text_default)
                            .child(*sc)
                            .into_any_element();

                        sec2_items.push(make_row(
                            *name,
                            *desc,
                            ctrl_sc,
                            theme,
                            inner_border_color,
                        ));
                    }
                }

                let sec2_ed = cx.entity().downgrade();
                sections.push(make_section(
                    "pref-sec-view-controls",
                    "Interface & View Controls",
                    is_sec2_expanded,
                    Box::new(move |_ev, _win, cx| {
                        let _ = sec2_ed.update(cx, |ed, cx| {
                            ed.area_layout.toggle_settings_section(sec2_key);
                            cx.notify();
                        });
                    }),
                    sec2_items,
                    theme,
                ));
            }
        }

        let right_content = div()
            .id("pref-right-content")
            .relative()
            .flex_1()
            .h_full()
            .p(px(14.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .children(sections);

        // --- Main Root Layout ---
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_row()
            .bg(c.editor_background)
            .child(left_nav)
            .child(right_content)
            .into_any_element()
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.install_close_guard(cx, window);
        self.apply_pending_focus(window, cx);
        self.apply_pending_scroll_into_view(window, cx);
        self.last_selection_snapshot = self.capture_source_selection_snapshot(cx);
        self.sync_pending_save(window, cx);
        self.sync_pending_save_as(window, cx);
        self.sync_pending_open_link(window, cx);
        self.sync_window_edited_state(window);

        let viewport_bounds = self.scroll_handle.bounds();
        let viewport_size = viewport_bounds.size;
        self.sync_scroll_viewport(viewport_size, cx);

        let theme = cx.global::<ThemeManager>().current_arc();
        let strings = cx.global::<I18nManager>().strings_arc();
        self.sync_window_title(window, &strings);

        let d = &theme.dimensions;
        let visible_blocks = self.document.visible_blocks().to_vec();
        let editor = cx.entity().downgrade();
        let has_menus = cx
            .get_menus()
            .map(|menus| !menus.is_empty())
            .unwrap_or(false);
        let titlebar_height = custom_titlebar_height(window, d);
        let _menu_bar_height =
            in_window_menu_bar_height_for_target_os(std::env::consts::OS, has_menus, d);
        let scroll_trigger_padding = (d.block_min_height * 0.75).max(16.0);
        let max_scroll_y = f32::from(self.scroll_handle.max_offset().height.max(px(0.0)));
        let viewport_height = f32::from(viewport_bounds.size.height.max(px(1.0)));
        // Extra room below the last block so the lowest line can be scrolled up
        // to the viewport center instead of being pinned to the bottom edge.
        let scroll_beyond_bottom = viewport_height * 0.5;
        let viewport_width = f32::from(viewport_bounds.size.width.max(px(1.0)));
        let has_overflow = max_scroll_y > 0.5;

        let centered_width = Self::centered_column_width(viewport_width, &theme.dimensions);
        let current_scroll_y = (-f32::from(self.scroll_handle.offset().y)).clamp(0.0, max_scroll_y);
        let scrollbar_geometry =
            Self::scrollbar_geometry(viewport_height, max_scroll_y, current_scroll_y);
        let track_height = scrollbar_geometry.track_height;
        let thumb_height = scrollbar_geometry.thumb_height;
        let thumb_top = scrollbar_geometry.thumb_top;

        let show_custom_scrollbar = has_overflow
            && (self.scrollbar_drag.is_some()
                || self.scrollbar_hovered
                || Instant::now() <= self.scrollbar_visible_until);

        // Spacing metadata is read on demand instead of pre-collected into a
        // Vec<RenderedRowSpacingInfo> sized to all visible blocks. For long
        // documents this skips a ~tens-of-KB allocation per frame; per-block
        // entity.read_with is a cheap immutable lock + 7-field struct copy.
        let spacing_for = |index: usize| -> RenderedRowSpacingInfo {
            visible_blocks[index]
                .entity
                .read_with(cx, |block, _cx| RenderedRowSpacingInfo::from_block(block))
        };
        let mut previous_row_spacing = None;
        // One entry per render row; off-screen rows are dropped after windowing.
        let mut row_elements: Vec<AnyElement> = Vec::new();
        let mut row_starts: Vec<usize> = Vec::new();
        // Each row's leading `mt` gap; the top spacer subtracts the first mounted
        // row's, since that row re-applies it.
        let mut row_top_gaps: Vec<f32> = Vec::new();
        let mut index = 0usize;
        while index < visible_blocks.len() {
            let first_visible = visible_blocks[index].clone();
            let first_spacing = spacing_for(index);
            let top_gap = rendered_row_top_gap(previous_row_spacing, first_spacing, d.block_gap);

            if let (Some(callout_anchor), Some(callout_variant)) =
                (first_spacing.callout_anchor, first_spacing.callout_variant)
            {
                let mut group_children = Vec::new();
                let mut group_end = index;
                let mut previous_callout_row = None;
                while group_end < visible_blocks.len()
                    && spacing_for(group_end).callout_anchor == Some(callout_anchor)
                {
                    let row_spacing = spacing_for(group_end);
                    if let Some(footnote_anchor) = row_spacing.footnote_anchor {
                        let mut footnote_children = Vec::new();
                        let mut footnote_end = group_end;
                        let mut previous_footnote_row = None;
                        while footnote_end < visible_blocks.len()
                            && spacing_for(footnote_end).callout_anchor == Some(callout_anchor)
                            && spacing_for(footnote_end).footnote_anchor == Some(footnote_anchor)
                        {
                            let footnote_spacing = spacing_for(footnote_end);
                            let entity = visible_blocks[footnote_end].entity.clone();
                            let row = div()
                                .w_full()
                                .flex_shrink_0()
                                .mt(px(footnote_row_top_gap(previous_footnote_row, d.block_gap)))
                                .child(entity.clone());
                            let row = if self.view_mode == super::ViewMode::Rendered {
                                let row_editor = editor.clone();
                                let entity_id = entity.entity_id();
                                row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                    let _ = row_editor.update(cx, |editor, cx| {
                                        editor.on_block_context_menu_mouse_down(
                                            entity_id, event, window, cx,
                                        );
                                    });
                                })
                            } else {
                                row
                            };
                            footnote_children.push(row.into_any_element());
                            previous_footnote_row = Some(footnote_spacing);
                            footnote_end += 1;
                        }

                        group_children.push(
                            div()
                                .w_full()
                                .flex_shrink_0()
                                .mt(px(callout_row_top_gap(
                                    previous_callout_row,
                                    row_spacing,
                                    d,
                                )))
                                .child(footnote_group_shell(footnote_children, &theme, d))
                                .into_any_element(),
                        );
                        previous_callout_row = Some(spacing_for(footnote_end - 1));
                        group_end = footnote_end;
                        continue;
                    }

                    let entity = visible_blocks[group_end].entity.clone();
                    let row = div()
                        .w_full()
                        .flex_shrink_0()
                        .mt(px(callout_row_top_gap(
                            previous_callout_row,
                            row_spacing,
                            d,
                        )))
                        .child(entity.clone());
                    let row = if self.view_mode == super::ViewMode::Rendered {
                        let row_editor = editor.clone();
                        let entity_id = entity.entity_id();
                        row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            let _ = row_editor.update(cx, |editor, cx| {
                                editor
                                    .on_block_context_menu_mouse_down(entity_id, event, window, cx);
                            });
                        })
                    } else {
                        row
                    };
                    group_children.push(row.into_any_element());
                    previous_callout_row = Some(row_spacing);
                    group_end += 1;
                }

                let (accent, _background) = callout_colors(callout_variant, &theme);
                row_starts.push(index);
                row_top_gaps.push(top_gap);
                row_elements.push(
                    div()
                        .w(px(centered_width))
                        .max_w(relative(1.0))
                        .flex_shrink_0()
                        .mt(px(top_gap))
                        .flex()
                        .flex_col()
                        .gap(px(0.0))
                        .px(px(d.callout_padding_x))
                        .py(px(d.callout_padding_y))
                        .rounded_r(px(d.callout_radius))
                        .border_l(px(d.callout_border_width))
                        .border_color(accent)
                        .children(group_children)
                        .into_any_element(),
                );
                previous_row_spacing = Some(spacing_for(group_end - 1));
                index = group_end;
                continue;
            }

            if let Some(footnote_anchor) = first_spacing.footnote_anchor {
                let mut group_children = Vec::new();
                let mut group_end = index;
                let mut previous_footnote_row = None;
                while group_end < visible_blocks.len()
                    && spacing_for(group_end).footnote_anchor == Some(footnote_anchor)
                {
                    let row_spacing = spacing_for(group_end);
                    let entity = visible_blocks[group_end].entity.clone();
                    let row = div()
                        .w_full()
                        .flex_shrink_0()
                        .mt(px(footnote_row_top_gap(previous_footnote_row, d.block_gap)))
                        .child(entity.clone());
                    let row = if self.view_mode == super::ViewMode::Rendered {
                        let row_editor = editor.clone();
                        let entity_id = entity.entity_id();
                        row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                            let _ = row_editor.update(cx, |editor, cx| {
                                editor
                                    .on_block_context_menu_mouse_down(entity_id, event, window, cx);
                            });
                        })
                    } else {
                        row
                    };
                    group_children.push(row.into_any_element());
                    previous_footnote_row = Some(row_spacing);
                    group_end += 1;
                }

                row_starts.push(index);
                row_top_gaps.push(top_gap);
                row_elements.push(
                    div()
                        .w(px(centered_width))
                        .max_w(relative(1.0))
                        .flex_shrink_0()
                        .mt(px(top_gap))
                        .child(footnote_group_shell(group_children, &theme, d))
                        .into_any_element(),
                );
                previous_row_spacing = Some(spacing_for(group_end - 1));
                index = group_end;
                continue;
            }

            let entity = first_visible.entity.clone();
            let row = div()
                .w(px(centered_width))
                .max_w(relative(1.0))
                .flex_shrink_0()
                .mt(px(top_gap))
                .child(entity.clone());
            let row = if self.view_mode == super::ViewMode::Rendered {
                let row_editor = editor.clone();
                let entity_id = entity.entity_id();
                row.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    let _ = row_editor.update(cx, |editor, cx| {
                        editor.on_block_context_menu_mouse_down(entity_id, event, window, cx);
                    });
                })
            } else {
                row
            };
            row_starts.push(index);
            row_top_gaps.push(top_gap);
            row_elements.push(row.into_any_element());
            previous_row_spacing = Some(first_spacing);
            index += 1;
        }

        // The focused row is always kept mounted so its caret is not blurred; a
        // table cell maps to its containing table block's row.
        let focus_row = self
            .focused_edit_target_entity_id(window, cx)
            .and_then(|id| {
                self.document.visible_index_for_entity_id(id).or_else(|| {
                    self.table_cell_binding(id).and_then(|binding| {
                        self.document
                            .visible_index_for_entity_id(binding.table_block.entity_id())
                    })
                })
            })
            .map(|visible_index| {
                row_starts
                    .partition_point(|&start| start <= visible_index)
                    .saturating_sub(1)
            });

        // A row's first block keys its cached height; its painted top (from last
        // frame) feeds the footprints below.
        let row_first_ids: Vec<EntityId> = row_starts
            .iter()
            .map(|&start| visible_blocks[start].entity.entity_id())
            .collect();
        let row_tops: Vec<Option<f32>> = row_starts
            .iter()
            .map(|&start| {
                visible_blocks[start]
                    .entity
                    .read_with(cx, |block, _cx| block.last_bounds)
                    .map(|bounds| f32::from(bounds.top()))
            })
            .collect();

        // On a structural edit the row indices no longer match last frame, so the
        // cache refresh below is skipped; its block-keyed entries still hold.
        let structural_change = visible_blocks.len() != self.prev_visible_block_ids.len()
            || visible_blocks
                .iter()
                .zip(&self.prev_visible_block_ids)
                .any(|(visible, prev)| visible.entity.entity_id() != *prev);
        if structural_change {
            self.prev_visible_block_ids = visible_blocks
                .iter()
                .map(|v| v.entity.entity_id())
                .collect();
        }

        // Rows mounted together last frame shared one scroll offset, so their
        // adjacent painted-top differences are scroll-free heights. Caching those,
        // not raw positions, is what keeps the window stable while scrolling.
        if !structural_change {
            if let Some((prev_start, prev_end)) = self.prev_render_window {
                let prev_end = prev_end.min(row_first_ids.len());
                for row in prev_start..prev_end.saturating_sub(1) {
                    if let (Some(top), Some(next_top)) = (row_tops[row], row_tops[row + 1]) {
                        let stride = next_top - top;
                        if stride > 0.0 && stride.is_finite() {
                            self.row_stride_cache.insert(row_first_ids[row], stride);
                        }
                    }
                }
            }
        }

        // Unmeasured rows use the minimum block height: a lower bound, so the
        // window over-mounts rather than ever landing on a spacer.
        let estimate = d.block_min_height.max(1.0);
        let strides: Vec<f32> = row_first_ids
            .iter()
            .map(|id| self.row_stride_cache.get(id).copied().unwrap_or(estimate))
            .collect();

        // Bound the cache against block churn, only when it outgrows the live rows.
        if self.row_stride_cache.len() > row_first_ids.len().saturating_mul(2) {
            let live: std::collections::HashSet<EntityId> = row_first_ids.iter().copied().collect();
            self.row_stride_cache.retain(|id, _| live.contains(id));
        }

        let render_window = Self::rendered_window(
            &strides,
            current_scroll_y,
            viewport_height,
            RENDER_OVERDRAW_PX,
            focus_row,
        );
        self.prev_render_window = Some((render_window.run_start, render_window.run_end));

        // The first mounted row re-applies its `mt`, so drop it from the top
        // spacer to avoid shifting content down by a gap.
        let top_h = match row_top_gaps.get(render_window.run_start) {
            Some(gap) => (render_window.top_h - gap).max(0.0),
            None => render_window.top_h,
        };
        let mut block_rows: Vec<AnyElement> =
            Vec::with_capacity(render_window.run_end - render_window.run_start + 2);
        if top_h > 0.5 {
            block_rows.push(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .h(px(top_h))
                    .into_any_element(),
            );
        }
        for (row_index, element) in row_elements.into_iter().enumerate() {
            if row_index >= render_window.run_start && row_index < render_window.run_end {
                block_rows.push(element);
            }
        }
        if render_window.bottom_h > 0.5 {
            block_rows.push(
                div()
                    .w_full()
                    .flex_shrink_0()
                    .h(px(render_window.bottom_h))
                    .into_any_element(),
            );
        }

        let scroll_content = div()
            .id("editor-scroll-inner")
            .flex()
            .flex_col()
            .flex_grow()
            .h_full()
            .items_center()
            .bg(theme.colors.editor_background)
            .overflow_y_scroll()
            .scrollbar_width(px(0.0))
            .track_scroll(&self.scroll_handle)
            .on_hover(cx.listener(Self::on_editor_hover))
            .capture_any_mouse_down(cx.listener(Self::on_editor_capture_mouse_down))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_editor_mouse_down))
            .on_mouse_move(cx.listener(Self::on_editor_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_editor_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_editor_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_editor_scroll_wheel))
            .p(px(d.editor_padding))
            .pb(px(d.editor_padding
                + scroll_trigger_padding
                + scroll_beyond_bottom))
            .children(block_rows);
        let scroll_content = if self.view_mode == super::ViewMode::Rendered {
            scroll_content.on_mouse_down(
                MouseButton::Right,
                cx.listener(Self::on_editor_context_menu_mouse_down),
            )
        } else {
            scroll_content
        };

        let content_area = div()
            .id("editor-scroll")
            .w_full()
            .h_full()
            .flex_1()
            .min_w(px(0.0))
            .bg(theme.colors.editor_background)
            .relative()
            .child(scroll_content);

        let content_area = if show_custom_scrollbar {
            let scrollbar_editor = editor.clone();
            let track_origin_y = f32::from(viewport_bounds.origin.y);
            content_area.child(
                div()
                    .id("editor-scrollbar-thumb")
                    .absolute()
                    .occlude()
                    .top(px(thumb_top))
                    .right(px(d.scrollbar_right))
                    .w(px(d.scrollbar_width))
                    .h(px(thumb_height))
                    .rounded(px(999.0))
                    .bg(theme.colors.scrollbar_thumb)
                    .cursor_pointer()
                    .on_hover(cx.listener(Self::on_editor_hover))
                    .on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                        let pointer_offset_y =
                            f32::from(event.position.y) - track_origin_y - thumb_top;
                        let _ = scrollbar_editor.update(cx, |editor, cx| {
                            cx.stop_propagation();
                            editor.start_scrollbar_drag(
                                pointer_offset_y,
                                track_height,
                                thumb_height,
                                max_scroll_y,
                                cx,
                            );
                        });
                    })
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |_thumb_bounds, _, window, _| {
                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |_event: &MouseUpEvent, phase, _window, cx| {
                                        if !phase.bubble() {
                                            return;
                                        }
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.end_scrollbar_drag(cx);
                                        });
                                    }
                                });

                                window.on_mouse_event({
                                    let editor = editor.clone();
                                    move |event: &MouseMoveEvent, phase, _window, cx| {
                                        if !phase.bubble() || !event.dragging() {
                                            return;
                                        }

                                        let pointer_y_in_track =
                                            f32::from(event.position.y) - track_origin_y;
                                        let _ = editor.update(cx, |editor, cx| {
                                            editor.update_scrollbar_drag(pointer_y_in_track, cx);
                                        });
                                    }
                                });
                            },
                        )
                        .size_full(),
                    ),
            )
        } else {
            content_area
        };

        // Repaint when the Cmd/Ctrl follow modifier toggles so a hovered link's
        // hand cursor updates without moving the pointer. `ModifiersChanged` is
        // dispatched along the focused element's path to the root, and this root
        // is an ancestor of every block, so one listener here covers a link in any
        // block while editing. Gated to the secondary modifier so Shift during
        // selection does not repaint.
        let follow_modifier_active = window.modifiers().secondary();

        let base = div()
            .w_full()
            .h_full()
            .flex()
            .flex_col()
            .relative()
            .bg(theme.colors.editor_background)
            .font(editor_text_font())
            .on_modifiers_changed(move |event, window, _| {
                if event.modifiers.secondary() != follow_modifier_active {
                    window.refresh();
                }
            })
            .capture_action(cx.listener(Self::on_copy_capture))
            .capture_action(cx.listener(Self::on_cut_capture))
            .capture_action(cx.listener(Self::on_delete_capture))
            .capture_action(cx.listener(Self::on_delete_back_capture))
            .capture_key_down(cx.listener(Self::on_editor_key_down_capture))
            .can_drop(|dragged, _window, _cx| dragged.is::<ExternalPaths>())
            .on_drop::<ExternalPaths>(cx.listener(Self::on_external_paths_drop))
            .on_action(cx.listener(Self::on_undo))
            .on_action(cx.listener(Self::on_redo))
            .on_action(cx.listener(Self::on_save_document))
            .on_action(cx.listener(Self::on_save_document_as))
            .on_action(cx.listener(Self::on_export_html))
            .on_action(cx.listener(Self::on_export_pdf))
            .on_action(cx.listener(Self::on_quit_application))
            .on_action(cx.listener(Self::on_close_window))
            .on_action(cx.listener(Self::on_toggle_view_mode_action))
            .on_action(cx.listener(Self::on_toggle_workspace_action))
            .on_action(cx.listener(Self::on_page_up))
            .on_action(cx.listener(Self::on_page_down))
            .on_action(cx.listener(Self::on_jump_to_top))
            .on_action(cx.listener(Self::on_jump_to_bottom))
            .on_action(cx.listener(Self::on_dismiss_transient_ui))
            .on_action(cx.listener(Self::on_install_cli_tool))
            .on_action(cx.listener(Self::on_uninstall_cli_tool));
        // Fetch menus + collect labels once for both renderers; previously each
        // of render_in_window_menu_bar / render_in_window_menu_panel called
        // cx.get_menus() and walked menus.iter().map(|m| m.name.to_string())
        // independently — two redundant Vec<OwnedMenu> + two redundant
        // Vec<String>-of-N-allocations per frame.
        let menus = supports_in_window_menu()
            .then(|| cx.get_menus())
            .flatten()
            .filter(|m| !m.is_empty());
        let menu_labels: Vec<SharedString> = menus
            .as_ref()
            .map(|m| m.iter().map(|menu| menu.name.clone()).collect())
            .unwrap_or_default();
        let window_title =
            Self::window_title(self.file_path.as_deref(), self.document_dirty, &strings);
        let inline_menu =
            self.render_inline_titlebar_menu(&theme, cx, menus.as_deref(), &menu_labels);
        let base = if let Some(titlebar) = render_custom_titlebar(
            "editor-titlebar",
            window_title.into(),
            inline_menu,
            &theme,
            window,
            cx,
            Self::on_titlebar_close,
        ) {
            base.child(titlebar)
        } else {
            base
        };
        let main_content = div()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .pt(px(titlebar_height))
            .flex()
            .min_w(px(0.0))
            .child(self.render_tiled_layout(
                content_area.into_any_element(),
                &theme,
                &strings,
                window,
                cx,
            ));
        let base = base.child(main_content);
        let base = if let Some(status_bar) = self.render_status_bar(&theme, &strings, window, cx) {
            base.child(status_bar)
        } else {
            base
        };
        let base = if let Some(menu_panel) = self.render_in_window_menu_panel(
            &theme,
            cx,
            menus.as_deref(),
            &menu_labels,
            titlebar_height,
            f32::from(window.viewport_size().height.max(px(1.0))),
        ) {
            base.child(menu_panel)
        } else {
            base
        };
        let base = if let Some(context_menu) = self.render_context_menu_overlay(&theme, cx) {
            base.child(context_menu)
        } else {
            base
        };
        let base = if let Some(table_dialog) = self.render_table_insert_dialog_overlay(&theme, cx) {
            base.child(table_dialog)
        } else {
            base
        };
        if let Some(kind) = self.info_dialog {
            base.child(self.render_info_dialog_overlay(&theme, kind, cx))
        } else if self.show_drop_replace_dialog {
            base.child(self.render_drop_replace_overlay(&theme, cx))
        } else if self.show_unsaved_changes_dialog {
            base.child(self.render_unsaved_changes_overlay(&theme, cx))
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NoRecentFiles, RenderedRowSpacingInfo, callout_row_top_gap, editor_text_font,
        import_menu_split_index, in_window_menu_bar_height_for_target_os, menu_bar_button_width,
        menu_items_visual_height_with_gaps, menu_panel_left, menu_panel_width_for_labels,
        owned_menu_item_labels, rendered_row_top_gap, scrollable_import_menu_scroll_height,
        submenu_bridge_geometry, supports_in_window_menu_for_target_os,
        tibetan_font_fallbacks_for_target_os,
    };
    use crate::components::{AddLanguageConfig, AddThemeConfig};
    use crate::theme::Theme;
    use gpui::{OwnedMenu, OwnedMenuItem};
    use uuid::Uuid;

    fn disabled_menu_action(name: &str) -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: name.into(),
            action: Box::new(NoRecentFiles),
            os_action: None,
        }
    }

    fn add_theme_menu_action() -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: "Add Theme Config".into(),
            action: Box::new(AddThemeConfig),
            os_action: None,
        }
    }

    fn add_language_menu_action() -> OwnedMenuItem {
        OwnedMenuItem::Action {
            name: "Add Language Config".into(),
            action: Box::new(AddLanguageConfig),
            os_action: None,
        }
    }

    #[test]
    fn contiguous_quote_rows_collapse_inter_row_gap() {
        let group = Uuid::new_v4();
        let gap = rendered_row_top_gap(
            Some(RenderedRowSpacingInfo {
                quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo {
                quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn editor_text_font_keeps_system_ui_as_primary_family() {
        assert_eq!(editor_text_font().family.to_string(), ".SystemUIFont");
    }

    #[test]
    fn tibetan_font_fallbacks_prioritize_platform_defaults() {
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("windows")
                .first()
                .map(String::as_str),
            Some("Microsoft Himalaya")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("macos")
                .first()
                .map(String::as_str),
            Some("Kailasa")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("linux")
                .first()
                .map(String::as_str),
            Some("Noto Serif Tibetan")
        );
        assert_eq!(
            tibetan_font_fallbacks_for_target_os("unknown")
                .first()
                .map(String::as_str),
            Some("Noto Serif Tibetan")
        );
    }

    #[test]
    fn nested_quote_separator_row_keeps_outer_group_gap_collapsed() {
        let group = Uuid::new_v4();
        let gap = rendered_row_top_gap(
            Some(RenderedRowSpacingInfo {
                quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo {
                quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn distinct_quote_groups_keep_default_gap() {
        let gap = rendered_row_top_gap(
            Some(RenderedRowSpacingInfo {
                quote_group_anchor: Some(Uuid::new_v4()),
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo {
                quote_group_anchor: Some(Uuid::new_v4()),
                ..RenderedRowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 4.0);
    }

    #[test]
    fn non_quote_rows_keep_default_gap() {
        let gap = rendered_row_top_gap(
            Some(RenderedRowSpacingInfo {
                quote_group_anchor: None,
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo {
                quote_group_anchor: Some(Uuid::new_v4()),
                ..RenderedRowSpacingInfo::default()
            },
            4.0,
        );
        assert_eq!(gap, 4.0);
    }

    #[test]
    fn callout_inner_spacing_uses_header_and_body_tokens() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;

        let header_gap = callout_row_top_gap(
            Some(RenderedRowSpacingInfo {
                is_callout_header: true,
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo::default(),
            dimensions,
        );
        let body_gap = callout_row_top_gap(
            Some(RenderedRowSpacingInfo {
                is_callout_header: false,
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo::default(),
            dimensions,
        );

        assert_eq!(header_gap, dimensions.callout_header_margin_bottom);
        assert_eq!(body_gap, dimensions.callout_body_gap);
    }

    #[test]
    fn nested_quote_rows_inside_callout_collapse_body_gap() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let group = Uuid::new_v4();

        let gap = callout_row_top_gap(
            Some(RenderedRowSpacingInfo {
                is_callout_header: false,
                visible_quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            }),
            RenderedRowSpacingInfo {
                visible_quote_group_anchor: Some(group),
                ..RenderedRowSpacingInfo::default()
            },
            dimensions,
        );

        assert_eq!(gap, 0.0);
    }

    #[test]
    fn menu_button_width_expands_for_long_ascii_labels() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;

        assert_eq!(
            menu_bar_button_width("文件", dimensions),
            dimensions.menu_bar_button_width
        );
        assert!(menu_bar_button_width("Language", dimensions) > dimensions.menu_bar_button_width);
    }

    #[test]
    fn in_window_menu_is_enabled_for_every_target_except_macos() {
        for target_os in [
            "windows",
            "linux",
            "freebsd",
            "openbsd",
            "netbsd",
            "dragonfly",
            "solaris",
            "illumos",
            "android",
            "unknown",
        ] {
            assert!(
                supports_in_window_menu_for_target_os(target_os),
                "{target_os} should use the in-window fallback menu"
            );
        }
        assert!(!supports_in_window_menu_for_target_os("macos"));
    }

    #[test]
    fn in_window_menu_height_depends_on_platform_and_menu_presence() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;

        assert_eq!(
            in_window_menu_bar_height_for_target_os("linux", true, dimensions),
            dimensions.menu_bar_height
        );
        assert_eq!(
            in_window_menu_bar_height_for_target_os("windows", true, dimensions),
            dimensions.menu_bar_height
        );
        assert_eq!(
            in_window_menu_bar_height_for_target_os("linux", false, dimensions),
            0.0
        );
        assert_eq!(
            in_window_menu_bar_height_for_target_os("macos", true, dimensions),
            0.0
        );
    }

    #[test]
    fn menu_panel_left_uses_accumulated_dynamic_button_widths() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec![
            "File".to_string(),
            "Language".to_string(),
            "Theme".to_string(),
            "Help".to_string(),
        ];

        let left = menu_panel_left(2, &labels, dimensions);
        let expected = dimensions.menu_bar_padding_x
            + menu_bar_button_width("File", dimensions)
            + dimensions.menu_bar_gap
            + menu_bar_button_width("Language", dimensions)
            + dimensions.menu_bar_gap;
        let old_fixed_left = dimensions.menu_bar_padding_x
            + 2.0 * (dimensions.menu_bar_button_width + dimensions.menu_bar_gap);

        assert_eq!(left, expected);
        assert!(left > old_fixed_left);
    }

    #[test]
    fn menu_panel_width_expands_for_long_recent_paths() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let short_labels = vec!["Save".to_string()];
        let long_labels = vec![r"C:\Users\someone\Documents\Very Long Folder\notes.md".to_string()];

        assert_eq!(
            menu_panel_width_for_labels(&short_labels, dimensions),
            dimensions.menu_panel_width
        );
        assert!(
            menu_panel_width_for_labels(&long_labels, dimensions) > dimensions.menu_panel_width
        );
    }

    #[test]
    fn import_menu_split_detects_theme_and_language_import_tails() {
        let theme_items = vec![
            disabled_menu_action("Velotype"),
            OwnedMenuItem::Separator,
            add_theme_menu_action(),
        ];
        let language_items = vec![
            disabled_menu_action("English"),
            OwnedMenuItem::Separator,
            add_language_menu_action(),
        ];
        let regular_items = vec![
            disabled_menu_action("Open"),
            OwnedMenuItem::Separator,
            disabled_menu_action("Save"),
        ];
        let malformed_import_items =
            vec![disabled_menu_action("Velotype"), add_theme_menu_action()];

        assert_eq!(import_menu_split_index(&theme_items), Some(1));
        assert_eq!(import_menu_split_index(&language_items), Some(1));
        assert_eq!(import_menu_split_index(&regular_items), None);
        assert_eq!(import_menu_split_index(&malformed_import_items), None);
    }

    #[test]
    fn scrollable_import_menu_height_caps_visible_items_and_clamps_to_viewport() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let scroll_items = (0..20)
            .map(|index| disabled_menu_action(&format!("Custom Theme {index}")))
            .collect::<Vec<_>>();
        let footer_items = vec![OwnedMenuItem::Separator, add_theme_menu_action()];
        let expected_large_height =
            menu_items_visual_height_with_gaps(&scroll_items[..12], dimensions);
        let full_scroll_content_height =
            menu_items_visual_height_with_gaps(&scroll_items, dimensions);
        let footer_height = menu_items_visual_height_with_gaps(&footer_items, dimensions);

        let large_height = scrollable_import_menu_scroll_height(
            &scroll_items,
            &footer_items,
            2000.0,
            0.0,
            dimensions,
        );
        let small_height = scrollable_import_menu_scroll_height(
            &scroll_items,
            &footer_items,
            180.0,
            0.0,
            dimensions,
        );

        assert!((large_height - expected_large_height).abs() < f32::EPSILON);
        assert!(full_scroll_content_height > large_height);
        assert!(large_height < expected_large_height + footer_height);
        assert!(small_height < large_height);
        assert!(small_height >= dimensions.menu_item_height);
    }

    #[test]
    fn submenu_bridge_spans_parent_child_menu_gap() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec!["File".to_string()];
        let items = vec![
            OwnedMenuItem::Separator,
            OwnedMenuItem::Submenu(OwnedMenu {
                name: "Recent".into(),
                items: vec![OwnedMenuItem::Action {
                    name: r"C:\Users\someone\Documents\notes.md".into(),
                    action: Box::new(NoRecentFiles),
                    os_action: None,
                }],
            }),
        ];
        let submenu_labels = match &items[1] {
            OwnedMenuItem::Submenu(submenu) => owned_menu_item_labels(&submenu.items),
            _ => Vec::new(),
        };

        let bridge = submenu_bridge_geometry(0, &labels, &items, 1, &submenu_labels, dimensions)
            .expect("submenu bridge geometry should be available");
        let submenu_width = menu_panel_width_for_labels(&submenu_labels, dimensions);

        assert_eq!(
            bridge.left,
            dimensions.menu_bar_padding_x + dimensions.menu_panel_width
        );
        assert_eq!(bridge.width, dimensions.menu_panel_gap + submenu_width);
        assert!(bridge.height > dimensions.menu_item_height);
        let item_top = dimensions.menu_panel_top
            + dimensions.menu_panel_padding
            + dimensions.menu_separator_height
            + dimensions.menu_separator_margin_y * 2.0
            + dimensions.menu_panel_gap;
        assert!(bridge.top < item_top);
        assert!(bridge.top >= dimensions.menu_panel_top);
    }

    #[test]
    fn submenu_bridge_uses_dynamic_main_menu_width() {
        let theme = Theme::default_theme();
        let dimensions = &theme.dimensions;
        let labels = vec!["File".to_string()];
        let items = vec![OwnedMenuItem::Submenu(OwnedMenu {
            name: "Open Recently Used Markdown File".into(),
            items: vec![OwnedMenuItem::Action {
                name: r"C:\Users\someone\Documents\Very Long Folder\notes.md".into(),
                action: Box::new(NoRecentFiles),
                os_action: None,
            }],
        })];
        let submenu_labels = match &items[0] {
            OwnedMenuItem::Submenu(submenu) => owned_menu_item_labels(&submenu.items),
            _ => Vec::new(),
        };

        let bridge = submenu_bridge_geometry(0, &labels, &items, 0, &submenu_labels, dimensions)
            .expect("submenu bridge geometry should be available");

        assert!(bridge.left > dimensions.menu_bar_padding_x + dimensions.menu_panel_width);
        assert!(bridge.width > dimensions.menu_panel_gap + dimensions.menu_panel_width);
    }
}
