//! Search and text input component with unified styling and keyboard interaction.
//!
//! Provides a standardized search input with:
//! - Uniform styling matching the design system (28px height, rounded corners,
//!   secondary button background, 1px dialog border).
//! - Active bottom accent bar that lights up with `c.focus_accent` when focused
//!   or when a query is present.
//! - Accurate text cursor indicator when focused (at x=0 when placeholder is visible,
//!   matching Zed's picker behavior).
//! - Text selection highlight, mouse drag selection, double-click word selection,
//!   and triple-click select-all.
//! - Full keyboard navigation: Left/Right arrows (with Shift and Ctrl), Home/End,
//!   Backspace/Delete (with Ctrl word deletion), Enter (submit), Escape (dismiss/clear),
//!   and clipboard shortcuts (Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V).
//! - IME (Input Method Editor) support with accurate candidate window positioning.
//! - Optional clear button (×) when query is non-empty.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use gpui::*;
use theme::{Theme, ThemeColors, ThemeDimensions, ThemeManager};

/// Callback invoked when input text changes.
pub type InputChangeHandler = Arc<dyn Fn(String, &mut Window, &mut App) + 'static>;

/// Callback invoked on enter / submit.
pub type InputSubmitHandler = Arc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// Callback invoked on escape / dismiss.
pub type InputDismissHandler = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

// ── UTF-8 / UTF-16 Offset Conversions ─────────────────────────────────────────

pub fn utf16_to_utf8_in(text: &str, utf16_offset: usize) -> usize {
    let mut utf16_count = 0;
    let mut utf8_offset = 0;
    for ch in text.chars() {
        if utf16_count >= utf16_offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }
    utf8_offset
}

pub fn utf8_to_utf16_in(text: &str, utf8_offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;
    for ch in text.chars() {
        if utf8_count >= utf8_offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf8_to_utf16();
    }
    utf16_offset
}

trait Utf8ToUtf16Len {
    fn len_utf8_to_utf16(&self) -> usize;
}

impl Utf8ToUtf16Len for char {
    fn len_utf8_to_utf16(&self) -> usize {
        self.len_utf16()
    }
}

pub fn utf16_range_to_utf8_in(text: &str, range_utf16: &Range<usize>) -> Range<usize> {
    utf16_to_utf8_in(text, range_utf16.start)..utf16_to_utf8_in(text, range_utf16.end)
}

pub fn utf8_range_to_utf16_in(text: &str, range: &Range<usize>) -> Range<usize> {
    utf8_to_utf16_in(text, range.start)..utf8_to_utf16_in(text, range.end)
}

pub fn utf8_to_utf16_in_single(text: &str, utf8_offset: usize) -> usize {
    utf8_to_utf16_in(text, utf8_offset)
}

// ── Word Boundary Helpers ───────────────────────────────────────────────────

pub fn find_prev_word_boundary(text: &str, cursor: usize) -> usize {
    if cursor == 0 || text.is_empty() {
        return 0;
    }
    let cursor = cursor.min(text.len());
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let idx = chars.iter().rposition(|&(offset, _)| offset < cursor).unwrap_or(0);
    let mut i = idx;
    // Skip whitespace if starting on one
    while i > 0 && chars[i].1.is_whitespace() {
        i -= 1;
    }
    // Skip backwards through word characters
    while i > 0 && !chars[i - 1].1.is_whitespace() {
        i -= 1;
    }
    chars[i].0
}

pub fn find_next_word_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() || text.is_empty() {
        return text.len();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let idx = chars.iter().position(|&(offset, _)| offset >= cursor).unwrap_or(chars.len());
    let mut i = idx;
    // Skip word characters
    while i < chars.len() && !chars[i].1.is_whitespace() {
        i += 1;
    }
    // Skip trailing whitespace
    while i < chars.len() && chars[i].1.is_whitespace() {
        i += 1;
    }
    if i < chars.len() {
        chars[i].0
    } else {
        text.len()
    }
}

// ── Search Input Mutable State ──────────────────────────────────────────────

/// Persistent editing state for an active search input.
#[derive(Debug, Clone)]
pub struct SearchInputState {
    pub value: String,
    pub cursor_offset: usize,
    pub selection: Range<usize>,
    pub reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub last_line: Option<ShapedLine>,
    pub is_dragging: bool,
    pub drag_anchor: usize,
}

impl SearchInputState {
    pub fn new(value: String) -> Self {
        let len = value.len();
        Self {
            value,
            cursor_offset: len,
            selection: len..len,
            reversed: false,
            marked_range: None,
            last_bounds: None,
            last_line: None,
            is_dragging: false,
            drag_anchor: len,
        }
    }

    /// Synchronize state with incoming external value.
    pub fn sync_value(&mut self, new_value: String) {
        if self.value != new_value {
            self.value = new_value;
            let len = self.value.len();
            self.cursor_offset = self.cursor_offset.min(len);
            let start = self.selection.start.min(len);
            let end = self.selection.end.min(len);
            self.selection = start..end;
            self.marked_range = None;
        }
    }

    pub fn selection_range(&self) -> Range<usize> {
        if self.reversed {
            self.selection.end..self.selection.start
        } else {
            self.selection.start..self.selection.end
        }
    }

    pub fn selected_text(&self) -> &str {
        let range = self.selection_range();
        if range.start <= range.end && range.end <= self.value.len() {
            &self.value[range]
        } else {
            ""
        }
    }

    pub fn cursor(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    pub fn selection_anchor(&self) -> usize {
        if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        }
    }

    pub fn set_cursor(&mut self, cursor: usize, anchor: usize, extend: bool) {
        let cursor = cursor.min(self.value.len());
        let anchor = anchor.min(self.value.len());
        if extend {
            if cursor < anchor {
                self.selection = cursor..anchor;
                self.reversed = true;
            } else {
                self.selection = anchor..cursor;
                self.reversed = false;
            }
        } else {
            self.selection = cursor..cursor;
            self.reversed = false;
        }
        self.cursor_offset = cursor;
    }

    pub fn replace_range(&mut self, range: Range<usize>, new_text: &str) -> String {
        let start = range.start.min(self.value.len());
        let end = range.end.min(self.value.len());
        if start <= end {
            self.value.replace_range(start..end, new_text);
            let cursor = start + new_text.len();
            self.selection = cursor..cursor;
            self.reversed = false;
            self.marked_range = None;
            self.cursor_offset = cursor;
        }
        self.value.clone()
    }

    pub fn insert_text(&mut self, text: &str) -> String {
        let range = self.selection_range();
        self.replace_range(range, text)
    }

    pub fn delete_backward(&mut self) -> String {
        let range = self.selection_range();
        if !range.is_empty() {
            return self.replace_range(range, "");
        }
        let cursor = self.cursor();
        if cursor > 0 {
            let start = self.value.floor_char_boundary(cursor - 1);
            return self.replace_range(start..cursor, "");
        }
        self.value.clone()
    }

    pub fn delete_forward(&mut self) -> String {
        let range = self.selection_range();
        if !range.is_empty() {
            return self.replace_range(range, "");
        }
        let cursor = self.cursor();
        if cursor < self.value.len() {
            let end = self.value.ceil_char_boundary(cursor + 1).min(self.value.len());
            return self.replace_range(cursor..end, "");
        }
        self.value.clone()
    }

    pub fn delete_to_start(&mut self) -> String {
        let cursor = self.cursor();
        self.replace_range(0..cursor, "")
    }

    pub fn delete_word_backward(&mut self) -> String {
        let range = self.selection_range();
        if !range.is_empty() {
            return self.replace_range(range, "");
        }
        let cursor = self.cursor();
        let prev = find_prev_word_boundary(&self.value, cursor);
        self.replace_range(prev..cursor, "")
    }

    pub fn delete_word_forward(&mut self) -> String {
        let range = self.selection_range();
        if !range.is_empty() {
            return self.replace_range(range, "");
        }
        let cursor = self.cursor();
        let next = find_next_word_boundary(&self.value, cursor);
        self.replace_range(cursor..next, "")
    }

    pub fn move_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = self.selection_range().start;
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            self.cursor_offset = new_cursor;
            return;
        }
        let target = self.value.floor_char_boundary(cursor.saturating_sub(1));
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        if !extend && !self.selection_range().is_empty() {
            let new_cursor = self.selection_range().end;
            self.selection = new_cursor..new_cursor;
            self.reversed = false;
            self.cursor_offset = new_cursor;
            return;
        }
        let target = self.value.ceil_char_boundary(cursor + 1).min(self.value.len());
        self.set_cursor(target, anchor, extend);
    }

    pub fn move_word_left(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        let prev = find_prev_word_boundary(&self.value, cursor);
        self.set_cursor(prev, anchor, extend);
    }

    pub fn move_word_right(&mut self, extend: bool) {
        let cursor = self.cursor();
        let anchor = self.selection_anchor();
        let next = find_next_word_boundary(&self.value, cursor);
        self.set_cursor(next, anchor, extend);
    }

    pub fn move_home(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(0, anchor, extend);
    }

    pub fn move_end(&mut self, extend: bool) {
        let anchor = self.selection_anchor();
        self.set_cursor(self.value.len(), anchor, extend);
    }

    pub fn select_all(&mut self) {
        self.selection = 0..self.value.len();
        self.reversed = false;
        self.cursor_offset = self.value.len();
    }
}

thread_local! {
    static SEARCH_INPUT_STATES: RefCell<HashMap<ElementId, Rc<RefCell<SearchInputState>>>> =
        RefCell::new(HashMap::new());
}

fn get_or_create_input_state(id: &ElementId, initial_value: &str) -> Rc<RefCell<SearchInputState>> {
    SEARCH_INPUT_STATES.with(|states| {
        states
            .borrow_mut()
            .entry(id.clone())
            .or_insert_with(|| Rc::new(RefCell::new(SearchInputState::new(initial_value.to_string()))))
            .clone()
    })
}

// ── SearchInput Component ───────────────────────────────────────────────────

/// A standardized search input element with styling and interaction primitives.
#[derive(IntoElement)]
pub struct SearchInput {
    id: ElementId,
    value: SharedString,
    placeholder: SharedString,
    focus_handle: FocusHandle,
    autofocus: bool,
    key_context: Option<SharedString>,
    show_clear_button: bool,
    custom_colors: Option<ThemeColors>,
    custom_dimensions: Option<ThemeDimensions>,
    on_change: Option<InputChangeHandler>,
    on_submit: Option<InputSubmitHandler>,
    on_dismiss: Option<InputDismissHandler>,
}

impl SearchInput {
    /// Constructs a new search input.
    pub fn new(
        id: impl Into<ElementId>,
        value: impl Into<SharedString>,
        focus_handle: FocusHandle,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: SharedString::from("Search…"),
            focus_handle,
            autofocus: false,
            key_context: None,
            show_clear_button: true,
            custom_colors: None,
            custom_dimensions: None,
            on_change: None,
            on_submit: None,
            on_dismiss: None,
        }
    }

    /// Sets the placeholder text displayed when the input is empty.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets whether the input should automatically focus itself on mount.
    pub fn autofocus(mut self, autofocus: bool) -> Self {
        self.autofocus = autofocus;
        self
    }

    /// Sets a custom key context string (defaults to `"SearchInput"`).
    pub fn key_context(mut self, context: impl Into<SharedString>) -> Self {
        self.key_context = Some(context.into());
        self
    }

    /// Sets whether to show a clear button (×) when input is non-empty.
    pub fn show_clear_button(mut self, show: bool) -> Self {
        self.show_clear_button = show;
        self
    }

    /// Overrides default theme colors.
    pub fn colors(mut self, colors: ThemeColors) -> Self {
        self.custom_colors = Some(colors);
        self
    }

    /// Overrides default theme dimensions.
    pub fn dimensions(mut self, dimensions: ThemeDimensions) -> Self {
        self.custom_dimensions = Some(dimensions);
        self
    }

    /// Sets the text change callback.
    pub fn on_change(
        mut self,
        handler: impl Fn(String, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Arc::new(handler));
        self
    }

    /// Sets the enter / submit callback.
    pub fn on_submit(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_submit = Some(Arc::new(handler));
        self
    }

    /// Sets the escape / dismiss callback.
    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_dismiss = Some(Arc::new(handler));
        self
    }
}

impl RenderOnce for SearchInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (c, d) = match (self.custom_colors, self.custom_dimensions) {
            (Some(c), Some(d)) => (c, d),
            (Some(c), None) => {
                let d = if cx.has_global::<ThemeManager>() {
                    cx.global::<ThemeManager>().current().dimensions.clone()
                } else {
                    Theme::default_theme().dimensions
                };
                (c, d)
            }
            (None, Some(d)) => {
                let c = if cx.has_global::<ThemeManager>() {
                    cx.global::<ThemeManager>().current().colors.clone()
                } else {
                    Theme::default_theme().colors
                };
                (c, d)
            }
            (None, None) => {
                if cx.has_global::<ThemeManager>() {
                    let t = cx.global::<ThemeManager>().current();
                    (t.colors.clone(), t.dimensions.clone())
                } else {
                    let def = Theme::default_theme();
                    (def.colors, def.dimensions)
                }
            }
        };

        let state_rc = get_or_create_input_state(&self.id, self.value.as_ref());
        {
            let mut state = state_rc.borrow_mut();
            state.sync_value(self.value.to_string());
        }

        let mut is_focused = self.focus_handle.is_focused(window);
        if self.autofocus && !is_focused {
            self.focus_handle.focus(window, cx);
            is_focused = self.focus_handle.is_focused(window);
        }

        let value_str = self.value.as_ref();
        let is_empty = value_str.is_empty();
        let is_active = is_focused || !is_empty;

        let on_change_cb = self.on_change.clone();
        let on_submit_cb = self.on_submit.clone();
        let on_dismiss_cb = self.on_dismiss.clone();
        let focus_handle_kd = self.focus_handle.clone();
        let state_rc_kd = state_rc.clone();

        let key_down_handler = move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if !focus_handle_kd.is_focused(window) {
                return;
            }
            let keystroke = &event.keystroke;
            let ctrl = keystroke.modifiers.control || keystroke.modifiers.platform;
            let alt = keystroke.modifiers.alt;
            let shift = keystroke.modifiers.shift;
            let lower_key = keystroke.key.to_lowercase();

            match lower_key.as_str() {
                "escape" => {
                    cx.stop_propagation();
                    if let Some(ref on_dismiss) = on_dismiss_cb {
                        on_dismiss(window, cx);
                    } else {
                        let mut state = state_rc_kd.borrow_mut();
                        if !state.value.is_empty() {
                            state.value.clear();
                            state.cursor_offset = 0;
                            state.selection = 0..0;
                            state.reversed = false;
                            state.marked_range = None;
                            drop(state);
                            if let Some(ref on_change) = on_change_cb {
                                on_change(String::new(), window, cx);
                            }
                            window.refresh();
                        }
                    }
                    return;
                }
                "enter" | "return" => {
                    cx.stop_propagation();
                    let val = state_rc_kd.borrow().value.clone();
                    if let Some(ref on_submit) = on_submit_cb {
                        on_submit(&val, window, cx);
                    }
                    return;
                }
                "backspace" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    let new_val = if ctrl {
                        state.delete_word_backward()
                    } else {
                        state.delete_backward()
                    };
                    drop(state);
                    if let Some(ref on_change) = on_change_cb {
                        on_change(new_val, window, cx);
                    }
                    window.refresh();
                    return;
                }
                "delete" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    let new_val = if ctrl {
                        state.delete_word_forward()
                    } else {
                        state.delete_forward()
                    };
                    drop(state);
                    if let Some(ref on_change) = on_change_cb {
                        on_change(new_val, window, cx);
                    }
                    window.refresh();
                    return;
                }
                "left" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    if ctrl {
                        state.move_word_left(shift);
                    } else {
                        state.move_left(shift);
                    }
                    window.refresh();
                    return;
                }
                "right" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    if ctrl {
                        state.move_word_right(shift);
                    } else {
                        state.move_right(shift);
                    }
                    window.refresh();
                    return;
                }
                "home" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    state.move_home(shift);
                    window.refresh();
                    return;
                }
                "end" => {
                    cx.stop_propagation();
                    let mut state = state_rc_kd.borrow_mut();
                    state.move_end(shift);
                    window.refresh();
                    return;
                }
                _ => {}
            }

            if ctrl && !alt {
                match lower_key.as_str() {
                    "a" => {
                        cx.stop_propagation();
                        let mut state = state_rc_kd.borrow_mut();
                        state.select_all();
                        window.refresh();
                        return;
                    }
                    "c" => {
                        cx.stop_propagation();
                        let state = state_rc_kd.borrow();
                        let text = if !state.selection_range().is_empty() {
                            state.selected_text().to_string()
                        } else {
                            state.value.clone()
                        };
                        if !text.is_empty() {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                        }
                        return;
                    }
                    "x" => {
                        cx.stop_propagation();
                        let mut state = state_rc_kd.borrow_mut();
                        let text = if !state.selection_range().is_empty() {
                            state.selected_text().to_string()
                        } else {
                            state.value.clone()
                        };
                        if !text.is_empty() {
                            cx.write_to_clipboard(ClipboardItem::new_string(text));
                            let sel = state.selection_range();
                            let new_val = if !sel.is_empty() {
                                state.replace_range(sel, "")
                            } else {
                                state.value.clear();
                                state.cursor_offset = 0;
                                state.selection = 0..0;
                                String::new()
                            };
                            drop(state);
                            if let Some(ref on_change) = on_change_cb {
                                on_change(new_val, window, cx);
                            }
                            window.refresh();
                        }
                        return;
                    }
                    "v" => {
                        cx.stop_propagation();
                        if let Some(clipboard) = cx.read_from_clipboard()
                            && let Some(text) = clipboard.text()
                        {
                            let sanitized = text.replace(['\r', '\n'], "");
                            let mut state = state_rc_kd.borrow_mut();
                            let new_val = state.insert_text(&sanitized);
                            drop(state);
                            if let Some(ref on_change) = on_change_cb {
                                on_change(new_val, window, cx);
                            }
                            window.refresh();
                        }
                        return;
                    }
                    _ => {}
                }
            }
        };

        // Clear button (x)
        let clear_button = if self.show_clear_button && !is_empty {
            let on_change_clear = self.on_change.clone();
            let focus_handle_clear = self.focus_handle.clone();
            let state_rc_clear = state_rc.clone();
            Some(
                div()
                    .id((self.id.clone(), "clear"))
                    .cursor_pointer()
                    .flex_shrink_0()
                    .w(px(16.0))
                    .h(px(16.0))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|this| this.bg(c.panel_row_hover))
                    .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                        cx.stop_propagation();
                        focus_handle_clear.focus(window, cx);
                        {
                            let mut state = state_rc_clear.borrow_mut();
                            state.value.clear();
                            state.cursor_offset = 0;
                            state.selection = 0..0;
                            state.reversed = false;
                            state.marked_range = None;
                        }
                        if let Some(ref on_change) = on_change_clear {
                            on_change(String::new(), window, cx);
                        }
                        window.refresh();
                    })
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.dialog_muted)
                            .line_height(px(11.0))
                            .child("×"),
                    ),
            )
        } else {
            None
        };

        // Bottom indicator line (retained exactly as requested)
        let bottom_indicator = div()
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(px(2.0))
            .rounded_b(px(d.select_trigger_radius))
            .bg(if is_active {
                c.focus_accent
            } else {
                c.dialog_border
            });

        let on_dismiss_action = self.on_dismiss.clone();
        let on_change_action = self.on_change.clone();
        let on_paste_change = self.on_change.clone();
        let on_cut_change = self.on_change.clone();

        let state_action_dismiss = state_rc.clone();
        let state_action_copy = state_rc.clone();
        let state_action_cut = state_rc.clone();
        let state_action_paste = state_rc.clone();
        let state_action_select_all = state_rc.clone();

        let focus_handle_click = self.focus_handle.clone();
        let state_rc_mouse_down = state_rc.clone();
        let state_rc_mouse_move = state_rc.clone();
        let state_rc_mouse_up = state_rc.clone();

        let key_context = self.key_context.as_deref().unwrap_or("SearchInput");

        div()
            .id(self.id.clone())
            .key_context(key_context)
            .track_focus(&self.focus_handle)
            .relative()
            .overflow_hidden()
            .cursor_text()
            .flex()
            .items_center()
            .gap(px(6.0))
            .w_full()
            .h(px(28.0))
            .px(px(8.0))
            .rounded(px(d.select_trigger_radius))
            .bg(c.dialog_secondary_button_bg)
            .border_1()
            .border_color(c.dialog_border)
            .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                cx.stop_propagation();
                focus_handle_click.focus(window, cx);
                let mut state = state_rc_mouse_down.borrow_mut();
                if event.click_count >= 3 {
                    state.select_all();
                } else if event.click_count == 2 {
                    if state.value.is_empty() {
                        state.set_cursor(0, 0, false);
                    } else if let Some(bounds) = state.last_bounds
                        && let Some(ref line) = state.last_line
                    {
                        let rel_x = (event.position.x - bounds.left()).max(px(0.0));
                        let idx = line.closest_index_for_x(rel_x);
                        let prev = find_prev_word_boundary(&state.value, idx);
                        let next = find_next_word_boundary(&state.value, idx);
                        state.selection = prev..next;
                        state.cursor_offset = next;
                        state.reversed = false;
                    }
                } else {
                    if state.value.is_empty() {
                        state.set_cursor(0, 0, false);
                    } else if let Some(bounds) = state.last_bounds
                        && let Some(ref line) = state.last_line
                    {
                        let rel_x = (event.position.x - bounds.left()).max(px(0.0));
                        let idx = line.closest_index_for_x(rel_x);
                        let shift = event.modifiers.shift;
                        let anchor = state.selection_anchor();
                        state.set_cursor(idx, anchor, shift);
                        state.is_dragging = true;
                        state.drag_anchor = if shift { anchor } else { idx };
                    }
                }
                window.refresh();
            })
            .on_mouse_move(move |event, window, _cx| {
                let mut state = state_rc_mouse_move.borrow_mut();
                if state.is_dragging {
                    if let Some(bounds) = state.last_bounds
                        && let Some(ref line) = state.last_line
                    {
                        let rel_x = (event.position.x - bounds.left()).max(px(0.0));
                        let idx = line.closest_index_for_x(rel_x);
                        let anchor = state.drag_anchor;
                        state.set_cursor(idx, anchor, true);
                        window.refresh();
                    }
                }
            })
            .on_mouse_up(MouseButton::Left, move |_event, _window, _cx| {
                let mut state = state_rc_mouse_up.borrow_mut();
                state.is_dragging = false;
            })
            .on_action({
                let on_dismiss = on_dismiss_action;
                let on_change = on_change_action;
                let state_rc = state_action_dismiss;
                move |_: &platform_contracts::actions::DismissTransientUi, window, cx| {
                    cx.stop_propagation();
                    if let Some(ref on_dismiss) = on_dismiss {
                        on_dismiss(window, cx);
                    } else {
                        let mut state = state_rc.borrow_mut();
                        if !state.value.is_empty() {
                            state.value.clear();
                            state.cursor_offset = 0;
                            state.selection = 0..0;
                            drop(state);
                            if let Some(ref on_change) = on_change {
                                on_change(String::new(), window, cx);
                            }
                            window.refresh();
                        }
                    }
                }
            })
            .on_action({
                let state_rc = state_action_copy;
                move |_: &platform_contracts::actions::Copy, _window, cx| {
                    cx.stop_propagation();
                    let state = state_rc.borrow();
                    let text = if !state.selection_range().is_empty() {
                        state.selected_text().to_string()
                    } else {
                        state.value.clone()
                    };
                    if !text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
            })
            .on_action({
                let state_rc = state_action_cut;
                let on_change = on_cut_change;
                move |_: &platform_contracts::actions::Cut, window, cx| {
                    cx.stop_propagation();
                    let mut state = state_rc.borrow_mut();
                    let text = if !state.selection_range().is_empty() {
                        state.selected_text().to_string()
                    } else {
                        state.value.clone()
                    };
                    if !text.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                        let sel = state.selection_range();
                        let new_val = if !sel.is_empty() {
                            state.replace_range(sel, "")
                        } else {
                            state.value.clear();
                            state.cursor_offset = 0;
                            state.selection = 0..0;
                            String::new()
                        };
                        drop(state);
                        if let Some(ref on_change) = on_change {
                            on_change(new_val, window, cx);
                        }
                        window.refresh();
                    }
                }
            })
            .on_action({
                let state_rc = state_action_paste;
                let on_change = on_paste_change;
                move |_: &platform_contracts::actions::Paste, window, cx| {
                    cx.stop_propagation();
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(text) = clipboard.text()
                    {
                        let sanitized = text.replace(['\r', '\n'], "");
                        let mut state = state_rc.borrow_mut();
                        let new_val = state.insert_text(&sanitized);
                        drop(state);
                        if let Some(ref on_change) = on_change {
                            on_change(new_val, window, cx);
                        }
                        window.refresh();
                    }
                }
            })
            .on_action({
                let state_rc = state_action_select_all;
                move |_: &platform_contracts::actions::SelectAll, window, cx| {
                    cx.stop_propagation();
                    let mut state = state_rc.borrow_mut();
                    state.select_all();
                    window.refresh();
                }
            })
            .on_key_down(key_down_handler)
            .child(SearchInputContentElement {
                id: self.id.clone(),
                state: state_rc.clone(),
                placeholder: self.placeholder.clone(),
                focus_handle: self.focus_handle.clone(),
                colors: c.clone(),
                dimensions: d.clone(),
                on_change: self.on_change.clone(),
            })
            .children(clear_button)
            .child(bottom_indicator)
    }
}

// ── SearchInputContentElement ───────────────────────────────────────────────

pub struct SearchInputContentPrepaintState {
    line: Option<ShapedLine>,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    hitbox: Option<Hitbox>,
}

pub struct SearchInputContentElement {
    pub id: ElementId,
    pub state: Rc<RefCell<SearchInputState>>,
    pub placeholder: SharedString,
    pub focus_handle: FocusHandle,
    pub colors: ThemeColors,
    pub dimensions: ThemeDimensions,
    pub on_change: Option<InputChangeHandler>,
}

impl IntoElement for SearchInputContentElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SearchInputContentElement {
    type RequestLayoutState = ();
    type PrepaintState = SearchInputContentPrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some((self.id.clone(), "content").into())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(20.0).max(window.line_height()).into();
        style.flex_grow = 1.0;
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let is_focused = self.focus_handle.is_focused(window);
        let mut state = self.state.borrow_mut();
        state.last_bounds = Some(bounds);

        let is_empty = state.value.is_empty();
        let (display_text, text_color, is_placeholder): (SharedString, Hsla, bool) = if is_empty {
            (self.placeholder.clone(), self.colors.dialog_muted, true)
        } else {
            (state.value.clone().into(), self.colors.text_default, false)
        };

        let base_run = TextRun {
            len: display_text.len(),
            font: window.text_style().font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if !is_placeholder
            && let Some(marked_range) = state.marked_range.as_ref().filter(|_| !display_text.is_empty())
        {
            vec![
                TextRun {
                    len: marked_range.start.min(display_text.len()),
                    ..base_run.clone()
                },
                TextRun {
                    len: (marked_range.end - marked_range.start).min(display_text.len().saturating_sub(marked_range.start)),
                    underline: Some(UnderlineStyle {
                        color: Some(self.colors.text_default),
                        thickness: px(self.dimensions.underline_thickness),
                        wavy: false,
                    }),
                    ..base_run.clone()
                },
                TextRun {
                    len: display_text.len().saturating_sub(marked_range.end),
                    ..base_run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![base_run]
        };

        let font_size = px(12.0);
        let line = window.text_system().shape_line(display_text, font_size, &runs, None);
        state.last_line = Some(line.clone());

        let line_height = bounds.size.height;
        let cursor_height = px(14.0).min(line_height);
        let cursor_y = bounds.top() + (line_height - cursor_height) / 2.0;

        let selection = if is_focused && !is_placeholder && !state.selection_range().is_empty() {
            let sel = state.selection_range();
            let start_x = line.x_for_index(sel.start);
            let end_x = line.x_for_index(sel.end);
            let left = bounds.left() + start_x.min(end_x);
            let right = bounds.left() + start_x.max(end_x);
            Some(fill(
                Bounds::from_corners(
                    point(left, bounds.top() + px(1.0)),
                    point(right, bounds.bottom() - px(1.0)),
                ),
                self.colors.selection,
            ))
        } else {
            None
        };

        let cursor = if is_focused && state.selection_range().is_empty() {
            let cursor_x = if is_placeholder {
                // Zed behavior: cursor is at 0 (start of field), placeholder is visible behind it
                px(0.0)
            } else {
                line.x_for_index(state.cursor_offset.min(state.value.len()))
            };
            let mut cursor_color = self.colors.focus_accent;
            cursor_color.a = 1.0;
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_x, cursor_y),
                    size(px(self.dimensions.cursor_width.max(1.5)), cursor_height),
                ),
                cursor_color,
            ))
        } else {
            None
        };

        let hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        SearchInputContentPrepaintState {
            line: Some(line),
            selection,
            cursor,
            hitbox,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(hitbox) = prepaint.hitbox.as_ref()
            && hitbox.is_hovered(window)
        {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

        if self.focus_handle.is_focused(window) {
            window.handle_input(
                &self.focus_handle,
                SearchInputHandler {
                    state: self.state.clone(),
                    on_change: self.on_change.clone(),
                },
                cx,
            );
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }

        if let Some(line) = prepaint.line.take() {
            line.paint(
                bounds.origin,
                bounds.size.height,
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
        }

        if let Some(cursor) = prepaint.cursor.take() {
            window.paint_quad(cursor);
        }
    }
}

// ── SearchInputHandler (GPUI IME & Character Input Bridge) ──────────────────

/// Bridge between GPUI IME / character events and [`SearchInput`].
pub struct SearchInputHandler {
    pub state: Rc<RefCell<SearchInputState>>,
    pub on_change: Option<InputChangeHandler>,
}

impl SearchInputHandler {
    /// Pure helper that calculates text and adjusted range for a UTF-16 range.
    pub fn compute_text_for_range(value: &str, range_utf16: Range<usize>) -> (Option<String>, Option<Range<usize>>) {
        let utf16_chars: Vec<u16> = value.encode_utf16().collect();
        let start = range_utf16.start.min(utf16_chars.len());
        let end = range_utf16.end.min(utf16_chars.len());
        (String::from_utf16(&utf16_chars[start..end]).ok(), Some(start..end))
    }

    /// Pure helper that computes the replacement text in range.
    pub fn compute_replace_text_in_range(current: &str, replacement_range: Option<Range<usize>>, text: &str) -> String {
        if let Some(range) = replacement_range {
            let utf16_chars: Vec<u16> = current.encode_utf16().collect();
            let start = range.start.min(utf16_chars.len());
            let end = range.end.min(utf16_chars.len());
            let prefix = String::from_utf16_lossy(&utf16_chars[..start]);
            let suffix = String::from_utf16_lossy(&utf16_chars[end..]);
            format!("{prefix}{text}{suffix}")
        } else {
            format!("{current}{text}")
        }
    }
}

impl InputHandler for SearchInputHandler {
    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<UTF16Selection> {
        let state = self.state.borrow();
        let range = state.selection_range();
        let utf16_range = utf8_range_to_utf16_in(&state.value, &range);
        Some(UTF16Selection {
            range: utf16_range,
            reversed: state.reversed,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, _cx: &mut App) -> Option<Range<usize>> {
        let state = self.state.borrow();
        state
            .marked_range
            .as_ref()
            .map(|range| utf8_range_to_utf16_in(&state.value, range))
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<String> {
        let state = self.state.borrow();
        let range = utf16_range_to_utf8_in(&state.value, &range_utf16);
        *adjusted_range = Some(utf8_range_to_utf16_in(&state.value, &range));
        Some(state.value[range].to_string())
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Filter out control characters that should not be typed into single-line inputs
        let filtered_text: String = text.chars().filter(|ch| !ch.is_control() || *ch == '\t').collect();
        if filtered_text.is_empty() && !text.is_empty() {
            return;
        }

        let mut state = self.state.borrow_mut();
        let byte_range = replacement_range
            .map(|r| utf16_range_to_utf8_in(&state.value, &r))
            .unwrap_or_else(|| state.selection_range());

        let new_val = state.replace_range(byte_range, &filtered_text);
        drop(state);

        if let Some(ref on_change) = self.on_change {
            on_change(new_val, window, cx);
        }
        window.refresh();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let mut state = self.state.borrow_mut();
        let byte_range = range_utf16
            .map(|r| utf16_range_to_utf8_in(&state.value, &r))
            .unwrap_or_else(|| state.selection_range());

        let mark_start = byte_range.start;
        state.replace_range(byte_range, new_text);
        let mark_end = mark_start + new_text.len();
        state.marked_range = if !new_text.is_empty() {
            Some(mark_start..mark_end)
        } else {
            None
        };

        if let Some(new_sel) = new_selected_range {
            let sel_utf8 = utf16_range_to_utf8_in(&state.value, &new_sel);
            state.selection = sel_utf8.start..sel_utf8.end;
            state.cursor_offset = sel_utf8.end;
            state.reversed = false;
        }
        let new_val = state.value.clone();
        drop(state);

        if let Some(ref on_change) = self.on_change {
            on_change(new_val, window, cx);
        }
        window.refresh();
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut App) {
        let mut state = self.state.borrow_mut();
        state.marked_range = None;
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        let state = self.state.borrow();
        let bounds = state.last_bounds?;
        if let Some(ref line) = state.last_line {
            let range = utf16_range_to_utf8_in(&state.value, &range_utf16);
            let start_x = line.x_for_index(range.start);
            let end_x = line.x_for_index(range.end);
            let left = bounds.left() + start_x.min(end_x);
            let width = (end_x - start_x).abs().max(px(2.0));
            Some(Bounds::new(
                point(left, bounds.top()),
                size(width, bounds.size.height),
            ))
        } else {
            Some(bounds)
        }
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<usize> {
        let state = self.state.borrow();
        let bounds = state.last_bounds?;
        let line = state.last_line.as_ref()?;
        let rel_x = (point.x - bounds.left()).max(px(0.0));
        let utf8_idx = line.closest_index_for_x(rel_x);
        Some(utf8_to_utf16_in_single(&state.value, utf8_idx))
    }
}

/// Convenience function to construct a [`SearchInput`].
pub fn search_input(
    id: impl Into<ElementId>,
    value: impl Into<SharedString>,
    focus_handle: FocusHandle,
) -> SearchInput {
    SearchInput::new(id, value, focus_handle)
}

/// Result of handling a keystroke on a search input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchKeyAction {
    /// No change / ignored keystroke.
    Ignored,
    /// Value updated to a new string.
    Change(String),
    /// Submit triggered (e.g. Enter).
    Submit,
    /// Dismiss triggered (e.g. Escape).
    Dismiss,
}

/// Pure helper that computes the next search input action for a keystroke.
pub fn handle_search_keystroke(
    current: &str,
    key: &str,
    key_char: Option<&str>,
    ctrl: bool,
    alt: bool,
) -> SearchKeyAction {
    let lower_key = key.to_lowercase();
    if ctrl && !alt {
        match lower_key.as_str() {
            "a" => return SearchKeyAction::Ignored,
            "backspace" => return SearchKeyAction::Change(String::new()),
            _ => {}
        }
    }

    match lower_key.as_str() {
        "escape" => SearchKeyAction::Dismiss,
        "enter" | "return" => SearchKeyAction::Submit,
        "backspace" | "delete" => {
            let mut new_str = current.to_string();
            new_str.pop();
            SearchKeyAction::Change(new_str)
        }
        "space" => {
            let mut new_str = current.to_string();
            new_str.push(' ');
            SearchKeyAction::Change(new_str)
        }
        _ => {
            if !ctrl && !alt {
                let text = key_char.unwrap_or_else(|| {
                    if key.chars().count() == 1 {
                        key
                    } else {
                        ""
                    }
                });
                if !text.is_empty() && !text.chars().any(|ch| ch.is_control()) {
                    let mut new_str = current.to_string();
                    new_str.push_str(text);
                    return SearchKeyAction::Change(new_str);
                }
            }
            SearchKeyAction::Ignored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::prelude::v1::test;

    #[test]
    fn test_search_keystroke_typing() {
        // Typing ASCII character
        let action = handle_search_keystroke("rust", "c", None, false, false);
        assert_eq!(action, SearchKeyAction::Change("rustc".to_string()));

        // Typing character with key_char
        let action = handle_search_keystroke("rust", "C", Some("C"), false, false);
        assert_eq!(action, SearchKeyAction::Change("rustC".to_string()));

        // Typing space
        let action = handle_search_keystroke("hello", "space", None, false, false);
        assert_eq!(action, SearchKeyAction::Change("hello ".to_string()));
    }

    #[test]
    fn test_search_keystroke_backspace_and_delete() {
        let action = handle_search_keystroke("abc", "backspace", None, false, false);
        assert_eq!(action, SearchKeyAction::Change("ab".to_string()));

        let action = handle_search_keystroke("abc", "delete", None, false, false);
        assert_eq!(action, SearchKeyAction::Change("ab".to_string()));

        // Backspacing empty string doesn't panic
        let action = handle_search_keystroke("", "backspace", None, false, false);
        assert_eq!(action, SearchKeyAction::Change("".to_string()));
    }

    #[test]
    fn test_search_keystroke_ctrl_shortcuts() {
        // Ctrl+Backspace clears the entire query
        let action = handle_search_keystroke("full query", "backspace", None, true, false);
        assert_eq!(action, SearchKeyAction::Change("".to_string()));

        // Ctrl+A is handled without inserting 'a'
        let action = handle_search_keystroke("test", "a", None, true, false);
        assert_eq!(action, SearchKeyAction::Ignored);
    }

    #[test]
    fn test_search_keystroke_submit_and_dismiss() {
        let action = handle_search_keystroke("test", "enter", None, false, false);
        assert_eq!(action, SearchKeyAction::Submit);

        let action = handle_search_keystroke("test", "escape", None, false, false);
        assert_eq!(action, SearchKeyAction::Dismiss);
    }

    #[test]
    fn test_search_keystroke_ignores_special_keys() {
        let action = handle_search_keystroke("test", "F1", None, false, false);
        assert_eq!(action, SearchKeyAction::Ignored);

        let action = handle_search_keystroke("test", "Shift", None, false, false);
        assert_eq!(action, SearchKeyAction::Ignored);
    }

    #[test]
    fn test_search_input_handler_text_and_selection() {
        let (text, actual) = SearchInputHandler::compute_text_for_range("Hello World", 0..5);
        assert_eq!(text, Some("Hello".to_string()));
        assert_eq!(actual, Some(0..5));

        let (text_utf16, actual_utf16) = SearchInputHandler::compute_text_for_range("你好世界", 0..2);
        assert_eq!(text_utf16, Some("你好".to_string()));
        assert_eq!(actual_utf16, Some(0..2));
    }

    #[test]
    fn test_search_input_handler_replace_and_multibyte() {
        // Append text
        let res = SearchInputHandler::compute_replace_text_in_range("Rust", None, " Lang");
        assert_eq!(res, "Rust Lang");

        // Chinese / IME insertion
        let res = SearchInputHandler::compute_replace_text_in_range("你好", Some(2..2), "世界");
        assert_eq!(res, "你好世界");

        // Replacing middle part
        let res = SearchInputHandler::compute_replace_text_in_range("abcdef", Some(2..4), "123");
        assert_eq!(res, "ab123ef");

        // Deleting range
        let res = SearchInputHandler::compute_replace_text_in_range("Hello World", Some(5..11), "");
        assert_eq!(res, "Hello");
    }

    #[test]
    fn test_search_input_state_editing_and_cursor() {
        let mut state = SearchInputState::new("".to_string());
        assert_eq!(state.cursor(), 0);

        state.insert_text("hello");
        assert_eq!(state.value, "hello");
        assert_eq!(state.cursor(), 5);

        state.move_left(false);
        assert_eq!(state.cursor(), 4);

        state.insert_text("X");
        assert_eq!(state.value, "hellXo");
        assert_eq!(state.cursor(), 5);

        state.delete_backward();
        assert_eq!(state.value, "hello");
        assert_eq!(state.cursor(), 4);

        state.delete_forward();
        assert_eq!(state.value, "hell");
        assert_eq!(state.cursor(), 4);
    }

    #[test]
    fn test_search_input_state_selection_and_replacement() {
        let mut state = SearchInputState::new("hello world".to_string());
        state.selection = 6..11;
        assert_eq!(state.selected_text(), "world");

        state.insert_text("there");
        assert_eq!(state.value, "hello there");
        assert_eq!(state.cursor(), 11);

        state.select_all();
        assert_eq!(state.selected_text(), "hello there");
        state.insert_text("reset");
        assert_eq!(state.value, "reset");
    }

    #[test]
    fn test_search_input_state_word_navigation() {
        let mut state = SearchInputState::new("foo bar baz".to_string());
        state.move_home(false);
        assert_eq!(state.cursor(), 0);

        state.move_word_right(false);
        assert_eq!(state.cursor(), 4); // start of "bar"

        state.move_word_right(false);
        assert_eq!(state.cursor(), 8); // start of "baz"

        state.move_word_left(false);
        assert_eq!(state.cursor(), 4);

        state.move_end(false);
        assert_eq!(state.cursor(), 11);

        state.delete_word_backward();
        assert_eq!(state.value, "foo bar ");
    }

    #[test]
    fn test_utf8_utf16_multibyte_conversions() {
        let text = "你好世界";
        // Each Chinese character is 3 UTF-8 bytes and 1 UTF-16 code unit
        assert_eq!(text.len(), 12);
        assert_eq!(text.encode_utf16().count(), 4);

        let utf16_range = 1..3; // "好世"
        let utf8_range = utf16_range_to_utf8_in(text, &utf16_range);
        assert_eq!(utf8_range, 3..9);
        assert_eq!(&text[utf8_range.clone()], "好世");

        let converted_back = utf8_range_to_utf16_in(text, &utf8_range);
        assert_eq!(converted_back, 1..3);
    }
}
