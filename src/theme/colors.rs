//! Color palette and semantic color tokens.

use gpui::{Hsla, rgba};
use serde::{Deserialize, Deserializer, Serialize};

/// All configurable colors for the editor UI.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeColors {
    /// Background of the editor scroll area (behind all blocks).
    pub editor_background: Hsla,
    /// Background of the focused raw block in source-editing mode.
    pub source_mode_block_bg: Hsla,
    /// Background used for visible Markdown comment blocks.
    pub comment_bg: Hsla,
    /// Default paragraph / body text colour.
    pub text_default: Hsla,
    /// Inline link text colour in rendered mode.
    pub text_link: Hsla,
    /// Placeholder text shown in empty focused blocks.
    pub text_placeholder: Hsla,
    /// H1 heading text colour.
    pub text_h1: Hsla,
    /// H2 heading text colour.
    pub text_h2: Hsla,
    /// H3 heading text colour.
    pub text_h3: Hsla,
    /// H4 heading text colour.
    pub text_h4: Hsla,
    /// H5 heading text colour.
    pub text_h5: Hsla,
    /// H6 heading text colour.
    pub text_h6: Hsla,
    /// H1 bottom-border colour.
    pub border_h1: Hsla,
    /// H2 bottom-border colour.
    pub border_h2: Hsla,
    /// Quote block text colour.
    pub text_quote: Hsla,
    /// Quote block left-border colour.
    pub border_quote: Hsla,
    /// Note callout background.
    pub callout_note_bg: Hsla,
    /// Note callout accent border/text colour.
    pub callout_note_border: Hsla,
    /// Tip callout background.
    pub callout_tip_bg: Hsla,
    /// Tip callout accent border/text colour.
    pub callout_tip_border: Hsla,
    /// Important callout background.
    pub callout_important_bg: Hsla,
    /// Important callout accent border/text colour.
    pub callout_important_border: Hsla,
    /// Warning callout background.
    pub callout_warning_bg: Hsla,
    /// Warning callout accent border/text colour.
    pub callout_warning_border: Hsla,
    /// Caution callout background.
    pub callout_caution_bg: Hsla,
    /// Caution callout accent border/text colour.
    pub callout_caution_border: Hsla,
    /// Background of footnote definition grouping shells.
    pub footnote_bg: Hsla,
    /// Border colour of footnote definition grouping shells.
    pub footnote_border: Hsla,
    /// Background of the footnote ordinal badge.
    pub footnote_badge_bg: Hsla,
    /// Text colour of the footnote ordinal badge.
    pub footnote_badge_text: Hsla,
    /// Back-reference colour inside footnote headers.
    pub footnote_backref: Hsla,
    /// Border colour of interactive task-list checkboxes.
    pub task_checkbox_border: Hsla,
    /// Background of unchecked task-list checkboxes.
    pub task_checkbox_bg: Hsla,
    /// Background of checked task-list checkboxes.
    pub task_checkbox_checked_bg: Hsla,
    /// Checkmark colour inside checked task-list checkboxes.
    pub task_checkbox_check: Hsla,
    /// Colour of the separator block line.
    pub separator_color: Hsla,
    /// Background of inline code and code-block quads.
    pub code_bg: Hsla,
    /// Text colour inside code blocks.
    pub code_text: Hsla,
    /// Background of the focused code-block language input.
    pub code_language_input_bg: Hsla,
    /// Border colour of the focused code-block language input.
    pub code_language_input_border: Hsla,
    /// Text colour of the focused code-block language input.
    pub code_language_input_text: Hsla,
    /// Placeholder colour of the focused code-block language input.
    pub code_language_input_placeholder: Hsla,
    /// Syntax colour for comments inside code blocks.
    pub code_syntax_comment: Hsla,
    /// Syntax colour for keywords inside code blocks.
    pub code_syntax_keyword: Hsla,
    /// Syntax colour for strings inside code blocks.
    pub code_syntax_string: Hsla,
    /// Syntax colour for numbers inside code blocks.
    pub code_syntax_number: Hsla,
    /// Syntax colour for types and modules inside code blocks.
    pub code_syntax_type: Hsla,
    /// Syntax colour for functions and constructors inside code blocks.
    pub code_syntax_function: Hsla,
    /// Syntax colour for constants inside code blocks.
    pub code_syntax_constant: Hsla,
    /// Syntax colour for variables and parameters inside code blocks.
    pub code_syntax_variable: Hsla,
    /// Syntax colour for properties and attributes inside code blocks.
    pub code_syntax_property: Hsla,
    /// Syntax colour for operators inside code blocks.
    pub code_syntax_operator: Hsla,
    /// Syntax colour for punctuation inside code blocks.
    pub code_syntax_punctuation: Hsla,
    /// Border colour of native table cells.
    pub table_border: Hsla,
    /// Background of native table header cells.
    pub table_header_bg: Hsla,
    /// Background of native table body cells.
    pub table_cell_bg: Hsla,
    /// Outline colour of the active native table cell.
    pub table_cell_active_outline: Hsla,
    /// Preview highlight colour for row/column table-axis selection bands.
    pub table_axis_preview_bg: Hsla,
    /// Selected highlight colour for row/column table-axis selection bands.
    pub table_axis_selected_bg: Hsla,
    /// Background of rendered-mode native table append controls.
    pub table_append_button_bg: Hsla,
    /// Hover background of rendered-mode native table append controls.
    pub table_append_button_hover: Hsla,
    /// Text colour of rendered-mode native table append controls.
    pub table_append_button_text: Hsla,
    /// Background of table handle pills (row/column drag handles).
    pub table_handle_bg: Hsla,
    /// Icon colour of table handle dots.
    pub table_handle_icon: Hsla,
    /// Border colour of the selection frame around selected table columns/rows.
    pub table_selection_border: Hsla,
    /// Background of image placeholders in rendered mode.
    pub image_placeholder_bg: Hsla,
    /// Border colour of image placeholders in rendered mode.
    pub image_placeholder_border: Hsla,
    /// Text colour of image placeholders in rendered mode.
    pub image_placeholder_text: Hsla,
    /// Caption text colour shown below rendered images.
    pub image_caption_text: Hsla,
    /// Scrollbar thumb colour (auto-fading overlay).
    pub scrollbar_thumb: Hsla,
    /// Text-editing cursor (caret) colour.
    pub cursor: Hsla,
    /// Text-selection highlight colour.
    pub selection: Hsla,
    /// Semi-transparent backdrop behind the unsaved-changes dialog.
    pub dialog_backdrop: Hsla,
    /// Background of the unsaved-changes dialog.
    pub dialog_surface: Hsla,
    /// Border colour of the unsaved-changes dialog.
    pub dialog_border: Hsla,
    /// Title text colour in the unsaved-changes dialog.
    pub dialog_title: Hsla,
    /// Body text colour in the unsaved-changes dialog.
    pub dialog_body: Hsla,
    /// Muted / hint text colour in the unsaved-changes dialog.
    pub dialog_muted: Hsla,
    /// Primary (save-and-close) button background.
    pub dialog_primary_button_bg: Hsla,
    /// Primary button hover background.
    pub dialog_primary_button_hover: Hsla,
    /// Primary button text colour.
    pub dialog_primary_button_text: Hsla,
    /// Secondary (cancel) button background.
    pub dialog_secondary_button_bg: Hsla,
    /// Secondary button hover background.
    pub dialog_secondary_button_hover: Hsla,
    /// Secondary button text colour.
    pub dialog_secondary_button_text: Hsla,
    /// Danger (discard-and-close) button background.
    pub dialog_danger_button_bg: Hsla,
    /// Danger button hover background.
    pub dialog_danger_button_hover: Hsla,
    /// Danger button text colour.
    pub dialog_danger_button_text: Hsla,
    /// Background of the editor status bar.
    pub bottombar_background: Hsla,
    /// Primary text colour in the status bar.
    pub bottombar_text: Hsla,
    /// Dimmed/secondary text colour in the status bar.
    pub bottombar_text_dim: Hsla,
    /// Hover background for clickable status bar items.
    pub bottombar_button_hover: Hsla,
    /// Explorer panel row hover background (translucent highlight).
    pub panel_row_hover: Hsla,
    /// Explorer panel row selection background (light blue).
    pub panel_row_selected: Hsla,
}

/// Deserialization adapter for `ThemeColors` with backward-compatible defaults.
#[derive(Deserialize)]
struct ThemeColorsDe {
    editor_background: Hsla,
    source_mode_block_bg: Option<Hsla>,
    block_focused_bg: Option<Hsla>,
    comment_bg: Option<Hsla>,
    text_default: Hsla,
    text_link: Option<Hsla>,
    text_placeholder: Hsla,
    text_h1: Hsla,
    text_h2: Hsla,
    text_h3: Hsla,
    text_h4: Hsla,
    text_h5: Hsla,
    text_h6: Hsla,
    border_h1: Hsla,
    border_h2: Option<Hsla>,
    text_quote: Hsla,
    border_quote: Hsla,
    callout_note_bg: Option<Hsla>,
    callout_note_border: Option<Hsla>,
    callout_tip_bg: Option<Hsla>,
    callout_tip_border: Option<Hsla>,
    callout_important_bg: Option<Hsla>,
    callout_important_border: Option<Hsla>,
    callout_warning_bg: Option<Hsla>,
    callout_warning_border: Option<Hsla>,
    callout_caution_bg: Option<Hsla>,
    callout_caution_border: Option<Hsla>,
    footnote_bg: Option<Hsla>,
    footnote_border: Option<Hsla>,
    footnote_badge_bg: Option<Hsla>,
    footnote_badge_text: Option<Hsla>,
    footnote_backref: Option<Hsla>,
    task_checkbox_border: Option<Hsla>,
    task_checkbox_bg: Option<Hsla>,
    task_checkbox_checked_bg: Option<Hsla>,
    task_checkbox_check: Option<Hsla>,
    separator_color: Option<Hsla>,
    code_bg: Option<Hsla>,
    code_text: Hsla,
    code_language_input_bg: Option<Hsla>,
    code_language_input_border: Option<Hsla>,
    code_language_input_text: Option<Hsla>,
    code_language_input_placeholder: Option<Hsla>,
    code_syntax_comment: Option<Hsla>,
    code_syntax_keyword: Option<Hsla>,
    code_syntax_string: Option<Hsla>,
    code_syntax_number: Option<Hsla>,
    code_syntax_type: Option<Hsla>,
    code_syntax_function: Option<Hsla>,
    code_syntax_constant: Option<Hsla>,
    code_syntax_variable: Option<Hsla>,
    code_syntax_property: Option<Hsla>,
    code_syntax_operator: Option<Hsla>,
    code_syntax_punctuation: Option<Hsla>,
    table_border: Option<Hsla>,
    table_header_bg: Option<Hsla>,
    table_cell_bg: Option<Hsla>,
    table_cell_active_outline: Option<Hsla>,
    table_axis_preview_bg: Option<Hsla>,
    table_axis_selected_bg: Option<Hsla>,
    table_append_button_bg: Option<Hsla>,
    table_append_button_hover: Option<Hsla>,
    table_append_button_text: Option<Hsla>,
    table_handle_bg: Option<Hsla>,
    table_handle_icon: Option<Hsla>,
    table_selection_border: Option<Hsla>,
    image_placeholder_bg: Option<Hsla>,
    image_placeholder_border: Option<Hsla>,
    image_placeholder_text: Option<Hsla>,
    image_caption_text: Option<Hsla>,
    scrollbar_thumb: Hsla,
    cursor: Hsla,
    selection: Hsla,
    dialog_backdrop: Hsla,
    dialog_surface: Hsla,
    dialog_border: Hsla,
    dialog_title: Hsla,
    dialog_body: Hsla,
    dialog_muted: Hsla,
    dialog_primary_button_bg: Hsla,
    dialog_primary_button_hover: Hsla,
    dialog_primary_button_text: Hsla,
    dialog_secondary_button_bg: Hsla,
    dialog_secondary_button_hover: Hsla,
    dialog_secondary_button_text: Hsla,
    dialog_danger_button_bg: Hsla,
    dialog_danger_button_hover: Hsla,
    dialog_danger_button_text: Hsla,
    bottombar_background: Option<Hsla>,
    bottombar_text: Option<Hsla>,
    bottombar_text_dim: Option<Hsla>,
    bottombar_button_hover: Option<Hsla>,
    panel_row_hover: Option<Hsla>,
    panel_row_selected: Option<Hsla>,
}

impl<'de> Deserialize<'de> for ThemeColors {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ThemeColorsDe::deserialize(deserializer)?;
        Ok(Self {
            editor_background: raw.editor_background,
            source_mode_block_bg: raw
                .source_mode_block_bg
                .or(raw.block_focused_bg)
                .unwrap_or_else(|| Hsla::from(rgba(0x313131ff))),
            comment_bg: raw
                .comment_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xfbbf2426))),
            text_default: raw.text_default,
            text_link: raw
                .text_link
                .unwrap_or_else(|| Hsla::from(rgba(0x60a5faff))),
            text_placeholder: raw.text_placeholder,
            text_h1: raw.text_h1,
            text_h2: raw.text_h2,
            text_h3: raw.text_h3,
            text_h4: raw.text_h4,
            text_h5: raw.text_h5,
            text_h6: raw.text_h6,
            border_h1: raw.border_h1,
            border_h2: raw
                .border_h2
                .unwrap_or_else(|| Hsla::from(rgba(0xe0e0e0cc))),
            text_quote: raw.text_quote,
            border_quote: raw.border_quote,
            callout_note_bg: raw
                .callout_note_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x94a3b81f))),
            callout_note_border: raw
                .callout_note_border
                .unwrap_or_else(|| Hsla::from(rgba(0x94a3b4ff))),
            callout_tip_bg: raw
                .callout_tip_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x1d4ed81f))),
            callout_tip_border: raw
                .callout_tip_border
                .unwrap_or_else(|| Hsla::from(rgba(0x60a5faff))),
            callout_important_bg: raw
                .callout_important_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xca8a041f))),
            callout_important_border: raw
                .callout_important_border
                .unwrap_or_else(|| Hsla::from(rgba(0xfbbf24ff))),
            callout_warning_bg: raw
                .callout_warning_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xfb71851f))),
            callout_warning_border: raw
                .callout_warning_border
                .unwrap_or_else(|| Hsla::from(rgba(0xfb7185ff))),
            callout_caution_bg: raw
                .callout_caution_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xdc26261f))),
            callout_caution_border: raw
                .callout_caution_border
                .unwrap_or_else(|| Hsla::from(rgba(0xf87171ff))),
            footnote_bg: raw
                .footnote_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x212124ff))),
            footnote_border: raw
                .footnote_border
                .unwrap_or_else(|| Hsla::from(rgba(0x71717a52))),
            footnote_badge_bg: raw
                .footnote_badge_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xa1a1aa24))),
            footnote_badge_text: raw
                .footnote_badge_text
                .unwrap_or_else(|| Hsla::from(rgba(0xd4d4d8cc))),
            footnote_backref: raw
                .footnote_backref
                .unwrap_or_else(|| Hsla::from(rgba(0xa1a1aaff))),
            task_checkbox_border: raw
                .task_checkbox_border
                .unwrap_or_else(|| Hsla::from(rgba(0x71717aff))),
            task_checkbox_bg: raw
                .task_checkbox_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x00000000))),
            task_checkbox_checked_bg: raw
                .task_checkbox_checked_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xf0efedff))),
            task_checkbox_check: raw
                .task_checkbox_check
                .unwrap_or_else(|| Hsla::from(rgba(0x18181bff))),
            separator_color: raw
                .separator_color
                .unwrap_or_else(|| Hsla::from(rgba(0x71717aff))),
            code_bg: raw.code_bg.unwrap_or_else(|| Hsla::from(rgba(0x111827ff))),
            code_text: raw.code_text,
            code_language_input_bg: raw
                .code_language_input_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x343941ff))),
            code_language_input_border: raw
                .code_language_input_border
                .unwrap_or_else(|| Hsla::from(rgba(0x4b5563cc))),
            code_language_input_text: raw
                .code_language_input_text
                .unwrap_or_else(|| Hsla::from(rgba(0xe5e7ebff))),
            code_language_input_placeholder: raw
                .code_language_input_placeholder
                .unwrap_or_else(|| Hsla::from(rgba(0x9ca3afcc))),
            code_syntax_comment: raw
                .code_syntax_comment
                .unwrap_or_else(|| Hsla::from(rgba(0x565f89ff))),
            code_syntax_keyword: raw
                .code_syntax_keyword
                .unwrap_or_else(|| Hsla::from(rgba(0xbb9af7ff))),
            code_syntax_string: raw
                .code_syntax_string
                .unwrap_or_else(|| Hsla::from(rgba(0x9ece6aff))),
            code_syntax_number: raw
                .code_syntax_number
                .unwrap_or_else(|| Hsla::from(rgba(0xff9e64ff))),
            code_syntax_type: raw
                .code_syntax_type
                .unwrap_or_else(|| Hsla::from(rgba(0x2ac3deff))),
            code_syntax_function: raw
                .code_syntax_function
                .unwrap_or_else(|| Hsla::from(rgba(0x7aa2f7ff))),
            code_syntax_constant: raw
                .code_syntax_constant
                .unwrap_or_else(|| Hsla::from(rgba(0xffd166ff))),
            code_syntax_variable: raw
                .code_syntax_variable
                .unwrap_or_else(|| Hsla::from(rgba(0xe5e9f0ff))),
            code_syntax_property: raw
                .code_syntax_property
                .unwrap_or_else(|| Hsla::from(rgba(0x7dcfffcc))),
            code_syntax_operator: raw
                .code_syntax_operator
                .unwrap_or_else(|| Hsla::from(rgba(0x89ddffff))),
            code_syntax_punctuation: raw
                .code_syntax_punctuation
                .unwrap_or_else(|| Hsla::from(rgba(0x9aa5ceff))),
            table_border: raw
                .table_border
                .unwrap_or_else(|| Hsla::from(rgba(0x3f3f46ff))),
            table_header_bg: raw
                .table_header_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x232326ff))),
            table_cell_bg: raw
                .table_cell_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x1d1d20ff))),
            table_cell_active_outline: raw
                .table_cell_active_outline
                .unwrap_or_else(|| Hsla::from(rgba(0x60a5faff))),
            table_axis_preview_bg: raw
                .table_axis_preview_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xf4f4f51a))),
            table_axis_selected_bg: raw
                .table_axis_selected_bg
                .unwrap_or_else(|| Hsla::from(rgba(0xf4f4f533))),
            table_append_button_bg: raw
                .table_append_button_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x27272aff))),
            table_append_button_hover: raw
                .table_append_button_hover
                .unwrap_or_else(|| Hsla::from(rgba(0x3f3f46ff))),
            table_append_button_text: raw
                .table_append_button_text
                .unwrap_or_else(|| Hsla::from(rgba(0xf4f4f5ff))),
            table_handle_bg: raw
                .table_handle_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x3f3f46ff))),
            table_handle_icon: raw
                .table_handle_icon
                .unwrap_or_else(|| Hsla::from(rgba(0xa1a1aaff))),
            table_selection_border: raw
                .table_selection_border
                .unwrap_or_else(|| Hsla::from(rgba(0x60a5faff))),
            image_placeholder_bg: raw
                .image_placeholder_bg
                .unwrap_or_else(|| Hsla::from(rgba(0x202024ff))),
            image_placeholder_border: raw
                .image_placeholder_border
                .unwrap_or_else(|| Hsla::from(rgba(0x52525bff))),
            image_placeholder_text: raw
                .image_placeholder_text
                .unwrap_or_else(|| Hsla::from(rgba(0xd4d4d8ff))),
            image_caption_text: raw
                .image_caption_text
                .unwrap_or_else(|| Hsla::from(rgba(0xa1a1aaff))),
            scrollbar_thumb: raw.scrollbar_thumb,
            cursor: raw.cursor,
            selection: raw.selection,
            dialog_backdrop: raw.dialog_backdrop,
            dialog_surface: raw.dialog_surface,
            dialog_border: raw.dialog_border,
            dialog_title: raw.dialog_title,
            dialog_body: raw.dialog_body,
            dialog_muted: raw.dialog_muted,
            dialog_primary_button_bg: raw.dialog_primary_button_bg,
            dialog_primary_button_hover: raw.dialog_primary_button_hover,
            dialog_primary_button_text: raw.dialog_primary_button_text,
            dialog_secondary_button_bg: raw.dialog_secondary_button_bg,
            dialog_secondary_button_hover: raw.dialog_secondary_button_hover,
            dialog_secondary_button_text: raw.dialog_secondary_button_text,
            dialog_danger_button_bg: raw.dialog_danger_button_bg,
            dialog_danger_button_hover: raw.dialog_danger_button_hover,
            dialog_danger_button_text: raw.dialog_danger_button_text,
            bottombar_background: raw
                .bottombar_background
                .unwrap_or_else(|| Hsla::from(rgba(0x1c1c1fff))),
            bottombar_text: raw
                .bottombar_text
                .unwrap_or_else(|| Hsla::from(rgba(0xd4d4d8cc))),
            bottombar_text_dim: raw
                .bottombar_text_dim
                .unwrap_or_else(|| Hsla::from(rgba(0x71717aff))),
            bottombar_button_hover: raw
                .bottombar_button_hover
                .unwrap_or_else(|| Hsla::from(rgba(0x3f3f46ff))),
            panel_row_hover: raw
                .panel_row_hover
                .unwrap_or_else(|| Hsla::from(rgba(0xffffff14))),
            panel_row_selected: raw
                .panel_row_selected
                .unwrap_or_else(|| Hsla::from(rgba(0x3b82f63d))),
        })
    }
}
