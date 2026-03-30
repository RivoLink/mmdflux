//! Sequence diagram text renderer.
//!
//! Renders a `SequenceLayout` onto a shared `Canvas` using box-drawing
//! characters from `CharSet`. Supports both Unicode and ASCII output.

use crate::render::text::canvas::Canvas;
use crate::render::text::chars::CharSet;
use crate::timeline::sequence::layout::{
    ActivationRect, ParticipantLayout, RowLayout, SELF_MSG_WIDTH, SequenceLayout,
};
use crate::timeline::sequence::model::{ArrowHead, LineStyle, NotePlacement};

/// Render a sequence layout to a string.
pub fn render(layout: &SequenceLayout, charset: &CharSet) -> String {
    if layout.participants.is_empty() {
        return String::new();
    }

    let mut canvas = Canvas::new(layout.width, layout.height);

    for p in &layout.participants {
        draw_participant_header(&mut canvas, p, charset);
    }

    let lifeline_start = 3;
    let lifeline_end = layout.height;
    for p in &layout.participants {
        for y in lifeline_start..lifeline_end {
            canvas.set(p.center_x, y, charset.vertical);
        }
    }

    // Draw activation boxes on lifelines
    for activation in &layout.activations {
        draw_activation(&mut canvas, activation, &layout.participants, charset);
    }

    for row in &layout.rows {
        match row {
            RowLayout::Message {
                y,
                from_idx,
                to_idx,
                line_style,
                arrow_head,
                text,
                number,
            } => {
                let from_x = layout.participants[*from_idx].center_x;
                let to_x = layout.participants[*to_idx].center_x;

                if from_idx == to_idx {
                    draw_self_message(
                        &mut canvas,
                        from_x,
                        *y,
                        text,
                        number,
                        line_style,
                        arrow_head,
                        charset,
                    );
                } else {
                    draw_message(
                        &mut canvas,
                        from_x,
                        to_x,
                        *y,
                        text,
                        number,
                        line_style,
                        arrow_head,
                        charset,
                    );
                }
            }
            RowLayout::Note {
                y,
                placement,
                participant_indices,
                text,
            } => {
                draw_note(
                    &mut canvas,
                    &layout.participants,
                    placement,
                    participant_indices,
                    *y,
                    text,
                    charset,
                );
            }
        }
    }

    canvas.to_string()
}

fn draw_participant_header(canvas: &mut Canvas, p: &ParticipantLayout, cs: &CharSet) {
    let x = p.box_x;
    let w = p.box_width;

    canvas.set(x, 0, cs.corner_tl);
    for i in 1..w - 1 {
        canvas.set(x + i, 0, cs.horizontal);
    }
    canvas.set(x + w - 1, 0, cs.corner_tr);

    canvas.set(x, 1, cs.vertical);
    canvas.set(x + 1, 1, ' ');
    canvas.write_str(x + 2, 1, &p.label);
    canvas.set(x + 2 + p.label.len(), 1, ' ');
    canvas.set(x + w - 1, 1, cs.vertical);

    canvas.set(x, 2, cs.corner_bl);
    for i in 1..w - 1 {
        canvas.set(x + i, 2, cs.horizontal);
    }
    canvas.set(x + w - 1, 2, cs.corner_br);

    canvas.set(p.center_x, 2, cs.tee_down);
}

/// Resolve the arrowhead character for the given direction and head type.
fn arrow_char(arrow_head: &ArrowHead, left_to_right: bool) -> char {
    match arrow_head {
        // Filled and Open look identical in monospace text mode
        ArrowHead::Filled | ArrowHead::Open => {
            if left_to_right {
                '>'
            } else {
                '<'
            }
        }
        ArrowHead::Cross => 'x',
        ArrowHead::Async => {
            if left_to_right {
                ')'
            } else {
                '('
            }
        }
    }
}

/// Resolve the line character for the given line style.
fn line_char(line_style: &LineStyle, cs: &CharSet) -> char {
    match line_style {
        LineStyle::Solid => cs.horizontal,
        LineStyle::Dashed => cs.dotted_horizontal,
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_message(
    canvas: &mut Canvas,
    from_x: usize,
    to_x: usize,
    y: usize,
    text: &str,
    number: &Option<usize>,
    line_style: &LineStyle,
    arrow_head: &ArrowHead,
    cs: &CharSet,
) {
    let left_to_right = to_x > from_x;
    let (start_x, end_x) = if left_to_right {
        (from_x + 1, to_x)
    } else {
        (to_x + 1, from_x)
    };

    let lc = line_char(line_style, cs);
    let ac = arrow_char(arrow_head, left_to_right);

    for x in start_x..end_x {
        canvas.set(x, y, lc);
    }

    if left_to_right {
        canvas.set(end_x - 1, y, ac);
    } else {
        canvas.set(start_x, y, ac);
    }

    let label = format_label(text, number);
    if !label.is_empty() {
        let label_x = start_x + 1;
        canvas.write_str(label_x, y, &label);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_self_message(
    canvas: &mut Canvas,
    center_x: usize,
    y: usize,
    text: &str,
    number: &Option<usize>,
    _line_style: &LineStyle,
    arrow_head: &ArrowHead,
    cs: &CharSet,
) {
    let arm_end = center_x + SELF_MSG_WIDTH;

    canvas.set(center_x, y, cs.tee_right);
    for x in (center_x + 1)..arm_end {
        canvas.set(x, y, cs.horizontal);
    }
    canvas.set(arm_end, y, cs.corner_tr);

    let label = format_label(text, number);
    if !label.is_empty() {
        canvas.write_str(arm_end + 2, y, &label);
    }

    canvas.set(arm_end, y + 1, cs.vertical);

    canvas.set(center_x, y + 2, arrow_char(arrow_head, false));
    for x in (center_x + 1)..arm_end {
        canvas.set(x, y + 2, cs.horizontal);
    }
    canvas.set(arm_end, y + 2, cs.corner_br);
}

fn draw_note(
    canvas: &mut Canvas,
    participants: &[ParticipantLayout],
    placement: &NotePlacement,
    participant_indices: &[usize],
    y: usize,
    text: &str,
    cs: &CharSet,
) {
    let min_box_width = text.len() + 4;

    let (box_x, box_width) = match placement {
        NotePlacement::LeftOf => {
            let center_x = participants[participant_indices[0]].center_x;
            let bx = center_x.saturating_sub(min_box_width + 1);
            (bx, min_box_width)
        }
        NotePlacement::RightOf => {
            let center_x = participants[participant_indices[0]].center_x;
            (center_x + 2, min_box_width)
        }
        NotePlacement::Over if participant_indices.len() == 2 => {
            let cx1 = participants[participant_indices[0]].center_x;
            let cx2 = participants[participant_indices[1]].center_x;
            let left = cx1.min(cx2);
            let right = cx1.max(cx2);
            let span_width = min_box_width.max(right - left + 4);
            let mid = (left + right) / 2;
            let bx = mid.saturating_sub(span_width / 2);
            (bx, span_width)
        }
        NotePlacement::Over => {
            let center_x = participants[participant_indices[0]].center_x;
            let bx = center_x.saturating_sub(min_box_width / 2);
            (bx, min_box_width)
        }
    };

    canvas.set(box_x, y, cs.corner_tl);
    for i in 1..box_width - 1 {
        canvas.set(box_x + i, y, cs.horizontal);
    }
    canvas.set(box_x + box_width - 1, y, cs.corner_tr);

    canvas.set(box_x, y + 1, cs.vertical);
    // Fill entire interior with spaces first (overwrites lifelines)
    for i in 1..box_width - 1 {
        canvas.set(box_x + i, y + 1, ' ');
    }
    let text_offset = (box_width - 2 - text.len()) / 2;
    canvas.write_str(box_x + 1 + text_offset, y + 1, text);
    canvas.set(box_x + box_width - 1, y + 1, cs.vertical);

    canvas.set(box_x, y + 2, cs.corner_bl);
    for i in 1..box_width - 1 {
        canvas.set(box_x + i, y + 2, cs.horizontal);
    }
    canvas.set(box_x + box_width - 1, y + 2, cs.corner_br);
}

fn format_label(text: &str, number: &Option<usize>) -> String {
    match number {
        Some(n) => {
            if text.is_empty() {
                format!("{n}.")
            } else {
                format!("{n}. {text}")
            }
        }
        None => text.to_string(),
    }
}

/// Draw an activation bar on a participant's lifeline.
///
/// Replaces the thin `│` lifeline with `║` during the active region.
/// Nested activations are drawn one column to the right of the lifeline.
fn draw_activation(
    canvas: &mut Canvas,
    activation: &ActivationRect,
    participants: &[ParticipantLayout],
    cs: &CharSet,
) {
    let center_x = participants[activation.participant_idx].center_x;
    // depth 0 draws on the lifeline itself; deeper levels offset right
    let x = center_x + activation.depth;

    let activation_char = if cs.is_ascii() { '#' } else { '║' };
    for y in activation.y_start..=activation.y_end {
        canvas.set(x, y, activation_char);
    }
}
