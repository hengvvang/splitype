//! Theme struct and trait definitions.

use anyhow::{Context as _, bail};
use gpui::{Hsla, hsla, rgba};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::infra::config::jsonc::{
    merge_non_empty_json_values, object_without_empty_values, prune_empty_json_values,
    sanitize_config_file_stem,
};

use super::colors::ThemeColors;
use super::dimensions::ThemeDimensions;
use super::typography::{FontWeightDef, ThemeTypography};

/// Placeholder text shown in empty interactive elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placeholders {
    /// Text shown in an empty focused block.
    pub empty_editing: String,
}

/// Computed heading typography and layout style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeadingStyle {
    pub text_color: Hsla,
    pub font_size: f32,
    pub font_weight: gpui::FontWeight,
    pub padding_bottom: f32,
    pub margin_bottom: f32,
    pub border_width: f32,
    pub border_color: Option<Hsla>,
}

/// Computed callout colors from the theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalloutStyle {
    pub border_color: Hsla,
    pub background_color: Hsla,
}

/// Top-level theme combining colors, dimensions, typography and placeholders.
///
/// Can be deserialized from JSON, allowing users to ship custom theme files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    pub dimensions: ThemeDimensions,
    pub typography: ThemeTypography,
    pub placeholders: Placeholders,
}

impl Theme {
    /// Returns the unified heading style for a given heading level (1–6).
    pub fn heading_style(&self, level: u8) -> HeadingStyle {
        let c = &self.colors;
        let d = &self.dimensions;
        let t = &self.typography;
        match level {
            1 => HeadingStyle {
                text_color: c.text_h1,
                font_size: t.h1_size,
                font_weight: t.h1_weight.to_font_weight(),
                padding_bottom: d.h1_padding_bottom,
                margin_bottom: d.h1_margin_bottom,
                border_width: d.h1_border_width,
                border_color: Some(c.border_h1),
            },
            2 => HeadingStyle {
                text_color: c.text_h2,
                font_size: t.h2_size,
                font_weight: t.h2_weight.to_font_weight(),
                padding_bottom: d.h1_padding_bottom,
                margin_bottom: d.h1_margin_bottom,
                border_width: d.h1_border_width,
                border_color: Some(c.border_h2),
            },
            3 => HeadingStyle {
                text_color: c.text_h3,
                font_size: t.h3_size,
                font_weight: t.h3_weight.to_font_weight(),
                padding_bottom: 0.0,
                margin_bottom: 0.0,
                border_width: 0.0,
                border_color: None,
            },
            4 => HeadingStyle {
                text_color: c.text_h4,
                font_size: t.h4_size,
                font_weight: t.h4_weight.to_font_weight(),
                padding_bottom: 0.0,
                margin_bottom: 0.0,
                border_width: 0.0,
                border_color: None,
            },
            5 => HeadingStyle {
                text_color: c.text_h5,
                font_size: t.h5_size,
                font_weight: t.h5_weight.to_font_weight(),
                padding_bottom: 0.0,
                margin_bottom: 0.0,
                border_width: 0.0,
                border_color: None,
            },
            6 => HeadingStyle {
                text_color: c.text_h6,
                font_size: t.h6_size,
                font_weight: t.h6_weight.to_font_weight(),
                padding_bottom: 0.0,
                margin_bottom: 0.0,
                border_width: 0.0,
                border_color: None,
            },
            _ => HeadingStyle {
                text_color: c.text_default,
                font_size: t.text_size,
                font_weight: gpui::FontWeight::NORMAL,
                padding_bottom: 0.0,
                margin_bottom: 0.0,
                border_width: 0.0,
                border_color: None,
            },
        }
    }

    /// Returns the unified callout colors for a given callout kind.
    pub fn callout_style(&self, variant: crate::model::block::CalloutKind) -> CalloutStyle {
        let c = &self.colors;
        match variant {
            crate::model::block::CalloutKind::Note => CalloutStyle {
                border_color: c.callout_note_border,
                background_color: c.callout_note_bg,
            },
            crate::model::block::CalloutKind::Tip => CalloutStyle {
                border_color: c.callout_tip_border,
                background_color: c.callout_tip_bg,
            },
            crate::model::block::CalloutKind::Important => CalloutStyle {
                border_color: c.callout_important_border,
                background_color: c.callout_important_bg,
            },
            crate::model::block::CalloutKind::Warning => CalloutStyle {
                border_color: c.callout_warning_border,
                background_color: c.callout_warning_bg,
            },
            crate::model::block::CalloutKind::Caution => CalloutStyle {
                border_color: c.callout_caution_border,
                background_color: c.callout_caution_bg,
            },
        }
    }
    /// Returns the built-in fallback theme used when no custom theme is loaded.
    pub fn default_theme() -> Self {
        Self {
            name: "Dark".into(),
            colors: ThemeColors {
                editor_background: Hsla::from(rgba(0x191919ff)),
                source_mode_block_bg: Hsla::from(rgba(0x313131ff)),
                comment_bg: Hsla::from(rgba(0xfbbf2426)),
                text_default: Hsla::from(rgba(0xf0efedff)),
                text_link: Hsla::from(rgba(0x60a5faff)),
                markdown_marker: Hsla::from(rgba(0x89ddffff)),
                text_placeholder: hsla(0., 0., 0.6, 1.0),
                text_h1: Hsla::from(rgba(0xf0efedff)),
                text_h2: Hsla::from(rgba(0xf0efedff)),
                text_h3: Hsla::from(rgba(0xf0efedff)),
                text_h4: Hsla::from(rgba(0xf0efedff)),
                text_h5: Hsla::from(rgba(0xf0efedff)),
                text_h6: Hsla::from(rgba(0xf0efedff)),
                border_h1: Hsla::from(rgba(0xe0e0e0ff)),
                border_h2: Hsla::from(rgba(0xe0e0e0cc)),
                text_quote: Hsla::from(rgba(0xd1d5dbff)),
                border_quote: Hsla::from(rgba(0x6b7280ff)),
                callout_note_bg: Hsla::from(rgba(0x94a3b81f)),
                callout_note_border: Hsla::from(rgba(0x94a3b4ff)),
                callout_tip_bg: Hsla::from(rgba(0x1d4ed81f)),
                callout_tip_border: Hsla::from(rgba(0x60a5faff)),
                callout_important_bg: Hsla::from(rgba(0xa78bfa1f)),
                callout_important_border: Hsla::from(rgba(0xa78bfaff)),
                callout_warning_bg: Hsla::from(rgba(0xfb71851f)),
                callout_warning_border: Hsla::from(rgba(0xfb7185ff)),
                callout_caution_bg: Hsla::from(rgba(0xdc26261f)),
                callout_caution_border: Hsla::from(rgba(0xf87171ff)),
                footnote_border: Hsla::from(rgba(0x71717a52)),
                footnote_backref: Hsla::from(rgba(0xa1a1aaff)),
                task_checkbox_border: Hsla::from(rgba(0xffffff66)),
                task_checkbox_bg: Hsla::from(rgba(0x00000000)),
                task_checkbox_checked_bg: Hsla::from(rgba(0x2383e2ff)),
                task_checkbox_check: Hsla::from(rgba(0xffffffff)),
                separator: Hsla::from(rgba(0xd1d1d8ff)),
                code_bg: Hsla::from(rgba(0x23272eff)),
                code_text: Hsla::from(rgba(0xe5e7ebff)),
                code_language_input_bg: Hsla::from(rgba(0x343941ff)),
                code_language_input_border: Hsla::from(rgba(0x4b5563cc)),
                code_language_input_text: Hsla::from(rgba(0xe5e7ebff)),
                code_language_input_placeholder: Hsla::from(rgba(0x9ca3afcc)),
                code_syntax_comment: Hsla::from(rgba(0x565f89ff)),
                code_syntax_keyword: Hsla::from(rgba(0xbb9af7ff)),
                code_syntax_string: Hsla::from(rgba(0x9ece6aff)),
                code_syntax_number: Hsla::from(rgba(0xff9e64ff)),
                code_syntax_type: Hsla::from(rgba(0x2ac3deff)),
                code_syntax_function: Hsla::from(rgba(0x7aa2f7ff)),
                code_syntax_constant: Hsla::from(rgba(0xffd166ff)),
                code_syntax_variable: Hsla::from(rgba(0xe5e9f0ff)),
                code_syntax_property: Hsla::from(rgba(0x7dcfffcc)),
                code_syntax_operator: Hsla::from(rgba(0x89ddffff)),
                code_syntax_punctuation: Hsla::from(rgba(0x9aa5ceff)),
                table_border: Hsla::from(rgba(0x3f3f46ff)),
                table_header_bg: Hsla::from(rgba(0x232326ff)),
                table_cell_bg: Hsla::from(rgba(0x1d1d20ff)),
                table_cell_active_outline: Hsla::from(rgba(0x60a5faff)),
                table_axis_preview_bg: Hsla::from(rgba(0xf4f4f51a)),
                table_axis_selected_bg: Hsla::from(rgba(0xf4f4f533)),
                table_append_button_bg: Hsla::from(rgba(0x27272aff)),
                table_append_button_hover: Hsla::from(rgba(0x3f3f46ff)),
                table_append_button_text: Hsla::from(rgba(0xf4f4f5ff)),
                table_handle_bg: Hsla::from(rgba(0x3f3f46ff)),
                table_handle_icon: Hsla::from(rgba(0xa1a1aaff)),
                table_selection_border: Hsla::from(rgba(0x60a5faff)),
                image_placeholder_bg: Hsla::from(rgba(0x202024ff)),
                image_placeholder_border: Hsla::from(rgba(0x52525bff)),
                image_placeholder_text: Hsla::from(rgba(0xd4d4d8ff)),
                image_caption_text: Hsla::from(rgba(0xa1a1aaff)),
                scrollbar_thumb: Hsla::from(rgba(0xd1d5dbd8)),
                cursor: Hsla::from(rgba(0xf0efedff)),
                selection: Hsla::from(rgba(0x1c3651ff)),
                focus_accent: Hsla::from(rgba(0x72cfefff)),
                split_indicator: Hsla::from(rgba(0x60a5faff)),
                dialog_backdrop: Hsla::from(rgba(0x09090bcc)),
                dialog_surface: Hsla::from(rgba(0x18181bff)),
                dialog_border: Hsla::from(rgba(0x27272aff)),
                dialog_title: Hsla::from(rgba(0xf4f4f5ff)),
                dialog_body: Hsla::from(rgba(0xd4d4d8ff)),
                dialog_muted: Hsla::from(rgba(0xa1a1aaff)),
                dialog_primary_button_bg: Hsla::from(rgba(0xf4f4f5ff)),
                dialog_primary_button_hover: Hsla::from(rgba(0xe4e4e7ff)),
                dialog_primary_button_text: Hsla::from(rgba(0x18181bff)),
                dialog_secondary_button_bg: Hsla::from(rgba(0x27272aff)),
                dialog_secondary_button_hover: Hsla::from(rgba(0x3f3f46ff)),
                dialog_secondary_button_text: Hsla::from(rgba(0xf4f4f5ff)),
                app_menu_active: Hsla::from(rgba(0x2383e2ff)),
                // Doubles as the destructive menu-item text color (e.g. Delete
                // Row/Column), so it must stay legible on the dark menu surface
                // rather than the muted red used previously.
                dialog_danger_button_bg: Hsla::from(rgba(0xef4444ff)),
                dialog_danger_button_hover: Hsla::from(rgba(0xdc2626ff)),
                dialog_danger_button_text: Hsla::from(rgba(0xfef2f2ff)),
                bottombar_background: Hsla::from(rgba(0x1c1c1fff)),
                bottombar_text: Hsla::from(rgba(0xd4d4d8cc)),
                bottombar_text_dim: Hsla::from(rgba(0x71717aff)),
                bottombar_button_hover: Hsla::from(rgba(0x3f3f46ff)),
                // Translucent white row highlight (dark theme).
                panel_row_hover: Hsla::from(rgba(0xffffff14)),
                // Light blue selection (dark theme).
                panel_row_selected: Hsla::from(rgba(0x3b82f63d)),
            },
            dimensions: ThemeDimensions {
                editor_padding: 24.0,
                block_gap: 6.0,
                block_min_height: 28.0,
                block_padding_y: 4.0,
                block_padding_x: 12.0,
                nested_block_indent: 20.0,
                list_marker_gap: 8.0,
                list_marker_width: 12.0,
                ordered_list_marker_width: 20.0,
                task_checkbox_size: 16.0,
                task_checkbox_radius: 2.0,
                task_checkbox_border_width: 1.5,
                task_checkbox_check_size: 14.5,
                h1_padding_bottom: 4.0,
                h1_margin_bottom: 4.0,
                cursor_width: 2.0,
                underline_thickness: 1.0,
                h1_border_width: 1.0,
                quote_border_width: 3.0,
                quote_padding_left: 12.0,
                callout_padding_x: 8.0,
                callout_padding_y: 10.0,
                callout_body_gap: 8.0,
                callout_radius: 6.0,
                callout_border_width: 3.0,
                callout_header_gap: 6.0,
                callout_header_margin_bottom: 6.0,
                separator_thickness: 4.0,
                separator_inset_x: 40.0,
                separator_margin_y: 10.0,
                code_block_padding_y: 8.0,
                code_block_padding_x: 12.0,
                code_bg_pad_x: 3.0,
                code_bg_pad_y: 1.0,
                code_bg_radius: 4.0,
                code_language_input_width: 156.0,
                code_language_input_height: 18.0,
                code_language_input_padding_x: 8.0,
                code_language_input_padding_y: 3.0,
                code_language_input_radius: 6.0,
                code_language_input_border_width: 1.0,
                code_language_input_gap: 8.0,
                table_cell_padding_x: 10.0,
                table_cell_padding_y: 8.0,
                table_cell_min_height: 42.0,
                table_append_button_extent: 16.0,
                table_append_button_inset: 8.0,
                table_append_activation_band: 18.0,
                table_border_radius: 4.0,
                table_handle_width: 10.0,
                table_handle_height: 36.0,
                table_selection_border_width: 2.0,
                image_radius: 12.0,
                image_root_max_height: 420.0,
                image_cell_max_height: 180.0,
                image_root_placeholder_height: 260.0,
                image_cell_placeholder_height: 120.0,
                image_caption_gap: 8.0,
                scrollbar_width: 6.0,
                scrollbar_right: 6.0,
                centered_shrink_start: 1100.0,
                centered_shrink_end: 2200.0,
                centered_min_ratio: 0.58,
                dialog_width: 460.0,
                dialog_padding: 20.0,
                dialog_gap: 14.0,
                dialog_radius: 14.0,
                dialog_border_width: 1.0,
                dialog_button_height: 36.0,
                dialog_button_gap: 10.0,
                dialog_button_padding_x: 14.0,
                menu_bar_height: 32.0,
                menu_bar_padding_x: 10.0,
                menu_bar_padding_y: 4.0,
                menu_bar_gap: 2.0,
                menu_bar_button_width: 48.0,
                menu_bar_button_height: 24.0,
                menu_bar_button_padding_x: 8.0,
                menu_bar_button_radius: 3.0,
                menu_text_size: 11.0,
                menu_panel_top: 2.0,
                menu_panel_width: 180.0,
                menu_panel_padding: 4.0,
                menu_panel_gap: 1.0,
                menu_panel_radius: 3.0,
                menu_item_height: 28.0,
                menu_item_padding_x: 8.0,
                menu_item_radius: 3.0,
                menu_separator_margin_x: 6.0,
                menu_separator_margin_y: 3.0,
                menu_separator_height: 1.0,
                context_menu_panel_width: 132.0,
                context_menu_submenu_width: 148.0,
                context_menu_submenu_gap: 2.0,
                context_menu_axis_panel_width: 164.0,
                table_insert_dialog_width: 380.0,
                table_insert_stepper_gap: 8.0,
                table_insert_stepper_button_size: 32.0,
                table_insert_stepper_value_min_width: 56.0,
                table_insert_stepper_value_padding_x: 10.0,
                table_insert_stepper_radius: 8.0,
                topbar_height: 28.0,
                bottombar_height: 28.0,
                bottombar_padding_x: 12.0,
                bottombar_item_gap: 12.0,
                bottombar_text_size: 11.0,
                panel_tile_gap: 6.0,
                pane_gap: 3.0,
                panel_tile_radius: 3.0,
            },
            typography: ThemeTypography {
                text_size: 17.0,
                text_line_height: 1.6,
                h1_size: 32.0,
                h1_weight: FontWeightDef::Bold,
                h2_size: 24.0,
                h2_weight: FontWeightDef::Bold,
                h3_size: 20.0,
                h3_weight: FontWeightDef::Semibold,
                h4_size: 18.0,
                h4_weight: FontWeightDef::Semibold,
                h5_size: 16.0,
                h5_weight: FontWeightDef::Semibold,
                h6_size: 14.0,
                h6_weight: FontWeightDef::Semibold,
                code_size: 15.0,
                dialog_title_size: 20.0,
                dialog_title_weight: FontWeightDef::Semibold,
                dialog_body_size: 14.0,
                dialog_body_weight: FontWeightDef::Normal,
                dialog_button_size: 14.0,
                dialog_button_weight: FontWeightDef::Medium,
            },
            placeholders: Placeholders {
                empty_editing: String::new(),
            },
        }
    }

    /// Returns the built-in light theme.
    ///
    /// The light theme intentionally reuses the default layout and typography
    /// tokens so it can focus on palette differences.
    pub fn light_theme() -> Self {
        let base = Self::default_theme();
        Self {
            name: BUILTIN_THEME_SPLITYPE_LIGHT_NAME.into(),
            colors: ThemeColors {
                editor_background: Hsla::from(rgba(0xffffffff)),
                source_mode_block_bg: Hsla::from(rgba(0xeef2f7ff)),
                comment_bg: Hsla::from(rgba(0xfef3c766)),
                text_default: Hsla::from(rgba(0x1f2937ff)),
                text_link: Hsla::from(rgba(0x2563ebff)),
                markdown_marker: Hsla::from(rgba(0x9333eaff)),
                text_placeholder: Hsla::from(rgba(0x6b7280cc)),
                text_h1: Hsla::from(rgba(0x111827ff)),
                text_h2: Hsla::from(rgba(0x111827ff)),
                text_h3: Hsla::from(rgba(0x111827ff)),
                text_h4: Hsla::from(rgba(0x111827ff)),
                text_h5: Hsla::from(rgba(0x111827ff)),
                text_h6: Hsla::from(rgba(0x111827ff)),
                border_h1: Hsla::from(rgba(0xcbd5e1ff)),
                border_h2: Hsla::from(rgba(0xdbe3efff)),
                text_quote: Hsla::from(rgba(0x475569ff)),
                border_quote: Hsla::from(rgba(0x94a3b8ff)),
                callout_note_bg: Hsla::from(rgba(0x2563eb14)),
                callout_note_border: Hsla::from(rgba(0x2563ebff)),
                callout_tip_bg: Hsla::from(rgba(0x16a34a14)),
                callout_tip_border: Hsla::from(rgba(0x16a34aff)),
                callout_important_bg: Hsla::from(rgba(0x7c3aed14)),
                callout_important_border: Hsla::from(rgba(0x7c3aedff)),
                callout_warning_bg: Hsla::from(rgba(0xf9731614)),
                callout_warning_border: Hsla::from(rgba(0xf97316ff)),
                callout_caution_bg: Hsla::from(rgba(0xdc262614)),
                callout_caution_border: Hsla::from(rgba(0xdc2626ff)),
                footnote_border: Hsla::from(rgba(0xcbd5e1ff)),
                footnote_backref: Hsla::from(rgba(0x2563ebff)),
                task_checkbox_border: Hsla::from(rgba(0x2b2b2bff)),
                task_checkbox_bg: Hsla::from(rgba(0xffffffff)),
                task_checkbox_checked_bg: Hsla::from(rgba(0x2383e2ff)),
                task_checkbox_check: Hsla::from(rgba(0xffffffff)),
                separator: Hsla::from(rgba(0xd5dde6ff)),
                code_bg: Hsla::from(rgba(0xf1f5f9ff)),
                code_text: Hsla::from(rgba(0x111827ff)),
                code_language_input_bg: Hsla::from(rgba(0xffffffff)),
                code_language_input_border: Hsla::from(rgba(0xcbd5e1ff)),
                code_language_input_text: Hsla::from(rgba(0x1f2937ff)),
                code_language_input_placeholder: Hsla::from(rgba(0x64748bcc)),
                code_syntax_comment: Hsla::from(rgba(0x6b7280ff)),
                code_syntax_keyword: Hsla::from(rgba(0x7c3aedff)),
                code_syntax_string: Hsla::from(rgba(0x15803dff)),
                code_syntax_number: Hsla::from(rgba(0xc2410cff)),
                code_syntax_type: Hsla::from(rgba(0x0f766eff)),
                code_syntax_function: Hsla::from(rgba(0x2563ebff)),
                code_syntax_constant: Hsla::from(rgba(0xb45309ff)),
                code_syntax_variable: Hsla::from(rgba(0x1f2937ff)),
                code_syntax_property: Hsla::from(rgba(0x0891b2ff)),
                code_syntax_operator: Hsla::from(rgba(0x9333eaff)),
                code_syntax_punctuation: Hsla::from(rgba(0x64748bff)),
                table_border: Hsla::from(rgba(0xd1d5dbff)),
                table_header_bg: Hsla::from(rgba(0xf1f5f9ff)),
                table_cell_bg: Hsla::from(rgba(0xffffffff)),
                table_cell_active_outline: Hsla::from(rgba(0x2563ebff)),
                table_axis_preview_bg: Hsla::from(rgba(0x2563eb14)),
                table_axis_selected_bg: Hsla::from(rgba(0x2563eb29)),
                table_append_button_bg: Hsla::from(rgba(0xe2e8f0ff)),
                table_append_button_hover: Hsla::from(rgba(0xcbd5e1ff)),
                table_append_button_text: Hsla::from(rgba(0x334155ff)),
                table_handle_bg: Hsla::from(rgba(0xcbd5e1ff)),
                table_handle_icon: Hsla::from(rgba(0x64748bff)),
                table_selection_border: Hsla::from(rgba(0x2563ebff)),
                image_placeholder_bg: Hsla::from(rgba(0xf8fafcff)),
                image_placeholder_border: Hsla::from(rgba(0xcbd5e1ff)),
                image_placeholder_text: Hsla::from(rgba(0x475569ff)),
                image_caption_text: Hsla::from(rgba(0x64748bff)),
                scrollbar_thumb: Hsla::from(rgba(0x64748bb8)),
                cursor: Hsla::from(rgba(0x111827ff)),
                selection: Hsla::from(rgba(0xbfdbfecc)),
                focus_accent: Hsla::from(rgba(0x0284c7ff)),
                split_indicator: Hsla::from(rgba(0x2563ebff)),
                dialog_backdrop: Hsla::from(rgba(0x0f172a66)),
                dialog_surface: Hsla::from(rgba(0xffffffff)),
                dialog_border: Hsla::from(rgba(0xd1d5dbff)),
                dialog_title: Hsla::from(rgba(0x111827ff)),
                dialog_body: Hsla::from(rgba(0x374151ff)),
                dialog_muted: Hsla::from(rgba(0x6b7280ff)),
                dialog_primary_button_bg: Hsla::from(rgba(0x2563ebff)),
                dialog_primary_button_hover: Hsla::from(rgba(0x1d4ed8ff)),
                dialog_primary_button_text: Hsla::from(rgba(0xffffffff)),
                dialog_secondary_button_bg: Hsla::from(rgba(0xf1f5f9ff)),
                dialog_secondary_button_hover: Hsla::from(rgba(0xe2e8f0ff)),
                dialog_secondary_button_text: Hsla::from(rgba(0x1f2937ff)),
                app_menu_active: Hsla::from(rgba(0x2383e2ff)),
                dialog_danger_button_bg: Hsla::from(rgba(0xdc2626ff)),
                dialog_danger_button_hover: Hsla::from(rgba(0xb91c1cff)),
                dialog_danger_button_text: Hsla::from(rgba(0xffffffff)),
                bottombar_background: Hsla::from(rgba(0xe2e8f0ff)),
                bottombar_text: Hsla::from(rgba(0x334155ff)),
                bottombar_text_dim: Hsla::from(rgba(0x64748bff)),
                bottombar_button_hover: Hsla::from(rgba(0xcbd5e1ff)),
                // Light-grey row highlight, clearly visible on white (light theme).
                panel_row_hover: Hsla::from(rgba(0xf2f2f2ff)),
                // Bright, saturated light-blue selection (light theme).
                panel_row_selected: Hsla::from(rgba(0x60a5fa33)),
            },
            dimensions: base.dimensions,
            typography: base.typography,
            placeholders: base.placeholders,
        }
    }

    /// Parses a theme from JSON text.
    #[cfg(test)]
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Serializes the theme into pretty-printed JSON.
    #[cfg(test)]
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Metadata for a selectable theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeCatalogEntry {
    pub id: String,
    pub name: String,
}

pub const BUILTIN_THEME_SPLITYPE_ID: &str = "splitype";
pub const BUILTIN_THEME_SPLITYPE_NAME: &str = "Dark";
pub const BUILTIN_THEME_SPLITYPE_LIGHT_ID: &str = "splitype-light";
pub const BUILTIN_THEME_SPLITYPE_LIGHT_NAME: &str = "Light";
pub const CUSTOM_THEME_ID: &str = "custom";

pub fn builtin_theme_catalog() -> Vec<ThemeCatalogEntry> {
    vec![
        ThemeCatalogEntry {
            id: BUILTIN_THEME_SPLITYPE_ID.into(),
            name: BUILTIN_THEME_SPLITYPE_NAME.into(),
        },
        ThemeCatalogEntry {
            id: BUILTIN_THEME_SPLITYPE_LIGHT_ID.into(),
            name: BUILTIN_THEME_SPLITYPE_LIGHT_NAME.into(),
        },
    ]
}

#[derive(Debug, Clone)]
pub struct CustomThemeEntry {
    pub id: String,
    pub name: String,
    pub creator: String,
    pub base_theme_id: String,
    pub theme: Theme,
}

pub fn custom_theme_from_value(value: Value) -> anyhow::Result<(CustomThemeEntry, Value)> {
    custom_theme_from_value_with_default_base(value, BUILTIN_THEME_SPLITYPE_ID)
}

pub fn custom_theme_from_value_with_default_base(
    mut value: Value,
    default_base_theme_id: &str,
) -> anyhow::Result<(CustomThemeEntry, Value)> {
    prune_empty_json_values(&mut value);
    let Value::Object(mut object) = value else {
        bail!("theme config must be a JSON object");
    };
    let object = object_without_empty_values(std::mem::take(&mut object));
    let name = required_string(&object, "name")?;
    let creator = required_string(&object, "creator")?;
    let base_theme_id = resolved_custom_theme_base_id(&object, default_base_theme_id);
    let raw_theme_patch = object
        .get("theme")
        .cloned()
        .unwrap_or_else(|| Value::Object(Map::new()));
    if !raw_theme_patch.is_object() {
        bail!("field 'theme' must be a JSON object when present");
    }

    let base_theme = custom_theme_base_theme(&base_theme_id);
    let mut merged = serde_json::to_value(base_theme)?;
    let mut theme_patch = filter_json_by_schema(&raw_theme_patch, &merged);
    if let Value::Object(theme_patch_object) = &mut theme_patch {
        theme_patch_object.remove("name");
    }
    merge_non_empty_json_values(&mut merged, &theme_patch);
    if let Value::Object(merged_object) = &mut merged {
        merged_object.insert("name".into(), Value::String(name.clone()));
    }
    let theme: Theme = serde_json::from_value(merged)
        .with_context(|| format!("failed to construct custom theme '{name}'"))?;
    let id = format!(
        "custom:{}_{}",
        sanitize_config_file_stem(&name),
        sanitize_config_file_stem(&creator)
    );
    let mut normalized_object = Map::new();
    normalized_object.insert("name".into(), Value::String(name.clone()));
    normalized_object.insert("creator".into(), Value::String(creator.clone()));
    normalized_object.insert(
        "base_theme_id".into(),
        Value::String(base_theme_id.to_string()),
    );
    for key in ["description", "version", "homepage", "license"] {
        if let Some(value) = object.get(key) {
            normalized_object.insert(key.into(), value.clone());
        }
    }
    if !theme_patch
        .as_object()
        .map(|object| object.is_empty())
        .unwrap_or(false)
    {
        normalized_object.insert("theme".into(), theme_patch);
    }
    let normalized = Value::Object(normalized_object);

    Ok((
        CustomThemeEntry {
            id,
            name,
            creator,
            base_theme_id: base_theme_id.to_string(),
            theme,
        },
        normalized,
    ))
}

fn resolved_custom_theme_base_id<'a>(
    object: &'a Map<String, Value>,
    default_base_theme_id: &'a str,
) -> &'a str {
    object
        .get("base_theme_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| is_builtin_theme_id(value))
        .unwrap_or_else(|| {
            if is_builtin_theme_id(default_base_theme_id) {
                default_base_theme_id
            } else {
                BUILTIN_THEME_SPLITYPE_ID
            }
        })
}

fn is_builtin_theme_id(theme_id: &str) -> bool {
    theme_id == BUILTIN_THEME_SPLITYPE_ID || theme_id == BUILTIN_THEME_SPLITYPE_LIGHT_ID
}

fn custom_theme_base_theme(theme_id: &str) -> Theme {
    if theme_id == BUILTIN_THEME_SPLITYPE_LIGHT_ID {
        Theme::light_theme()
    } else {
        Theme::default_theme()
    }
}

fn filter_json_by_schema(value: &Value, schema: &Value) -> Value {
    match (value, schema) {
        (Value::Object(value_object), Value::Object(schema_object)) => {
            let mut filtered = Map::new();
            for (key, value) in value_object {
                if let Some(schema_value) = schema_object.get(key) {
                    filtered.insert(key.clone(), filter_json_by_schema(value, schema_value));
                }
            }
            Value::Object(filtered)
        }
        (value, _) => value.clone(),
    }
}

fn required_string(object: &Map<String, Value>, key: &str) -> anyhow::Result<String> {
    let Some(value) = object.get(key) else {
        bail!("missing required field '{key}'");
    };
    let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    else {
        bail!("field '{key}' must be a non-empty string");
    };
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::Theme;
    use gpui::rgba;

    #[test]
    fn deserializes_legacy_block_focused_bg_key() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let legacy_json = default_json.replace("source_mode_block_bg", "block_focused_bg");

        let theme = Theme::from_json(&legacy_json).expect("legacy theme should deserialize");
        assert!(theme.colors.source_mode_block_bg.a > 0.0);
    }

    #[test]
    fn border_h2_falls_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();
        object
            .get_mut("colors")
            .and_then(|colors| colors.as_object_mut())
            .expect("theme should include colors")
            .remove("border_h2");
        let json = serde_json::to_string(&object).expect("theme json should serialize");

        let theme = Theme::from_json(&json).expect("theme without border_h2 should deserialize");
        assert_eq!(theme.colors.border_h2, rgba(0xe0e0e0cc).into());
    }

    #[test]
    fn comment_background_falls_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();
        object
            .get_mut("colors")
            .and_then(|colors| colors.as_object_mut())
            .expect("theme should include colors")
            .remove("comment_bg");
        let json = serde_json::to_string(&object).expect("theme json should serialize");

        let theme = Theme::from_json(&json).expect("theme without comment_bg should deserialize");
        assert_eq!(theme.colors.comment_bg, rgba(0xfbbf2426).into());
    }

    #[test]
    fn default_theme_json_omits_dialog_badge_and_strings_tokens() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");

        assert!(parsed.get("strings").is_none());

        let colors = parsed
            .get("colors")
            .and_then(|colors| colors.as_object())
            .expect("theme should include colors");
        assert!(!colors.contains_key(&format!("dialog_{}", "badge_bg")));
        assert!(!colors.contains_key(&format!("dialog_{}", "badge_text")));

        let dimensions = parsed
            .get("dimensions")
            .and_then(|dimensions| dimensions.as_object())
            .expect("theme should include dimensions");
        assert!(!dimensions.contains_key(&format!("dialog_{}", "badge_padding_x")));
        assert!(!dimensions.contains_key(&format!("dialog_{}", "badge_padding_y")));
    }

    #[test]
    fn legacy_theme_json_with_strings_still_loads() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();
        object.insert(
            "strings".into(),
            serde_json::json!({
                "menu_file": "Legacy File",
                "menu_language": "Legacy Language"
            }),
        );
        let json = serde_json::to_string(&object).expect("theme json should serialize");

        Theme::from_json(&json).expect("legacy theme strings should be ignored safely");
    }

    #[test]
    fn callout_dimensions_fall_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();
        let dimensions = object
            .get_mut("dimensions")
            .and_then(|dimensions| dimensions.as_object_mut())
            .expect("theme should include dimensions");
        dimensions.remove("callout_padding_x");
        dimensions.remove("callout_padding_y");
        dimensions.remove("callout_body_gap");
        dimensions.remove("callout_radius");
        dimensions.remove("callout_border_width");
        dimensions.remove("callout_header_gap");
        dimensions.remove("callout_header_margin_bottom");
        let json = serde_json::to_string(&object).expect("theme json should serialize");

        let theme = Theme::from_json(&json).expect("theme without callout dimensions should load");
        assert_eq!(theme.dimensions.callout_padding_x, 8.0);
        assert_eq!(theme.dimensions.callout_padding_y, 10.0);
        assert_eq!(theme.dimensions.callout_body_gap, 8.0);
        assert_eq!(theme.dimensions.callout_radius, 6.0);
        assert_eq!(theme.dimensions.callout_border_width, 3.0);
        assert_eq!(theme.dimensions.callout_header_gap, 6.0);
        assert_eq!(theme.dimensions.callout_header_margin_bottom, 6.0);
    }

    #[test]
    fn footnote_tokens_fall_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();

        let colors = object
            .get_mut("colors")
            .and_then(|colors| colors.as_object_mut())
            .expect("theme should include colors");
        colors.remove("footnote_border");
        colors.remove("footnote_backref");

        let json = serde_json::to_string(&object).expect("theme json should serialize");
        let theme = Theme::from_json(&json).expect("theme without footnote tokens should load");

        assert_eq!(theme.colors.footnote_border, rgba(0x71717a52).into());
        assert_eq!(theme.colors.footnote_backref, rgba(0xa1a1aaff).into());
    }

    #[test]
    fn code_language_palette_tokens_fall_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();

        let colors = object
            .get_mut("colors")
            .and_then(|colors| colors.as_object_mut())
            .expect("theme should include colors");
        colors.remove("code_bg");
        colors.remove("code_language_input_bg");
        colors.remove("code_language_input_border");
        colors.remove("code_language_input_text");
        colors.remove("code_language_input_placeholder");

        let json = serde_json::to_string(&object).expect("theme json should serialize");
        let theme =
            Theme::from_json(&json).expect("theme without code language palette should load");

        assert_eq!(theme.colors.code_bg, rgba(0x111827ff).into());
        assert_eq!(theme.colors.code_language_input_bg, rgba(0x343941ff).into());
        assert_eq!(
            theme.colors.code_language_input_border,
            rgba(0x4b5563cc).into()
        );
        assert_eq!(
            theme.colors.code_language_input_text,
            rgba(0xe5e7ebff).into()
        );
        assert_eq!(
            theme.colors.code_language_input_placeholder,
            rgba(0x9ca3afcc).into()
        );
    }

    #[test]
    fn important_callout_defaults_use_purple_palette() {
        let theme = Theme::default_theme();
        assert_eq!(theme.colors.callout_important_bg, rgba(0xa78bfa1f).into());
        assert_eq!(
            theme.colors.callout_important_border,
            rgba(0xa78bfaff).into()
        );
        assert_eq!(theme.dimensions.block_gap, 6.0);
        assert_eq!(theme.colors.code_bg, rgba(0x23272eff).into());
        assert_eq!(theme.colors.code_language_input_bg, rgba(0x343941ff).into());
        assert_eq!(
            theme.colors.code_language_input_border,
            rgba(0x4b5563cc).into()
        );
    }

    #[test]
    fn light_theme_uses_light_palette_without_changing_layout_tokens() {
        let dark = Theme::default_theme();
        let light = Theme::light_theme();

        assert_eq!(light.name, "Light");
        assert_eq!(light.colors.editor_background, rgba(0xffffffff).into());
        assert_eq!(light.colors.text_default, rgba(0x1f2937ff).into());
        assert_eq!(light.colors.text_link, rgba(0x2563ebff).into());
        assert_eq!(light.colors.code_bg, rgba(0xf1f5f9ff).into());
        assert_eq!(
            light.colors.code_language_input_border,
            rgba(0xcbd5e1ff).into()
        );
        assert_eq!(
            light.colors.table_cell_active_outline,
            rgba(0x2563ebff).into()
        );
        assert_eq!(light.dimensions.block_gap, dark.dimensions.block_gap);
        assert_eq!(light.typography.text_size, dark.typography.text_size);
    }

    #[test]
    fn menu_dimension_tokens_fall_back_when_omitted() {
        let default_json = Theme::default_theme()
            .to_json()
            .expect("default theme should serialize");
        let parsed: serde_json::Value =
            serde_json::from_str(&default_json).expect("default theme json should parse");
        let mut object = parsed
            .as_object()
            .expect("theme should serialize to a json object")
            .clone();

        let dimensions = object
            .get_mut("dimensions")
            .and_then(|dimensions| dimensions.as_object_mut())
            .expect("theme should include dimensions");
        dimensions.remove("menu_bar_height");
        dimensions.remove("menu_item_height");
        dimensions.remove("context_menu_panel_width");
        dimensions.remove("table_insert_dialog_width");

        let json = serde_json::to_string(&object).expect("theme json should serialize");
        let theme = Theme::from_json(&json).expect("theme without menu tokens should load");

        assert_eq!(theme.dimensions.menu_bar_height, 32.0);
        assert_eq!(theme.dimensions.menu_item_height, 28.0);
        assert_eq!(theme.dimensions.context_menu_panel_width, 132.0);
        assert_eq!(theme.dimensions.table_insert_dialog_width, 380.0);
    }

    #[test]
    fn custom_theme_pack_can_inherit_light_base() {
        let value = serde_json::json!({
            "name": "Day Writer",
            "creator": "Ada",
            "base_theme_id": "splitype-light",
            "theme": {
                "dimensions": {
                    "menu_panel_radius": 12.0
                },
                "colors": {
                    "text_link": null
                }
            }
        });

        let (entry, normalized) =
            super::custom_theme_from_value(value).expect("theme should import");
        let light = Theme::light_theme();

        assert_eq!(entry.base_theme_id, "splitype-light");
        assert_eq!(
            entry.theme.colors.editor_background,
            light.colors.editor_background
        );
        assert_eq!(entry.theme.colors.text_default, light.colors.text_default);
        assert_eq!(entry.theme.colors.text_link, light.colors.text_link);
        assert_eq!(entry.theme.dimensions.menu_panel_radius, 12.0);
        assert_eq!(
            normalized
                .get("base_theme_id")
                .and_then(|value| value.as_str()),
            Some("splitype-light")
        );
        assert!(
            normalized
                .pointer("/theme/colors")
                .and_then(|value| value.as_object())
                .map(|colors| !colors.contains_key("text_link"))
                .unwrap_or(true)
        );
    }

    #[test]
    fn invalid_custom_theme_base_falls_back_to_dark() {
        let value = serde_json::json!({
            "name": "Broken Base",
            "creator": "Ada",
            "base_theme_id": "missing",
            "theme": {
                "dimensions": {
                    "block_gap": 10.0
                }
            }
        });

        let (entry, normalized) =
            super::custom_theme_from_value(value).expect("invalid base should not fail import");

        assert_eq!(entry.base_theme_id, "splitype");
        assert_eq!(
            entry.theme.colors.editor_background,
            Theme::default_theme().colors.editor_background
        );
        assert_eq!(
            normalized
                .get("base_theme_id")
                .and_then(|value| value.as_str()),
            Some("splitype")
        );
    }
}
