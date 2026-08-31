//! Virtualized document viewport planning, row windowing, and container assembly
//! for the WYSIWYG editor mode.

use gpui::*;

use crate::markdown::block::CalloutKind;
use crate::model::BlockEntry;
use crate::render::layout::{
    RowSpacingInfo, callout_colors, callout_row_top_gap, footnote_row_top_gap, row_top_gap,
};
use theme::{Theme, ThemeDimensions};

#[derive(Clone, Debug)]
pub enum PlannedInnerSegment {
    /// A single block row with its leading `mt` gap.
    Block { gap: f32 },
    /// A footnote subgroup: an outer gap plus per-block row gaps.
    FootnoteSubgroup { gap: f32, row_gaps: Vec<f32> },
}

/// Lightweight plan for one render row, built for every row but materialized
/// into elements only for the windowed range.
#[derive(Clone, Debug)]
pub struct PlannedRow {
    /// Block range covered by this row, `[start, end)` in visible order.
    pub start: usize,
    pub end: usize,
    /// Callout accent variant when this row is a callout group.
    pub callout_variant: Option<CalloutKind>,
    /// The outer container's leading `mt` gap.
    pub outer_gap: f32,
    /// Inner rows in order; the sum of their block counts equals `end - start`.
    pub segments: Vec<PlannedInnerSegment>,
}

/// Plans all virtualized rows for the given block entries.
pub fn plan_document_rows(blocks: &[BlockEntry], d: &ThemeDimensions, cx: &App) -> Vec<PlannedRow> {
    let spacing_for = |index: usize| -> RowSpacingInfo {
        blocks[index]
            .entity
            .read_with(cx, |block, _cx| RowSpacingInfo::from_block(block))
    };

    let mut previous_row_spacing = None;
    let mut rows: Vec<PlannedRow> = Vec::new();
    let mut index = 0usize;

    while index < blocks.len() {
        let first_spacing = spacing_for(index);
        let top_gap = row_top_gap(previous_row_spacing, first_spacing, d.block_gap);

        if let (Some(callout_group_id), Some(callout_variant)) = (
            first_spacing.callout_group_id,
            first_spacing.callout_variant,
        ) {
            let mut segments = Vec::new();
            let mut group_end = index;
            let mut previous_callout_row = None;
            while group_end < blocks.len()
                && spacing_for(group_end).callout_group_id == Some(callout_group_id)
            {
                let row_spacing = spacing_for(group_end);
                if let Some(footnote_group_id) = row_spacing.footnote_group_id {
                    let mut footnote_end = group_end;
                    let mut previous_footnote_row = None;
                    let mut row_gaps = Vec::new();
                    while footnote_end < blocks.len()
                        && spacing_for(footnote_end).callout_group_id == Some(callout_group_id)
                        && spacing_for(footnote_end).footnote_group_id == Some(footnote_group_id)
                    {
                        let footnote_spacing = spacing_for(footnote_end);
                        row_gaps.push(footnote_row_top_gap(previous_footnote_row, d.block_gap));
                        previous_footnote_row = Some(footnote_spacing);
                        footnote_end += 1;
                    }

                    segments.push(PlannedInnerSegment::FootnoteSubgroup {
                        gap: callout_row_top_gap(previous_callout_row, row_spacing, d),
                        row_gaps,
                    });
                    previous_callout_row = Some(spacing_for(footnote_end - 1));
                    group_end = footnote_end;
                    continue;
                }

                segments.push(PlannedInnerSegment::Block {
                    gap: callout_row_top_gap(previous_callout_row, row_spacing, d),
                });
                previous_callout_row = Some(row_spacing);
                group_end += 1;
            }

            rows.push(PlannedRow {
                start: index,
                end: group_end,
                callout_variant: Some(callout_variant),
                outer_gap: top_gap,
                segments,
            });
            previous_row_spacing = Some(spacing_for(group_end - 1));
            index = group_end;
            continue;
        }

        if let Some(footnote_group_id) = first_spacing.footnote_group_id {
            let mut segments = Vec::new();
            let mut group_end = index;
            let mut previous_footnote_row = None;
            while group_end < blocks.len()
                && spacing_for(group_end).footnote_group_id == Some(footnote_group_id)
            {
                let row_spacing = spacing_for(group_end);
                segments.push(PlannedInnerSegment::Block {
                    gap: footnote_row_top_gap(previous_footnote_row, d.block_gap),
                });
                previous_footnote_row = Some(row_spacing);
                group_end += 1;
            }

            rows.push(PlannedRow {
                start: index,
                end: group_end,
                callout_variant: None,
                outer_gap: top_gap,
                segments,
            });
            previous_row_spacing = Some(spacing_for(group_end - 1));
            index = group_end;
            continue;
        }

        rows.push(PlannedRow {
            start: index,
            end: index + 1,
            callout_variant: None,
            outer_gap: top_gap,
            segments: Vec::new(),
        });
        previous_row_spacing = Some(first_spacing);
        index += 1;
    }

    rows
}

/// Materializes one planned render row into its element tree.
pub fn build_planned_row_element<F>(
    plan: &PlannedRow,
    blocks: &[BlockEntry],
    centered_width: f32,
    theme: &Theme,
    d: &ThemeDimensions,
    mut attach_context_menu: F,
) -> AnyElement
where
    F: FnMut(Div, EntityId) -> Div,
{
    match (plan.callout_variant, plan.segments.is_empty()) {
        // Single block row.
        (None, true) => {
            let entity = blocks[plan.start].entity.clone();
            let entity_id = entity.entity_id();
            let row = div()
                .w(px(centered_width))
                .max_w(relative(1.0))
                .flex_shrink_0()
                .mt(px(plan.outer_gap))
                .child(entity);
            attach_context_menu(row, entity_id).into_any_element()
        }
        // Plain footnote group.
        (None, false) => {
            let mut children = Vec::new();
            let mut block_offset = plan.start;
            for segment in &plan.segments {
                let PlannedInnerSegment::Block { gap } = segment else {
                    continue;
                };
                let entity = blocks[block_offset].entity.clone();
                let entity_id = entity.entity_id();
                let row = div().w_full().flex_shrink_0().mt(px(*gap)).child(entity);
                children.push(attach_context_menu(row, entity_id).into_any_element());
                block_offset += 1;
            }
            div()
                .w(px(centered_width))
                .max_w(relative(1.0))
                .flex_shrink_0()
                .mt(px(plan.outer_gap))
                .children(children)
                .into_any_element()
        }
        // Callout group (possibly with footnote subgroups inside).
        (Some(variant), _) => {
            let (accent, _background) = callout_colors(variant, theme);
            let mut group_children = Vec::new();
            let mut block_offset = plan.start;
            for segment in &plan.segments {
                match segment {
                    PlannedInnerSegment::Block { gap } => {
                        let entity = blocks[block_offset].entity.clone();
                        let entity_id = entity.entity_id();
                        let row = div().w_full().flex_shrink_0().mt(px(*gap)).child(entity);
                        group_children.push(attach_context_menu(row, entity_id).into_any_element());
                        block_offset += 1;
                    }
                    PlannedInnerSegment::FootnoteSubgroup { gap, row_gaps } => {
                        let mut footnote_children = Vec::new();
                        for row_gap in row_gaps {
                            let entity = blocks[block_offset].entity.clone();
                            let entity_id = entity.entity_id();
                            let row = div()
                                .w_full()
                                .flex_shrink_0()
                                .mt(px(*row_gap))
                                .child(entity);
                            footnote_children
                                .push(attach_context_menu(row, entity_id).into_any_element());
                            block_offset += 1;
                        }
                        group_children.push(
                            div()
                                .w_full()
                                .flex_shrink_0()
                                .mt(px(*gap))
                                .children(footnote_children)
                                .into_any_element(),
                        );
                    }
                }
            }
            div()
                .w(px(centered_width))
                .max_w(relative(1.0))
                .flex_shrink_0()
                .mt(px(plan.outer_gap))
                .relative()
                .pl(px(d.quote_padding_left))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(0.0))
                        .children(group_children),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(d.block_padding_x))
                        .w(px(d.callout_border_width))
                        .bg(accent),
                )
                .into_any_element()
        }
    }
}
