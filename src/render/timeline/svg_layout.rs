//! Proportional layout engine for sequence diagram SVG rendering.
//!
//! Parallel to the character-grid `timeline::sequence::layout` but produces
//! f64 pixel coordinates suitable for SVG output. Consumes the shared
//! `Sequence` model and `ProportionalTextMetrics` for text measurement.

use crate::graph::measure::ProportionalTextMetrics;
use crate::timeline::sequence::model::{
    ArrowHead, BlockDividerKind, BlockKind, LineStyle, NotePlacement, ParticipantKind, Sequence,
    SequenceEvent,
};

// ---------------------------------------------------------------------------
// Layout constants (pixels)
// ---------------------------------------------------------------------------

/// Horizontal padding inside participant boxes.
const PARTICIPANT_PADDING_X: f64 = 16.0;
/// Vertical padding inside participant boxes.
const PARTICIPANT_PADDING_Y: f64 = 10.0;
/// Minimum gap between adjacent participant centers.
const MIN_PARTICIPANT_GAP: f64 = 150.0;
/// Vertical space between the bottom of the header and the first event.
const HEADER_MARGIN: f64 = 20.0;
/// Vertical distance between successive events.
const EVENT_SPACING: f64 = 50.0;
/// Width of the self-message loop arm extending right of the lifeline.
const SELF_MSG_ARM: f64 = 30.0;
/// Vertical height of the self-message loop.
const SELF_MSG_HEIGHT: f64 = 30.0;
/// Horizontal padding inside note boxes.
const NOTE_PADDING_X: f64 = 10.0;
/// Vertical padding inside note boxes.
const NOTE_PADDING_Y: f64 = 8.0;
/// Gap between a lifeline and an adjacent note box.
const NOTE_GAP: f64 = 10.0;
/// Width of activation bars drawn on lifelines.
const ACTIVATION_WIDTH: f64 = 10.0;
/// Padding around the entire diagram.
const DIAGRAM_PADDING: f64 = 10.0;
/// Extra gap between message label and arrow line.
const LABEL_ABOVE_GAP: f64 = 4.0;
/// Vertical reservation for block headers, dividers, and footers.
const BLOCK_ROW_SPACING: f64 = 28.0;

// ---------------------------------------------------------------------------
// Layout output types
// ---------------------------------------------------------------------------

/// Complete proportional layout for SVG rendering.
#[derive(Debug)]
pub struct SvgSequenceLayout {
    pub participants: Vec<SvgParticipant>,
    pub lifelines: Vec<SvgLifeline>,
    pub rows: Vec<SvgRow>,
    pub blocks: Vec<SvgBlock>,
    pub activations: Vec<SvgActivation>,
    pub width: f64,
    pub height: f64,
    pub font_family: String,
    pub font_size: f64,
}

/// A positioned participant header box.
#[derive(Debug)]
pub struct SvgParticipant {
    pub center_x: f64,
    pub rect: SvgRect,
    pub label: String,
    pub kind: ParticipantKind,
}

/// Simple rectangle.
#[derive(Debug, Clone)]
pub struct SvgRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A vertical lifeline segment.
#[derive(Debug)]
pub struct SvgLifeline {
    pub x: f64,
    pub y_start: f64,
    pub y_end: f64,
}

/// A positioned event in the layout.
#[derive(Debug)]
pub enum SvgRow {
    Message(SvgMessage),
    SelfMessage(SvgSelfMessage),
    Note(SvgNote),
}

/// A message arrow between two lifelines.
#[derive(Debug)]
pub struct SvgMessage {
    pub from_x: f64,
    pub to_x: f64,
    pub y: f64,
    pub label: String,
    pub label_x: f64,
    pub label_y: f64,
    pub line_style: LineStyle,
    pub arrow_head: ArrowHead,
}

/// A self-referencing message (loop to the right of one lifeline).
#[derive(Debug)]
pub struct SvgSelfMessage {
    pub x: f64,
    pub y: f64,
    pub arm_width: f64,
    pub height: f64,
    pub label: String,
    pub label_x: f64,
    pub label_y: f64,
    pub line_style: LineStyle,
    pub arrow_head: ArrowHead,
}

/// A positioned note box.
#[derive(Debug)]
pub struct SvgNote {
    pub rect: SvgRect,
    pub text: String,
}

/// An activation bar on a lifeline.
#[derive(Debug)]
pub struct SvgActivation {
    pub x: f64,
    pub y_start: f64,
    pub y_end: f64,
    pub width: f64,
    pub depth: usize,
}

#[derive(Debug)]
pub struct SvgBlockDivider {
    pub y: f64,
    pub kind: BlockDividerKind,
    pub label: String,
}

#[derive(Debug)]
pub struct SvgBlock {
    pub rect: SvgRect,
    pub depth: usize,
    pub kind: BlockKind,
    pub label: String,
    pub dividers: Vec<SvgBlockDivider>,
}

// ---------------------------------------------------------------------------
// Layout engine
// ---------------------------------------------------------------------------

/// Count how many ActivateStart events for `participant` appear at the head
/// of `events` before the next non-activation event. This handles the
/// shorthand `A->>+B: msg` where the activation starts on the same message.
fn pending_activations(events: &[SequenceEvent], participant: usize) -> usize {
    events
        .iter()
        .take_while(|e| {
            matches!(
                e,
                SequenceEvent::ActivateStart { .. } | SequenceEvent::ActivateEnd { .. }
            )
        })
        .filter(
            |e| matches!(e, SequenceEvent::ActivateStart { participant: p } if *p == participant),
        )
        .count()
}

/// Compute the x position for a message endpoint on a participant.
///
/// If the participant has an active activation (depth > 0), returns the
/// edge of the topmost activation box. Otherwise returns the lifeline center.
fn activation_edge(center_x: f64, depth: usize, right_side: bool) -> f64 {
    if depth == 0 {
        return center_x;
    }
    let top_depth = depth - 1;
    let box_x = center_x - ACTIVATION_WIDTH / 2.0 + (top_depth as f64 * 3.0);
    if right_side {
        box_x + ACTIVATION_WIDTH
    } else {
        box_x
    }
}

/// Compute a proportional SVG layout for a sequence model.
pub fn layout(
    model: &Sequence,
    metrics: &ProportionalTextMetrics,
    font_family: &str,
) -> SvgSequenceLayout {
    if model.participants.is_empty() {
        return SvgSequenceLayout {
            participants: Vec::new(),
            lifelines: Vec::new(),
            rows: Vec::new(),
            blocks: Vec::new(),
            activations: Vec::new(),
            width: 0.0,
            height: 0.0,
            font_family: font_family.to_string(),
            font_size: metrics.font_size,
        };
    }

    // 1. Measure participant boxes
    let box_sizes: Vec<(f64, f64)> = model
        .participants
        .iter()
        .map(|p| {
            metrics.measure_text_with_padding(
                &p.label,
                PARTICIPANT_PADDING_X,
                PARTICIPANT_PADDING_Y,
            )
        })
        .collect();

    let header_height = box_sizes.iter().map(|(_, h)| *h).fold(0.0_f64, f64::max);

    // 2. Compute participant gap from message labels
    let participant_gap = compute_participant_gap(model, metrics);

    // 3. Compute left margin for left-of notes
    let left_margin = compute_left_note_margin(model, metrics, &box_sizes, participant_gap);

    // 4. Position participants horizontally
    let mut participants = Vec::with_capacity(model.participants.len());
    let mut x = DIAGRAM_PADDING + left_margin;

    for (i, p) in model.participants.iter().enumerate() {
        let (bw, _bh) = box_sizes[i];
        let center_x = x + bw / 2.0;

        participants.push(SvgParticipant {
            center_x,
            rect: SvgRect {
                x,
                y: DIAGRAM_PADDING,
                width: bw,
                height: header_height,
            },
            label: p.label.clone(),
            kind: p.kind.clone(),
        });

        if i < model.participants.len() - 1 {
            let next_bw = box_sizes[i + 1].0;
            x = center_x + participant_gap - next_bw / 2.0;
        }
    }

    // 5. Walk events and build rows
    let mut rows = Vec::new();
    let mut blocks = Vec::new();
    let mut cursor_y = DIAGRAM_PADDING + header_height + HEADER_MARGIN;

    let num_participants = model.participants.len();
    let mut activation_stacks: Vec<Vec<(f64, usize)>> = vec![Vec::new(); num_participants];
    let mut activation_depth: Vec<usize> = vec![0; num_participants];
    let mut activations: Vec<SvgActivation> = Vec::new();
    let mut open_blocks: Vec<OpenSvgBlock> = Vec::new();
    let mut last_message_y = cursor_y;

    for (ev_idx, event) in model.events.iter().enumerate() {
        match event {
            SequenceEvent::Message {
                from,
                to,
                line_style,
                arrow_head,
                text,
                number,
            } => {
                let label = format_label(text, number);
                let from_cx = participants[*from].center_x;
                let to_cx = participants[*to].center_x;

                // Peek ahead: if the next events activate from/to participants,
                // include those in the depth calculation so the arrow terminates
                // at the activation box edge rather than the lifeline.
                let from_depth = activation_depth[*from]
                    + pending_activations(&model.events[ev_idx + 1..], *from);
                let to_depth =
                    activation_depth[*to] + pending_activations(&model.events[ev_idx + 1..], *to);

                last_message_y = cursor_y;

                if from == to {
                    // Self-message: start from activation edge if active
                    let self_x = activation_edge(
                        from_cx, from_depth, true, // right side
                    );
                    let label_x = self_x + SELF_MSG_ARM + 8.0;
                    let label_y = cursor_y + LABEL_ABOVE_GAP;

                    rows.push(SvgRow::SelfMessage(SvgSelfMessage {
                        x: self_x,
                        y: cursor_y,
                        arm_width: SELF_MSG_ARM,
                        height: SELF_MSG_HEIGHT,
                        label,
                        label_x,
                        label_y,
                        line_style: *line_style,
                        arrow_head: *arrow_head,
                    }));
                    let (left, right) = svg_row_extent(rows.last().unwrap(), metrics);
                    update_open_svg_block_extents(&mut open_blocks, left, right);

                    cursor_y += SELF_MSG_HEIGHT + EVENT_SPACING;
                } else {
                    // Adjust endpoints to activation box edges when active
                    let left_to_right = to_cx > from_cx;
                    let from_x = activation_edge(
                        from_cx,
                        from_depth,
                        left_to_right, // right edge if going right
                    );
                    let to_x = activation_edge(
                        to_cx,
                        to_depth,
                        !left_to_right, // left edge if arrow comes from left
                    );

                    let mid_x = (from_x + to_x) / 2.0;
                    let label_y = cursor_y - LABEL_ABOVE_GAP;

                    rows.push(SvgRow::Message(SvgMessage {
                        from_x,
                        to_x,
                        y: cursor_y,
                        label,
                        label_x: mid_x,
                        label_y,
                        line_style: *line_style,
                        arrow_head: *arrow_head,
                    }));
                    let (left, right) = svg_row_extent(rows.last().unwrap(), metrics);
                    update_open_svg_block_extents(&mut open_blocks, left, right);

                    cursor_y += EVENT_SPACING;
                }
            }
            SequenceEvent::Note {
                placement,
                participants: indices,
                text,
            } => {
                let (tw, th) =
                    metrics.measure_text_with_padding(text, NOTE_PADDING_X, NOTE_PADDING_Y);

                let rect = match placement {
                    NotePlacement::LeftOf => {
                        let cx = participants[indices[0]].center_x;
                        SvgRect {
                            x: cx - NOTE_GAP - tw,
                            y: cursor_y,
                            width: tw,
                            height: th,
                        }
                    }
                    NotePlacement::RightOf => {
                        let cx = participants[indices[0]].center_x;
                        SvgRect {
                            x: cx + NOTE_GAP,
                            y: cursor_y,
                            width: tw,
                            height: th,
                        }
                    }
                    NotePlacement::Over if indices.len() == 2 => {
                        let cx1 = participants[indices[0]].center_x;
                        let cx2 = participants[indices[1]].center_x;
                        let left = cx1.min(cx2);
                        let right = cx1.max(cx2);
                        let span_width = tw.max(right - left + 20.0);
                        let mid = (left + right) / 2.0;
                        SvgRect {
                            x: mid - span_width / 2.0,
                            y: cursor_y,
                            width: span_width,
                            height: th,
                        }
                    }
                    NotePlacement::Over => {
                        let cx = participants[indices[0]].center_x;
                        SvgRect {
                            x: cx - tw / 2.0,
                            y: cursor_y,
                            width: tw,
                            height: th,
                        }
                    }
                };

                rows.push(SvgRow::Note(SvgNote {
                    rect: rect.clone(),
                    text: text.clone(),
                }));
                update_open_svg_block_extents(&mut open_blocks, rect.x, rect.x + rect.width);

                cursor_y += rect.height + EVENT_SPACING;
            }
            SequenceEvent::ActivateStart { participant } => {
                let depth = activation_depth[*participant];
                activation_stacks[*participant].push((last_message_y, depth));
                activation_depth[*participant] += 1;
            }
            SequenceEvent::ActivateEnd { participant } => {
                if let Some((y_start, depth)) = activation_stacks[*participant].pop() {
                    let y_end = last_message_y.max(y_start);
                    let cx = participants[*participant].center_x;
                    activations.push(SvgActivation {
                        x: cx - ACTIVATION_WIDTH / 2.0 + (depth as f64 * 3.0),
                        y_start,
                        y_end,
                        width: ACTIVATION_WIDTH,
                        depth,
                    });
                    activation_depth[*participant] =
                        activation_depth[*participant].saturating_sub(1);
                }
            }
            SequenceEvent::BlockStart { kind, label } => {
                open_blocks.push(OpenSvgBlock {
                    top_y: cursor_y,
                    depth: open_blocks.len(),
                    kind: *kind,
                    label: label.clone(),
                    dividers: Vec::new(),
                    min_x: None,
                    max_x: None,
                });
                cursor_y += BLOCK_ROW_SPACING;
            }
            SequenceEvent::BlockDivider { kind, label } => {
                if let Some(block) = open_blocks.last_mut() {
                    block.dividers.push(SvgBlockDivider {
                        y: cursor_y,
                        kind: *kind,
                        label: label.clone(),
                    });
                }
                cursor_y += BLOCK_ROW_SPACING;
            }
            SequenceEvent::BlockEnd => {
                if let Some(block) = open_blocks.pop() {
                    let finalized =
                        finalize_svg_block(block, cursor_y, participants.as_slice(), metrics);
                    update_open_svg_block_extents(
                        &mut open_blocks,
                        finalized.rect.x,
                        finalized.rect.x + finalized.rect.width,
                    );
                    blocks.push(finalized);
                }
                cursor_y += BLOCK_ROW_SPACING;
            }
        }
    }

    // Close unclosed activations
    for (pidx, stack) in activation_stacks.iter_mut().enumerate() {
        while let Some((y_start, depth)) = stack.pop() {
            let y_end = (cursor_y - EVENT_SPACING / 2.0).max(y_start);
            let cx = participants[pidx].center_x;
            activations.push(SvgActivation {
                x: cx - ACTIVATION_WIDTH / 2.0 + (depth as f64 * 3.0),
                y_start,
                y_end,
                width: ACTIVATION_WIDTH,
                depth,
            });
        }
    }

    // Sort by depth so outer activations render first (behind inner ones).
    activations.sort_by_key(|a| a.depth);
    blocks.sort_by_key(|block| block.depth);

    // 6. Compute lifelines
    let lifeline_end = cursor_y;
    let lifelines: Vec<SvgLifeline> = participants
        .iter()
        .map(|p| SvgLifeline {
            x: p.center_x,
            y_start: DIAGRAM_PADDING + header_height,
            y_end: lifeline_end,
        })
        .collect();

    // 7. Compute diagram bounds
    let max_right = participants
        .iter()
        .map(|p| p.rect.x + p.rect.width)
        .fold(0.0_f64, f64::max);

    let row_right = rows
        .iter()
        .map(|row| svg_row_extent(row, metrics).1)
        .fold(0.0_f64, f64::max);

    let block_right = blocks
        .iter()
        .map(|block| block.rect.x + block.rect.width)
        .fold(0.0_f64, f64::max);

    let width = max_right.max(row_right).max(block_right) + DIAGRAM_PADDING;
    let height = lifeline_end + DIAGRAM_PADDING;

    SvgSequenceLayout {
        participants,
        lifelines,
        rows,
        blocks,
        activations,
        width,
        height,
        font_family: font_family.to_string(),
        font_size: metrics.font_size,
    }
}

#[derive(Debug)]
struct OpenSvgBlock {
    top_y: f64,
    depth: usize,
    kind: BlockKind,
    label: String,
    dividers: Vec<SvgBlockDivider>,
    min_x: Option<f64>,
    max_x: Option<f64>,
}

fn update_open_svg_block_extents(open_blocks: &mut [OpenSvgBlock], left: f64, right: f64) {
    for block in open_blocks {
        block.min_x = Some(block.min_x.map_or(left, |current| current.min(left)));
        block.max_x = Some(block.max_x.map_or(right, |current| current.max(right)));
    }
}

fn finalize_svg_block(
    block: OpenSvgBlock,
    bottom_y: f64,
    participants: &[SvgParticipant],
    metrics: &ProportionalTextMetrics,
) -> SvgBlock {
    let fallback_center = participants.first().map(|p| p.center_x).unwrap_or(20.0);
    let raw_left = block.min_x.unwrap_or(fallback_center - 12.0);
    let raw_right = block.max_x.unwrap_or(fallback_center + 12.0);
    let inset = block.depth as f64 * 8.0;

    let left_x = raw_left - 12.0 + inset;
    let mut right_x = raw_right + 12.0 - inset;

    let min_width = block
        .dividers
        .iter()
        .map(|divider| {
            let label = format_badge(divider.kind.keyword(), &divider.label);
            metrics.measure_text_with_padding(&label, 12.0, 0.0).0
        })
        .fold(
            metrics
                .measure_text_with_padding(
                    &format_badge(block.kind.keyword(), &block.label),
                    12.0,
                    0.0,
                )
                .0,
            f64::max,
        )
        .max(48.0);

    if right_x < left_x + min_width {
        right_x = left_x + min_width;
    }

    SvgBlock {
        rect: SvgRect {
            x: left_x,
            y: block.top_y,
            width: right_x - left_x,
            height: (bottom_y - block.top_y).max(BLOCK_ROW_SPACING),
        },
        depth: block.depth,
        kind: block.kind,
        label: block.label,
        dividers: block.dividers,
    }
}

fn svg_row_extent(row: &SvgRow, metrics: &ProportionalTextMetrics) -> (f64, f64) {
    match row {
        SvgRow::Message(msg) => {
            let (lw, _) = metrics.measure_text_with_padding(&msg.label, 0.0, 0.0);
            let label_left = msg.label_x - lw / 2.0;
            let label_right = msg.label_x + lw / 2.0;
            (
                msg.from_x.min(msg.to_x).min(label_left),
                msg.from_x.max(msg.to_x).max(label_right),
            )
        }
        SvgRow::SelfMessage(sm) => {
            let (lw, _) = metrics.measure_text_with_padding(&sm.label, 0.0, 0.0);
            (sm.x, (sm.x + sm.arm_width + 8.0 + lw).max(sm.label_x + lw))
        }
        SvgRow::Note(note) => (note.rect.x, note.rect.x + note.rect.width),
    }
}

/// Compute minimum gap between participant centers based on message labels.
fn compute_participant_gap(model: &Sequence, metrics: &ProportionalTextMetrics) -> f64 {
    let max_label_width = model
        .events
        .iter()
        .filter_map(|e| match e {
            SequenceEvent::Message {
                from,
                to,
                text,
                number,
                ..
            } if from != to => {
                let label = format_label(text, number);
                let (w, _) = metrics.measure_text_with_padding(&label, 0.0, 0.0);
                Some(w)
            }
            _ => None,
        })
        .fold(0.0_f64, f64::max);

    (max_label_width + 40.0).max(MIN_PARTICIPANT_GAP)
}

/// Compute extra left margin needed for left-of notes on the first participant.
fn compute_left_note_margin(
    model: &Sequence,
    metrics: &ProportionalTextMetrics,
    box_sizes: &[(f64, f64)],
    participant_gap: f64,
) -> f64 {
    if model.participants.is_empty() {
        return 0.0;
    }

    // Tentative center_x for each participant
    let mut centers = Vec::with_capacity(model.participants.len());
    let mut x = 0.0_f64;
    for (i, (bw, _)) in box_sizes.iter().enumerate() {
        centers.push(x + bw / 2.0);
        if i < box_sizes.len() - 1 {
            let next_bw = box_sizes[i + 1].0;
            x = centers[i] + participant_gap - next_bw / 2.0;
        }
    }

    let mut max_overhang = 0.0_f64;
    for event in &model.events {
        if let SequenceEvent::Note {
            placement: NotePlacement::LeftOf,
            participants: indices,
            text,
        } = event
        {
            let (tw, _) = metrics.measure_text_with_padding(text, NOTE_PADDING_X, NOTE_PADDING_Y);
            let center_x = centers[indices[0]];
            let needed = tw + NOTE_GAP;
            if needed > center_x {
                max_overhang = max_overhang.max(needed - center_x);
            }
        }
    }
    max_overhang
}

/// Format a message label with optional autonumber prefix.
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

fn format_badge(keyword: &str, label: &str) -> String {
    if label.is_empty() {
        format!("[{keyword}]")
    } else {
        format!("[{keyword}] {label}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::measure::ProportionalTextMetrics;
    use crate::timeline::sequence::model::{Participant, ParticipantKind, Sequence};

    fn test_metrics() -> ProportionalTextMetrics {
        ProportionalTextMetrics::new(16.0, 15.0, 15.0)
    }

    #[test]
    fn layout_empty_model() {
        let model = Sequence {
            participants: Vec::new(),
            events: Vec::new(),
            autonumber: false,
        };
        let layout = layout(&model, &test_metrics(), "sans-serif");
        assert_eq!(layout.width, 0.0);
        assert_eq!(layout.height, 0.0);
        assert!(layout.participants.is_empty());
    }

    #[test]
    fn layout_two_participants_one_message() {
        let model = Sequence {
            participants: vec![
                Participant {
                    id: "Alice".into(),
                    label: "Alice".into(),
                    kind: ParticipantKind::Participant,
                },
                Participant {
                    id: "Bob".into(),
                    label: "Bob".into(),
                    kind: ParticipantKind::Participant,
                },
            ],
            events: vec![SequenceEvent::Message {
                from: 0,
                to: 1,
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Filled,
                text: "Hello".into(),
                number: None,
            }],
            autonumber: false,
        };
        let layout = layout(&model, &test_metrics(), "sans-serif");

        assert_eq!(layout.participants.len(), 2);
        assert_eq!(layout.lifelines.len(), 2);
        assert_eq!(layout.rows.len(), 1);
        assert!(layout.width > 0.0);
        assert!(layout.height > 0.0);

        // First participant should be left of second
        assert!(layout.participants[0].center_x < layout.participants[1].center_x);
    }

    #[test]
    fn self_message_produces_self_message_row() {
        let model = Sequence {
            participants: vec![Participant {
                id: "A".into(),
                label: "A".into(),
                kind: ParticipantKind::Participant,
            }],
            events: vec![SequenceEvent::Message {
                from: 0,
                to: 0,
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Filled,
                text: "self".into(),
                number: None,
            }],
            autonumber: false,
        };
        let layout = layout(&model, &test_metrics(), "sans-serif");

        assert_eq!(layout.rows.len(), 1);
        assert!(matches!(layout.rows[0], SvgRow::SelfMessage(_)));
    }

    #[test]
    fn layout_tracks_svg_blocks() {
        let model = Sequence {
            participants: vec![
                Participant {
                    id: "A".into(),
                    label: "A".into(),
                    kind: ParticipantKind::Participant,
                },
                Participant {
                    id: "B".into(),
                    label: "B".into(),
                    kind: ParticipantKind::Participant,
                },
            ],
            events: vec![
                SequenceEvent::BlockStart {
                    kind: BlockKind::Alt,
                    label: "available".into(),
                },
                SequenceEvent::Message {
                    from: 0,
                    to: 1,
                    line_style: LineStyle::Solid,
                    arrow_head: ArrowHead::Filled,
                    text: "Request".into(),
                    number: None,
                },
                SequenceEvent::BlockDivider {
                    kind: BlockDividerKind::Else,
                    label: "busy".into(),
                },
                SequenceEvent::Message {
                    from: 1,
                    to: 0,
                    line_style: LineStyle::Dashed,
                    arrow_head: ArrowHead::Open,
                    text: "Retry later".into(),
                    number: None,
                },
                SequenceEvent::BlockEnd,
            ],
            autonumber: false,
        };
        let layout = layout(&model, &test_metrics(), "sans-serif");

        assert_eq!(layout.blocks.len(), 1);
        assert_eq!(layout.blocks[0].dividers.len(), 1);
        assert!(layout.blocks[0].rect.width > 0.0);
        assert!(layout.blocks[0].rect.height > 0.0);
    }
}
