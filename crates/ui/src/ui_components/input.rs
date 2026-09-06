//! Search and text input component with unified styling and keyboard interaction.
//!
//! Provides a standardized search input with:
//! - Uniform styling matching the design system (28px height, rounded corners,
//!   secondary button background, 1px dialog border).
//! - Active bottom accent bar that lights up with `c.focus_accent` when focused
//!   or when a query is present.
//! - Active text cursor indicator when focused.
//! - Immediate focus on mouse-down with event propagation stopped.
//! - Optional autofocus on initial render.
//! - Keyboard navigation: character input (including `key_char`), Space, Backspace,
//!   Delete, Enter (submit), Escape (dismiss / clear).
//! - Optional clear button (×) when query is non-empty.

use std::sync::Arc;

use gpui::*;
use theme::{Theme, ThemeColors, ThemeDimensions, ThemeManager};

/// Callback invoked when input text changes.
pub type InputChangeHandler = Arc<dyn Fn(String, &mut Window, &mut App) + 'static>;

/// Callback invoked on enter / submit.
pub type InputSubmitHandler = Arc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

/// Callback invoked on escape / dismiss.
pub type InputDismissHandler = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

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
        let current_value = self.value.clone();
        let focus_handle_kd = self.focus_handle.clone();

        let key_down_handler = move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if !focus_handle_kd.is_focused(window) {
                return;
            }
            let keystroke = &event.keystroke;
            let ctrl = keystroke.modifiers.control || keystroke.modifiers.platform;
            let alt = keystroke.modifiers.alt;

            if ctrl && !alt {
                match keystroke.key.to_lowercase().as_str() {
                    "v" => {
                        cx.stop_propagation();
                        if let Some(clipboard) = cx.read_from_clipboard()
                            && let Some(text) = clipboard.text()
                        {
                            let sanitized = text.replace(['\r', '\n'], "");
                            let mut new_str = current_value.to_string();
                            new_str.push_str(&sanitized);
                            if let Some(ref on_change) = on_change_cb {
                                on_change(new_str, window, cx);
                            }
                        }
                        return;
                    }
                    "c" => {
                        cx.stop_propagation();
                        if !current_value.is_empty() {
                            cx.write_to_clipboard(ClipboardItem::new_string(current_value.to_string()));
                        }
                        return;
                    }
                    "x" => {
                        cx.stop_propagation();
                        if !current_value.is_empty() {
                            cx.write_to_clipboard(ClipboardItem::new_string(current_value.to_string()));
                            if let Some(ref on_change) = on_change_cb {
                                on_change(String::new(), window, cx);
                            }
                        }
                        return;
                    }
                    _ => {}
                }
            }

            let action = handle_search_keystroke(
                current_value.as_ref(),
                keystroke.key.as_str(),
                keystroke.key_char.as_deref(),
                ctrl,
                alt,
            );

            match action {
                SearchKeyAction::Change(new_val) => {
                    cx.stop_propagation();
                    if let Some(ref on_change) = on_change_cb {
                        on_change(new_val, window, cx);
                    }
                }
                SearchKeyAction::Submit => {
                    cx.stop_propagation();
                    if let Some(ref on_submit) = on_submit_cb {
                        on_submit(current_value.as_ref(), window, cx);
                    }
                }
                SearchKeyAction::Dismiss => {
                    cx.stop_propagation();
                    if let Some(ref on_dismiss) = on_dismiss_cb {
                        on_dismiss(window, cx);
                    } else if !current_value.is_empty() {
                        if let Some(ref on_change) = on_change_cb {
                            on_change(String::new(), window, cx);
                        }
                    }
                }
                SearchKeyAction::Ignored => {}
            }
        };


        let clear_button = if self.show_clear_button && !is_empty {
            let on_change_clear = self.on_change.clone();
            let focus_handle_clear = self.focus_handle.clone();
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
                        if let Some(ref on_change) = on_change_clear {
                            on_change(String::new(), window, cx);
                        }
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

        let cursor_element = if is_focused {
            Some(
                div()
                    .w(px(1.5))
                    .h(px(13.0))
                    .ml(px(1.0))
                    .bg(c.focus_accent),
            )
        } else {
            None
        };

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
        let current_value_action = self.value.clone();
        let on_paste_change = self.on_change.clone();
        let current_value_paste = self.value.clone();
        let on_cut_change = self.on_change.clone();
        let current_value_cut = self.value.clone();
        let current_value_copy = self.value.clone();

        let focus_handle_click = self.focus_handle.clone();
        let key_context = self.key_context.as_deref().unwrap_or("SearchInput");

        div()
            .id(self.id)
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
            .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                cx.stop_propagation();
                focus_handle_click.focus(window, cx);
            })
            .on_action({
                let on_dismiss = on_dismiss_action;
                let on_change = on_change_action;
                let val = current_value_action;
                move |_: &platform_contracts::actions::DismissTransientUi, window, cx| {
                    cx.stop_propagation();
                    if let Some(ref on_dismiss) = on_dismiss {
                        on_dismiss(window, cx);
                    } else if !val.is_empty() {
                        if let Some(ref on_change) = on_change {
                            on_change(String::new(), window, cx);
                        }
                    }
                }
            })
            .on_action({
                let val = current_value_copy;
                move |_: &platform_contracts::actions::Copy, _window, cx| {
                    cx.stop_propagation();
                    if !val.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(val.to_string()));
                    }
                }
            })
            .on_action({
                let val = current_value_cut;
                let on_change = on_cut_change;
                move |_: &platform_contracts::actions::Cut, window, cx| {
                    cx.stop_propagation();
                    if !val.is_empty() {
                        cx.write_to_clipboard(ClipboardItem::new_string(val.to_string()));
                        if let Some(ref on_change) = on_change {
                            on_change(String::new(), window, cx);
                        }
                    }
                }
            })
            .on_action({
                let val = current_value_paste;
                let on_change = on_paste_change;
                move |_: &platform_contracts::actions::Paste, window, cx| {
                    cx.stop_propagation();
                    if let Some(clipboard) = cx.read_from_clipboard()
                        && let Some(text) = clipboard.text()
                    {
                        let sanitized = text.replace(['\r', '\n'], "");
                        let mut new_str = val.to_string();
                        new_str.push_str(&sanitized);
                        if let Some(ref on_change) = on_change {
                            on_change(new_str, window, cx);
                        }
                    }
                }
            })
            .on_action(|_: &platform_contracts::actions::SelectAll, _window, cx| {
                cx.stop_propagation();
            })
            .on_key_down(key_down_handler)
            .child(SearchInputBridgeElement {
                focus_handle: self.focus_handle.clone(),
                value: self.value.clone(),
                on_change: self.on_change.clone(),
            })
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .text_size(px(12.0))
                            .truncate()
                            .text_color(if is_empty {
                                c.dialog_muted
                            } else {
                                c.text_default
                            })
                            .child(if is_empty {
                                self.placeholder.clone()
                            } else {
                                self.value.clone()
                            }),
                    )
                    .children(cursor_element),
            )
            .children(clear_button)
            .child(bottom_indicator)
    }
}

/// Helper element that registers a GPUI [`InputHandler`] for [`SearchInput`] while focused.
pub struct SearchInputBridgeElement {
    pub focus_handle: FocusHandle,
    pub value: SharedString,
    pub on_change: Option<InputChangeHandler>,
}

impl IntoElement for SearchInputBridgeElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SearchInputBridgeElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
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
        (window.request_layout(Style::default(), [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {}

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.focus_handle.is_focused(window) {
            window.handle_input(
                &self.focus_handle,
                SearchInputHandler {
                    value: self.value.clone(),
                    bounds,
                    on_change: self.on_change.clone(),
                },
                cx,
            );
        }
    }
}

/// Bridge between GPUI IME / character events and [`SearchInput`].
pub struct SearchInputHandler {
    pub value: SharedString,
    pub bounds: Bounds<Pixels>,
    pub on_change: Option<InputChangeHandler>,
}

impl SearchInputHandler {
    /// Pure helper that calculates text and adjusted range for a UTF-16 range.
    pub fn compute_text_for_range(value: &str, range_utf16: std::ops::Range<usize>) -> (Option<String>, Option<std::ops::Range<usize>>) {
        let utf16_chars: Vec<u16> = value.encode_utf16().collect();
        let start = range_utf16.start.min(utf16_chars.len());
        let end = range_utf16.end.min(utf16_chars.len());
        (String::from_utf16(&utf16_chars[start..end]).ok(), Some(start..end))
    }

    /// Pure helper that computes the replacement text in range.
    pub fn compute_replace_text_in_range(current: &str, replacement_range: Option<std::ops::Range<usize>>, text: &str) -> String {
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
        let len = self.value.encode_utf16().count();
        Some(UTF16Selection {
            range: len..len,
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, _cx: &mut App) -> Option<std::ops::Range<usize>> {
        None
    }

    fn text_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        adjusted_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<String> {
        let (text, adj) = Self::compute_text_for_range(self.value.as_ref(), range_utf16);
        *adjusted_range = adj;
        text
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<std::ops::Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut App,
    ) {
        let next = Self::compute_replace_text_in_range(self.value.as_ref(), replacement_range, text);
        if let Some(ref on_change) = self.on_change {
            on_change(next, window, cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<std::ops::Range<usize>>,
        new_text: &str,
        _new_selected_range: Option<std::ops::Range<usize>>,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.replace_text_in_range(range_utf16, new_text, window, cx);
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut App) {}

    fn bounds_for_range(
        &mut self,
        _range_utf16: std::ops::Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        Some(self.bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<usize> {
        Some(self.value.encode_utf16().count())
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
}

