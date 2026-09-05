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
            .on_key_down(key_down_handler)
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
    if ctrl && !alt {
        match key {
            "a" | "A" => return SearchKeyAction::Ignored,
            "backspace" => return SearchKeyAction::Change(String::new()),
            _ => {}
        }
    }

    match key {
        "escape" => SearchKeyAction::Dismiss,
        "enter" => SearchKeyAction::Submit,
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
                    if key.len() == 1 {
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
}

