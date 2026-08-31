//! Editing settings components: Typography & Fonts, Editor Behavior.

use gpui::*;

use crate::components::font_picker::{SearchableFontPickerProps, render_searchable_font_picker};
use crate::components::number_field::{NumberFieldProps, render_number_field};
use crate::ui_helpers::{
    SettingsClickHandler, SettingsKeyHandler, SettingsOptionHandler, SettingsSearchHandler,
    make_row, make_row_with_reset, make_section,
};
use theme::{ThemeColors, ThemeDimensions};
use ui::switch::Switch;

pub(crate) struct TypographyProps {
    pub ui_font_name: String,
    pub is_ui_font_open: bool,
    pub search_query_ui_font: String,
    pub on_toggle_ui_font: SettingsClickHandler,
    pub on_search_ui_font: SettingsSearchHandler,
    pub on_select_ui_font: SettingsOptionHandler<String>,
    pub on_reset_ui_font: Option<SettingsClickHandler>,

    pub prose_font_name: String,
    pub is_prose_font_open: bool,
    pub search_query_prose_font: String,
    pub on_toggle_prose_font: SettingsClickHandler,
    pub on_search_prose_font: SettingsSearchHandler,
    pub on_select_prose_font: SettingsOptionHandler<String>,
    pub on_reset_prose_font: Option<SettingsClickHandler>,

    pub code_font_name: String,
    pub is_code_font_open: bool,
    pub search_query_code_font: String,
    pub on_toggle_code_font: SettingsClickHandler,
    pub on_search_code_font: SettingsSearchHandler,
    pub on_select_code_font: SettingsOptionHandler<String>,
    pub on_reset_code_font: Option<SettingsClickHandler>,

    pub available_fonts: Vec<SharedString>,

    pub font_size: u32,
    pub is_editing_font_size: bool,
    pub edit_buffer_font_size: Option<String>,
    pub font_size_focus_handle: FocusHandle,
    pub on_font_dec: SettingsClickHandler,
    pub on_font_inc: SettingsClickHandler,
    pub on_start_edit_font_size: SettingsClickHandler,
    pub on_key_down_font_size: SettingsKeyHandler,
    pub on_reset_font_size: Option<SettingsClickHandler>,

    pub line_height: f32,
    pub is_editing_line_height: bool,
    pub edit_buffer_line_height: Option<String>,
    pub line_height_focus_handle: FocusHandle,
    pub on_lh_dec: SettingsClickHandler,
    pub on_lh_inc: SettingsClickHandler,
    pub on_start_edit_line_height: SettingsClickHandler,
    pub on_key_down_line_height: SettingsKeyHandler,
    pub on_reset_line_height: Option<SettingsClickHandler>,
}

pub(crate) fn render_typography_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: TypographyProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // 1. Interface Font Row
        let ui_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-ui-font",
                current_font_name: props.ui_font_name,
                default_label: "Lexend (default)".to_string(),
                is_open: props.is_ui_font_open,
                search_query: props.search_query_ui_font,
                on_toggle: props.on_toggle_ui_font,
                on_search_change: props.on_search_ui_font,
                available_fonts: props.available_fonts.clone(),
                on_select_font: props.on_select_ui_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Interface Font",
            "Font used for menus, explorer sidebar, and application chrome",
            props.on_reset_ui_font,
            ui_picker,
        ));

        // 2. Prose Text Font Row
        let prose_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-prose-font",
                current_font_name: props.prose_font_name,
                default_label: "Lexend (default)".to_string(),
                is_open: props.is_prose_font_open,
                search_query: props.search_query_prose_font,
                on_toggle: props.on_toggle_prose_font,
                on_search_change: props.on_search_prose_font,
                available_fonts: props.available_fonts.clone(),
                on_select_font: props.on_select_prose_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Prose Text Font",
            "Font used for Markdown prose, headings, and tables",
            props.on_reset_prose_font,
            prose_picker,
        ));

        // 3. Code Block Font Row
        let default_code_label = if cfg!(target_os = "windows") {
            "Consolas (default)".to_string()
        } else if cfg!(target_os = "macos") {
            "Menlo (default)".to_string()
        } else {
            "monospace (default)".to_string()
        };
        let code_picker = render_searchable_font_picker(
            c,
            d,
            SearchableFontPickerProps {
                id_prefix: "pref-code-font",
                current_font_name: props.code_font_name,
                default_label: default_code_label,
                is_open: props.is_code_font_open,
                search_query: props.search_query_code_font,
                on_toggle: props.on_toggle_code_font,
                on_search_change: props.on_search_code_font,
                available_fonts: props.available_fonts,
                on_select_font: props.on_select_code_font,
            },
        );
        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Code Block Font",
            "Monospace font used for code blocks and inline code",
            props.on_reset_code_font,
            code_picker,
        ));

        // 4. Document Font Size Row
        let ctrl_font = render_number_field(
            c,
            d,
            NumberFieldProps {
                id_prefix: "pref-font-size",
                value_text: format!("{}", props.font_size),
                is_editing: props.is_editing_font_size,
                edit_buffer: props.edit_buffer_font_size,
                focus_handle: props.font_size_focus_handle,
                on_dec: props.on_font_dec,
                on_inc: props.on_font_inc,
                on_start_edit: props.on_start_edit_font_size,
                on_key_down: props.on_key_down_font_size,
            },
        );

        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Document Font Size",
            "Baseline font size in pixels for document prose content",
            props.on_reset_font_size,
            ctrl_font,
        ));

        // 5. Line Spacing Multiplier Row
        let ctrl_lh = render_number_field(
            c,
            d,
            NumberFieldProps {
                id_prefix: "pref-line-height",
                value_text: format!("{:.2}", props.line_height),
                is_editing: props.is_editing_line_height,
                edit_buffer: props.edit_buffer_line_height,
                focus_handle: props.line_height_focus_handle,
                on_dec: props.on_lh_dec,
                on_inc: props.on_lh_inc,
                on_start_edit: props.on_start_edit_line_height,
                on_key_down: props.on_key_down_line_height,
            },
        );

        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Line Spacing Multiplier",
            "Proportional line height for comfortable reading",
            props.on_reset_line_height,
            ctrl_lh,
        ));
    }

    make_section(c, d, id, "Typography & Fonts", expanded, toggle_fn, rows)
}

pub(crate) struct EditorBehaviorProps {
    pub line_numbers: bool,
    pub on_toggle_line_numbers: SettingsClickHandler,

    pub word_wrap: bool,
    pub on_toggle_word_wrap: SettingsClickHandler,

    pub tab_size: u32,
    pub is_editing_tab_size: bool,
    pub edit_buffer_tab_size: Option<String>,
    pub tab_size_focus_handle: FocusHandle,
    pub on_tab_size_dec: SettingsClickHandler,
    pub on_tab_size_inc: SettingsClickHandler,
    pub on_start_edit_tab_size: SettingsClickHandler,
    pub on_key_down_tab_size: SettingsKeyHandler,
    pub on_reset_tab_size: Option<SettingsClickHandler>,

    pub insert_spaces: bool,
    pub on_toggle_insert_spaces: SettingsClickHandler,

    pub highlight_active_line: bool,
    pub on_toggle_highlight_active_line: SettingsClickHandler,
}

pub(crate) fn render_editor_behavior_section(
    c: &ThemeColors,
    d: &ThemeDimensions,
    id: impl Into<ElementId>,
    expanded: bool,
    toggle_fn: SettingsClickHandler,
    props: EditorBehaviorProps,
) -> AnyElement {
    let mut inner_border_color = c.dialog_border;
    inner_border_color.a *= 0.4;

    let mut rows = Vec::new();

    if expanded {
        // 1. Line Numbers Toggle
        let ctrl_lines = Switch::new("switch-ed-linenums")
            .checked(props.line_numbers)
            .on_click(props.on_toggle_line_numbers)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Line Numbers",
            "Show line numbers in editor gutter and code blocks",
            ctrl_lines,
        ));

        // 2. Word Wrap Toggle
        let ctrl_wrap = Switch::new("switch-ed-wordwrap")
            .checked(props.word_wrap)
            .on_click(props.on_toggle_word_wrap)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Soft Word Wrapping",
            "Wrap long lines to fit the current window editor viewport width",
            ctrl_wrap,
        ));

        // 3. Tab Size
        let ctrl_tab_size = render_number_field(
            c,
            d,
            NumberFieldProps {
                id_prefix: "pref-tab-size",
                value_text: format!("{}", props.tab_size),
                is_editing: props.is_editing_tab_size,
                edit_buffer: props.edit_buffer_tab_size,
                focus_handle: props.tab_size_focus_handle,
                on_dec: props.on_tab_size_dec,
                on_inc: props.on_tab_size_inc,
                on_start_edit: props.on_start_edit_tab_size,
                on_key_down: props.on_key_down_tab_size,
            },
        );

        rows.push(make_row_with_reset(
            inner_border_color,
            c,
            d,
            "Tab Indentation Width",
            "The number of spaces a tab character equals (2, 4, or 8 spaces)",
            props.on_reset_tab_size,
            ctrl_tab_size,
        ));

        // 4. Insert Spaces for Tabs
        let ctrl_spaces = Switch::new("switch-ed-spaces")
            .checked(props.insert_spaces)
            .on_click(props.on_toggle_insert_spaces)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Insert Spaces on Tab",
            "Pressing Tab key inserts spaces instead of literal tab characters",
            ctrl_spaces,
        ));

        // 5. Highlight Active Line
        let ctrl_active_line = Switch::new("switch-ed-active-line")
            .checked(props.highlight_active_line)
            .on_click(props.on_toggle_highlight_active_line)
            .into_any_element();

        rows.push(make_row(
            inner_border_color,
            c,
            d,
            "Highlight Active Line",
            "Subtly highlight the background of the line containing text cursor",
            ctrl_active_line,
        ));
    }

    make_section(
        c,
        d,
        id,
        "Editor Behaviors & Indentation",
        expanded,
        toggle_fn,
        rows,
    )
}
