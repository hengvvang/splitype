//! Spacing, sizing, and layout dimensions.

use serde::{Deserialize, Deserializer, Serialize};

/// All configurable dimensions (paddings, gaps, sizes) for the editor UI.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeDimensions {
    /// Padding around the editor content area.
    pub editor_padding: f32,
    /// Vertical gap between adjacent blocks.
    pub block_gap: f32,
    /// Minimum height of every block.
    pub block_min_height: f32,
    /// Vertical padding inside each block.
    pub block_padding_y: f32,
    /// Horizontal padding inside each block.
    pub block_padding_x: f32,
    /// Extra horizontal indent per nesting level (list items).
    pub nested_block_indent: f32,
    /// Gap between list marker and its text content.
    pub list_marker_gap: f32,
    /// Minimum width of the bullet list marker column.
    pub list_marker_width: f32,
    /// Minimum width of the ordered-list marker column.
    pub ordered_list_marker_width: f32,
    /// Width and height of the interactive task-list checkbox.
    pub task_checkbox_size: f32,
    /// Corner radius of the task-list checkbox.
    pub task_checkbox_radius: f32,
    /// Border width of the task-list checkbox.
    pub task_checkbox_border_width: f32,
    /// Checkmark font size inside the task-list checkbox.
    pub task_checkbox_check_size: f32,
    /// Extra padding below H1 text.
    pub h1_padding_bottom: f32,
    /// Margin below the H1 bottom border.
    pub h1_margin_bottom: f32,
    /// Width of the text-editing cursor (caret).
    pub cursor_width: f32,
    /// Thickness of the underline decoration.
    pub underline_thickness: f32,
    /// H1 bottom-border thickness.
    pub h1_border_width: f32,
    /// Quote block left-border thickness.
    pub quote_border_width: f32,
    /// Extra left padding between quote border and text.
    pub quote_padding_left: f32,
    /// Horizontal padding inside editor-level callout shells.
    pub callout_padding_x: f32,
    /// Vertical padding inside editor-level callout shells.
    pub callout_padding_y: f32,
    /// Vertical gap between callout body rows.
    pub callout_body_gap: f32,
    /// Corner radius of editor-level callout shells.
    pub callout_radius: f32,
    /// Accent border width of editor-level callout shells.
    pub callout_border_width: f32,
    /// Gap between callout icon and header text.
    pub callout_header_gap: f32,
    /// Vertical margin between the callout header row and the first body row.
    pub callout_header_margin_bottom: f32,
    /// Horizontal padding inside footnote grouping shells.
    pub footnote_padding_x: f32,
    /// Vertical padding inside footnote grouping shells.
    pub footnote_padding_y: f32,
    /// Corner radius of footnote grouping shells.
    pub footnote_radius: f32,
    /// Horizontal padding inside the footnote ordinal badge.
    pub footnote_badge_padding_x: f32,
    /// Vertical padding inside the footnote ordinal badge.
    pub footnote_badge_padding_y: f32,
    /// Thickness of the separator block line.
    pub separator_thickness: f32,
    /// Extra horizontal inset applied to separator blocks.
    pub separator_inset_x: f32,
    /// Vertical margin around separator blocks.
    pub separator_margin_y: f32,
    /// Vertical padding inside a code block.
    pub code_block_padding_y: f32,
    /// Horizontal padding inside a code block.
    pub code_block_padding_x: f32,
    /// Horizontal padding around inline code background quads.
    pub code_bg_pad_x: f32,
    /// Vertical padding around inline code background quads.
    pub code_bg_pad_y: f32,
    /// Corner radius for inline code background quads.
    pub code_bg_radius: f32,
    /// Width of the code-block language input.
    pub code_language_input_width: f32,
    /// Text layout height inside the code-block language input.
    pub code_language_input_height: f32,
    /// Horizontal padding inside the code-block language input.
    pub code_language_input_padding_x: f32,
    /// Vertical padding inside the code-block language input.
    pub code_language_input_padding_y: f32,
    /// Corner radius of the code-block language input.
    pub code_language_input_radius: f32,
    /// Border width of the code-block language input.
    pub code_language_input_border_width: f32,
    /// Gap between code text and the language input.
    pub code_language_input_gap: f32,
    /// Horizontal padding inside native table cells.
    pub table_cell_padding_x: f32,
    /// Vertical padding inside native table cells.
    pub table_cell_padding_y: f32,
    /// Minimum height of native table cells.
    pub table_cell_min_height: f32,
    /// Width of the append-column control and height of the append-row control.
    pub table_append_button_extent: f32,
    /// Inset padding around rendered-mode native table append controls.
    pub table_append_button_inset: f32,
    /// Invisible activation overlap that keeps append controls easy to hover.
    pub table_append_activation_band: f32,
    /// Outer corner radius of the table container.
    pub table_border_radius: f32,
    /// Width of the table handle pill.
    pub table_handle_width: f32,
    /// Height of the table handle pill.
    pub table_handle_height: f32,
    /// Border thickness of the selection frame around selected columns/rows.
    pub table_selection_border_width: f32,
    /// Corner radius of rendered images and image placeholders.
    pub image_radius: f32,
    /// Maximum height of rendered root-paragraph images.
    pub image_root_max_height: f32,
    /// Maximum height of rendered table-cell images.
    pub image_cell_max_height: f32,
    /// Default placeholder height for rendered root-paragraph images.
    pub image_root_placeholder_height: f32,
    /// Default placeholder height for rendered table-cell images.
    pub image_cell_placeholder_height: f32,
    /// Vertical gap between a rendered image and its caption.
    pub image_caption_gap: f32,
    /// Width of the custom scrollbar thumb.
    pub scrollbar_width: f32,
    /// Distance of the scrollbar thumb from the right edge.
    pub scrollbar_right: f32,
    /// Viewport width at which the content column starts shrinking.
    pub centered_shrink_start: f32,
    /// Viewport width at which the content column reaches minimum ratio.
    pub centered_shrink_end: f32,
    /// Minimum content-column width as a fraction of available width.
    pub centered_min_ratio: f32,
    /// Width of the unsaved-changes dialog.
    pub dialog_width: f32,
    /// Padding inside the unsaved-changes dialog.
    pub dialog_padding: f32,
    /// Gap between dialog sections.
    pub dialog_gap: f32,
    /// Corner radius of the unsaved-changes dialog.
    pub dialog_radius: f32,
    /// Border width of the unsaved-changes dialog.
    pub dialog_border_width: f32,
    /// Height of dialog action buttons.
    pub dialog_button_height: f32,
    /// Gap between dialog action buttons.
    pub dialog_button_gap: f32,
    /// Horizontal padding inside dialog action buttons.
    pub dialog_button_padding_x: f32,
    /// Height reserved for the in-window fallback menu bar.
    pub menu_bar_height: f32,
    /// Horizontal padding inside the in-window fallback menu bar.
    pub menu_bar_padding_x: f32,
    /// Vertical padding inside the in-window fallback menu bar.
    pub menu_bar_padding_y: f32,
    /// Gap between top-level menu buttons.
    pub menu_bar_gap: f32,
    /// Minimum width of each top-level menu button.
    pub menu_bar_button_width: f32,
    /// Height of each top-level menu button.
    pub menu_bar_button_height: f32,
    /// Horizontal padding inside top-level menu buttons.
    pub menu_bar_button_padding_x: f32,
    /// Corner radius of top-level menu buttons.
    pub menu_bar_button_radius: f32,
    /// Text size used by menu labels.
    pub menu_text_size: f32,
    /// Top position of the in-window fallback floating menu panel.
    pub menu_panel_top: f32,
    /// Width of the in-window fallback floating menu panel.
    pub menu_panel_width: f32,
    /// Padding inside floating menu panels.
    pub menu_panel_padding: f32,
    /// Gap between items inside floating menu panels.
    pub menu_panel_gap: f32,
    /// Corner radius of floating menu panels.
    pub menu_panel_radius: f32,
    /// Height of each floating menu item.
    pub menu_item_height: f32,
    /// Horizontal padding inside floating menu items.
    pub menu_item_padding_x: f32,
    /// Corner radius of floating menu items.
    pub menu_item_radius: f32,
    /// Horizontal margin around menu separators.
    pub menu_separator_margin_x: f32,
    /// Vertical margin around menu separators.
    pub menu_separator_margin_y: f32,
    /// Height of menu separators.
    pub menu_separator_height: f32,
    /// Width of the root insert context menu panel.
    pub context_menu_panel_width: f32,
    /// Width of the insert-submenu panel.
    pub context_menu_submenu_width: f32,
    /// Horizontal gap between a context menu and its submenu.
    pub context_menu_submenu_gap: f32,
    /// Width of the table-axis context menu panel.
    pub context_menu_axis_panel_width: f32,
    /// Maximum width of the table-insert dialog.
    pub table_insert_dialog_width: f32,
    /// Gap between table-insert stepper label and controls.
    pub table_insert_stepper_gap: f32,
    /// Size of table-insert stepper buttons.
    pub table_insert_stepper_button_size: f32,
    /// Minimum width of the table-insert stepper value pill.
    pub table_insert_stepper_value_min_width: f32,
    /// Horizontal padding inside the table-insert stepper value pill.
    pub table_insert_stepper_value_padding_x: f32,
    /// Corner radius of table-insert stepper controls.
    pub table_insert_stepper_radius: f32,
    /// Left inset of the view-mode toggle.
    pub view_mode_toggle_left: f32,
    /// Bottom inset of the view-mode toggle.
    pub view_mode_toggle_bottom: f32,
    /// Horizontal padding inside the view-mode toggle.
    pub view_mode_toggle_padding_x: f32,
    /// Vertical padding inside the view-mode toggle.
    pub view_mode_toggle_padding_y: f32,
    /// Minimum width of the view-mode toggle.
    pub view_mode_toggle_min_width: f32,
    /// Corner radius of the view-mode toggle.
    pub view_mode_toggle_radius: f32,
    /// Border width of the view-mode toggle.
    pub view_mode_toggle_border_width: f32,
    /// Text size of the view-mode toggle.
    pub view_mode_toggle_text_size: f32,
    /// Height of the status bar.
    pub status_bar_height: f32,
    /// Horizontal padding inside the status bar.
    pub status_bar_padding_x: f32,
    /// Gap between items in the status bar.
    pub status_bar_item_gap: f32,
    /// Font size for status bar text.
    pub status_bar_text_size: f32,
    pub area_tile_gap: f32,
    pub area_tile_radius: f32,
}

/// Deserialization adapter for `ThemeDimensions` with backward-compatible defaults.
#[derive(Deserialize)]
struct ThemeDimensionsDe {
    editor_padding: f32,
    block_gap: f32,
    block_min_height: f32,
    block_padding_y: f32,
    block_padding_x: f32,
    nested_block_indent: f32,
    list_marker_gap: f32,
    list_marker_width: f32,
    ordered_list_marker_width: f32,
    task_checkbox_size: Option<f32>,
    task_checkbox_radius: Option<f32>,
    task_checkbox_border_width: Option<f32>,
    task_checkbox_check_size: Option<f32>,
    h1_padding_bottom: f32,
    h1_margin_bottom: f32,
    cursor_width: f32,
    underline_thickness: f32,
    h1_border_width: f32,
    quote_border_width: f32,
    quote_padding_left: f32,
    callout_padding_x: Option<f32>,
    callout_padding_y: Option<f32>,
    callout_body_gap: Option<f32>,
    callout_radius: Option<f32>,
    callout_border_width: Option<f32>,
    callout_header_gap: Option<f32>,
    callout_header_margin_bottom: Option<f32>,
    footnote_padding_x: Option<f32>,
    footnote_padding_y: Option<f32>,
    footnote_radius: Option<f32>,
    footnote_badge_padding_x: Option<f32>,
    footnote_badge_padding_y: Option<f32>,
    separator_thickness: Option<f32>,
    separator_inset_x: Option<f32>,
    separator_margin_y: Option<f32>,
    code_block_padding_y: f32,
    code_block_padding_x: f32,
    code_bg_pad_x: f32,
    code_bg_pad_y: f32,
    code_bg_radius: f32,
    code_language_input_width: Option<f32>,
    code_language_input_height: Option<f32>,
    code_language_input_padding_x: Option<f32>,
    code_language_input_padding_y: Option<f32>,
    code_language_input_radius: Option<f32>,
    code_language_input_border_width: Option<f32>,
    code_language_input_gap: Option<f32>,
    table_cell_padding_x: Option<f32>,
    table_cell_padding_y: Option<f32>,
    table_cell_min_height: Option<f32>,
    table_append_button_extent: Option<f32>,
    table_append_button_inset: Option<f32>,
    table_append_activation_band: Option<f32>,
    table_border_radius: Option<f32>,
    table_handle_width: Option<f32>,
    table_handle_height: Option<f32>,
    table_selection_border_width: Option<f32>,
    image_radius: Option<f32>,
    image_root_max_height: Option<f32>,
    image_cell_max_height: Option<f32>,
    image_root_placeholder_height: Option<f32>,
    image_cell_placeholder_height: Option<f32>,
    image_caption_gap: Option<f32>,
    scrollbar_width: f32,
    scrollbar_right: f32,
    centered_shrink_start: f32,
    centered_shrink_end: f32,
    centered_min_ratio: f32,
    dialog_width: f32,
    dialog_padding: f32,
    dialog_gap: f32,
    dialog_radius: f32,
    dialog_border_width: f32,
    dialog_button_height: f32,
    dialog_button_gap: f32,
    dialog_button_padding_x: f32,
    menu_bar_height: Option<f32>,
    menu_bar_padding_x: Option<f32>,
    menu_bar_padding_y: Option<f32>,
    menu_bar_gap: Option<f32>,
    menu_bar_button_width: Option<f32>,
    menu_bar_button_height: Option<f32>,
    menu_bar_button_padding_x: Option<f32>,
    menu_bar_button_radius: Option<f32>,
    menu_text_size: Option<f32>,
    menu_panel_top: Option<f32>,
    menu_panel_width: Option<f32>,
    menu_panel_padding: Option<f32>,
    menu_panel_gap: Option<f32>,
    menu_panel_radius: Option<f32>,
    menu_item_height: Option<f32>,
    menu_item_padding_x: Option<f32>,
    menu_item_radius: Option<f32>,
    menu_separator_margin_x: Option<f32>,
    menu_separator_margin_y: Option<f32>,
    menu_separator_height: Option<f32>,
    context_menu_panel_width: Option<f32>,
    context_menu_submenu_width: Option<f32>,
    context_menu_submenu_gap: Option<f32>,
    context_menu_axis_panel_width: Option<f32>,
    table_insert_dialog_width: Option<f32>,
    table_insert_stepper_gap: Option<f32>,
    table_insert_stepper_button_size: Option<f32>,
    table_insert_stepper_value_min_width: Option<f32>,
    table_insert_stepper_value_padding_x: Option<f32>,
    table_insert_stepper_radius: Option<f32>,
    view_mode_toggle_left: Option<f32>,
    view_mode_toggle_bottom: Option<f32>,
    view_mode_toggle_padding_x: Option<f32>,
    view_mode_toggle_padding_y: Option<f32>,
    view_mode_toggle_min_width: Option<f32>,
    view_mode_toggle_radius: Option<f32>,
    view_mode_toggle_border_width: Option<f32>,
    view_mode_toggle_text_size: Option<f32>,
    status_bar_height: Option<f32>,
    status_bar_padding_x: Option<f32>,
    status_bar_item_gap: Option<f32>,
    status_bar_text_size: Option<f32>,
    area_tile_gap: Option<f32>,
    area_tile_radius: Option<f32>,
}

impl<'de> Deserialize<'de> for ThemeDimensions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = ThemeDimensionsDe::deserialize(deserializer)?;
        Ok(Self {
            editor_padding: raw.editor_padding,
            block_gap: raw.block_gap,
            block_min_height: raw.block_min_height,
            block_padding_y: raw.block_padding_y,
            block_padding_x: raw.block_padding_x,
            nested_block_indent: raw.nested_block_indent,
            list_marker_gap: raw.list_marker_gap,
            list_marker_width: raw.list_marker_width,
            ordered_list_marker_width: raw.ordered_list_marker_width,
            task_checkbox_size: raw.task_checkbox_size.unwrap_or(16.0),
            task_checkbox_radius: raw.task_checkbox_radius.unwrap_or(2.0),
            task_checkbox_border_width: raw.task_checkbox_border_width.unwrap_or(1.5),
            task_checkbox_check_size: raw.task_checkbox_check_size.unwrap_or(14.5),
            h1_padding_bottom: raw.h1_padding_bottom,
            h1_margin_bottom: raw.h1_margin_bottom,
            cursor_width: raw.cursor_width,
            underline_thickness: raw.underline_thickness,
            h1_border_width: raw.h1_border_width,
            quote_border_width: raw.quote_border_width,
            quote_padding_left: raw.quote_padding_left,
            callout_padding_x: raw.callout_padding_x.unwrap_or(8.0),
            callout_padding_y: raw.callout_padding_y.unwrap_or(10.0),
            callout_body_gap: raw.callout_body_gap.unwrap_or(8.0),
            callout_radius: raw.callout_radius.unwrap_or(6.0),
            callout_border_width: raw.callout_border_width.unwrap_or(3.0),
            callout_header_gap: raw.callout_header_gap.unwrap_or(6.0),
            callout_header_margin_bottom: raw.callout_header_margin_bottom.unwrap_or(6.0),
            footnote_padding_x: raw.footnote_padding_x.unwrap_or(10.0),
            footnote_padding_y: raw.footnote_padding_y.unwrap_or(6.0),
            footnote_radius: raw.footnote_radius.unwrap_or(6.0),
            footnote_badge_padding_x: raw.footnote_badge_padding_x.unwrap_or(4.0),
            footnote_badge_padding_y: raw.footnote_badge_padding_y.unwrap_or(1.0),
            separator_thickness: raw.separator_thickness.unwrap_or(1.0),
            separator_inset_x: raw.separator_inset_x.unwrap_or(40.0),
            separator_margin_y: raw.separator_margin_y.unwrap_or(10.0),
            code_block_padding_y: raw.code_block_padding_y,
            code_block_padding_x: raw.code_block_padding_x,
            code_bg_pad_x: raw.code_bg_pad_x,
            code_bg_pad_y: raw.code_bg_pad_y,
            code_bg_radius: raw.code_bg_radius,
            code_language_input_width: raw.code_language_input_width.unwrap_or(156.0),
            code_language_input_height: raw.code_language_input_height.unwrap_or(18.0),
            code_language_input_padding_x: raw.code_language_input_padding_x.unwrap_or(8.0),
            code_language_input_padding_y: raw.code_language_input_padding_y.unwrap_or(3.0),
            code_language_input_radius: raw.code_language_input_radius.unwrap_or(6.0),
            code_language_input_border_width: raw.code_language_input_border_width.unwrap_or(1.0),
            code_language_input_gap: raw.code_language_input_gap.unwrap_or(8.0),
            table_cell_padding_x: raw.table_cell_padding_x.unwrap_or(10.0),
            table_cell_padding_y: raw.table_cell_padding_y.unwrap_or(8.0),
            table_cell_min_height: raw.table_cell_min_height.unwrap_or(42.0),
            table_append_button_extent: raw.table_append_button_extent.unwrap_or(16.0),
            table_append_button_inset: raw.table_append_button_inset.unwrap_or(8.0),
            table_append_activation_band: raw.table_append_activation_band.unwrap_or(18.0),
            table_border_radius: raw.table_border_radius.unwrap_or(4.0),
            table_handle_width: raw.table_handle_width.unwrap_or(10.0),
            table_handle_height: raw.table_handle_height.unwrap_or(36.0),
            table_selection_border_width: raw.table_selection_border_width.unwrap_or(2.0),
            image_radius: raw.image_radius.unwrap_or(12.0),
            image_root_max_height: raw.image_root_max_height.unwrap_or(420.0),
            image_cell_max_height: raw.image_cell_max_height.unwrap_or(180.0),
            image_root_placeholder_height: raw.image_root_placeholder_height.unwrap_or(260.0),
            image_cell_placeholder_height: raw.image_cell_placeholder_height.unwrap_or(120.0),
            image_caption_gap: raw.image_caption_gap.unwrap_or(8.0),
            scrollbar_width: raw.scrollbar_width,
            scrollbar_right: raw.scrollbar_right,
            centered_shrink_start: raw.centered_shrink_start,
            centered_shrink_end: raw.centered_shrink_end,
            centered_min_ratio: raw.centered_min_ratio,
            dialog_width: raw.dialog_width,
            dialog_padding: raw.dialog_padding,
            dialog_gap: raw.dialog_gap,
            dialog_radius: raw.dialog_radius,
            dialog_border_width: raw.dialog_border_width,
            dialog_button_height: raw.dialog_button_height,
            dialog_button_gap: raw.dialog_button_gap,
            dialog_button_padding_x: raw.dialog_button_padding_x,
            menu_bar_height: raw.menu_bar_height.unwrap_or(32.0),
            menu_bar_padding_x: raw.menu_bar_padding_x.unwrap_or(10.0),
            menu_bar_padding_y: raw.menu_bar_padding_y.unwrap_or(4.0),
            menu_bar_gap: raw.menu_bar_gap.unwrap_or(2.0),
            menu_bar_button_width: raw.menu_bar_button_width.unwrap_or(48.0),
            menu_bar_button_height: raw.menu_bar_button_height.unwrap_or(24.0),
            menu_bar_button_padding_x: raw.menu_bar_button_padding_x.unwrap_or(8.0),
            menu_bar_button_radius: raw.menu_bar_button_radius.unwrap_or(3.0),
            menu_text_size: raw.menu_text_size.unwrap_or(12.0),
            menu_panel_top: raw.menu_panel_top.unwrap_or(2.0),
            menu_panel_width: raw.menu_panel_width.unwrap_or(180.0),
            menu_panel_padding: raw.menu_panel_padding.unwrap_or(4.0),
            menu_panel_gap: raw.menu_panel_gap.unwrap_or(1.0),
            menu_panel_radius: raw.menu_panel_radius.unwrap_or(3.0),
            menu_item_height: raw.menu_item_height.unwrap_or(28.0),
            menu_item_padding_x: raw.menu_item_padding_x.unwrap_or(8.0),
            menu_item_radius: raw.menu_item_radius.unwrap_or(3.0),
            menu_separator_margin_x: raw.menu_separator_margin_x.unwrap_or(6.0),
            menu_separator_margin_y: raw.menu_separator_margin_y.unwrap_or(3.0),
            menu_separator_height: raw.menu_separator_height.unwrap_or(1.0),
            context_menu_panel_width: raw.context_menu_panel_width.unwrap_or(132.0),
            context_menu_submenu_width: raw.context_menu_submenu_width.unwrap_or(148.0),
            context_menu_submenu_gap: raw.context_menu_submenu_gap.unwrap_or(2.0),
            context_menu_axis_panel_width: raw.context_menu_axis_panel_width.unwrap_or(164.0),
            table_insert_dialog_width: raw.table_insert_dialog_width.unwrap_or(380.0),
            table_insert_stepper_gap: raw.table_insert_stepper_gap.unwrap_or(8.0),
            table_insert_stepper_button_size: raw.table_insert_stepper_button_size.unwrap_or(32.0),
            table_insert_stepper_value_min_width: raw
                .table_insert_stepper_value_min_width
                .unwrap_or(56.0),
            table_insert_stepper_value_padding_x: raw
                .table_insert_stepper_value_padding_x
                .unwrap_or(10.0),
            table_insert_stepper_radius: raw.table_insert_stepper_radius.unwrap_or(8.0),
            view_mode_toggle_left: raw.view_mode_toggle_left.unwrap_or(12.0),
            view_mode_toggle_bottom: raw.view_mode_toggle_bottom.unwrap_or(12.0),
            view_mode_toggle_padding_x: raw.view_mode_toggle_padding_x.unwrap_or(8.0),
            view_mode_toggle_padding_y: raw.view_mode_toggle_padding_y.unwrap_or(4.0),
            view_mode_toggle_min_width: raw.view_mode_toggle_min_width.unwrap_or(88.0),
            view_mode_toggle_radius: raw.view_mode_toggle_radius.unwrap_or(999.0),
            view_mode_toggle_border_width: raw.view_mode_toggle_border_width.unwrap_or(1.0),
            view_mode_toggle_text_size: raw.view_mode_toggle_text_size.unwrap_or(11.0),
            status_bar_height: raw.status_bar_height.unwrap_or(28.0),
            status_bar_padding_x: raw.status_bar_padding_x.unwrap_or(12.0),
            status_bar_item_gap: raw.status_bar_item_gap.unwrap_or(12.0),
            status_bar_text_size: raw.status_bar_text_size.unwrap_or(11.0),
            area_tile_gap: raw.area_tile_gap.unwrap_or(6.0),
            area_tile_radius: raw.area_tile_radius.unwrap_or(3.0),
        })
    }
}
