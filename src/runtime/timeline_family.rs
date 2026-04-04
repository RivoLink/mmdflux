//! Runtime rendering for timeline-family payloads (sequence diagrams).
//!
//! Parallel to `graph_family` — computes SVG layout and serializes to MMDS
//! JSON via the types defined in `mmds::sequence`.

use crate::graph::measure::ProportionalTextMetrics;
use crate::mmds::sequence::{
    self, MmdsActivation, MmdsBlock, MmdsBlockDivider, MmdsBounds, MmdsMessage, MmdsNote,
    MmdsParticipant, MmdsParticipantBox, MmdsPosition, MmdsRect, MmdsSize, SequenceMetadata,
    SequenceOutput,
};
use crate::render::timeline::svg_layout::{
    self as svg_layout, SvgBlock, SvgRow, SvgSequenceLayout,
};
use crate::timeline::sequence::model::{
    ArrowHead, BlockDividerKind, BlockKind, LineStyle, ParticipantKind, Sequence,
};

// ---------------------------------------------------------------------------
// String helpers
// ---------------------------------------------------------------------------

fn line_style_str(s: LineStyle) -> &'static str {
    match s {
        LineStyle::Solid => "solid",
        LineStyle::Dashed => "dashed",
    }
}

fn arrow_head_str(a: ArrowHead) -> &'static str {
    match a {
        ArrowHead::None => "none",
        ArrowHead::Filled => "filled",
        ArrowHead::Cross => "cross",
        ArrowHead::Async => "async",
    }
}

fn participant_kind_str(k: &ParticipantKind) -> &'static str {
    match k {
        ParticipantKind::Participant => "participant",
        ParticipantKind::Actor => "actor",
    }
}

fn block_kind_str(k: BlockKind) -> &'static str {
    match k {
        BlockKind::Loop => "loop",
        BlockKind::Alt => "alt",
        BlockKind::Opt => "opt",
        BlockKind::Par => "par",
        BlockKind::Critical => "critical",
        BlockKind::Break => "break",
        BlockKind::Rect => "rect",
    }
}

fn block_divider_kind_str(k: BlockDividerKind) -> &'static str {
    match k {
        BlockDividerKind::Else => "else",
        BlockDividerKind::And => "and",
        BlockDividerKind::Option => "option",
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Serialize a sequence diagram model to MMDS JSON using proportional layout
/// positions from the SVG layout engine.
pub(in crate::runtime) fn to_json(model: &Sequence, metrics: &ProportionalTextMetrics) -> String {
    let svg = svg_layout::layout(model, metrics, "sans-serif");
    let output = build_output(model, &svg);
    sequence::serialize(&output)
}

fn build_output(model: &Sequence, svg: &SvgSequenceLayout) -> SequenceOutput {
    let participants = build_participants(model, svg);
    let (messages, notes) = build_messages_and_notes(svg);
    let activations = build_activations(svg);
    let blocks = build_blocks(svg);
    let participant_boxes = build_participant_boxes(model, svg);

    SequenceOutput {
        version: 1,
        geometry_level: "layout".to_string(),
        metadata: SequenceMetadata {
            diagram_type: "sequence".to_string(),
            bounds: MmdsBounds {
                width: svg.width,
                height: svg.height,
            },
        },
        nodes: Vec::new(),
        edges: Vec::new(),
        participants,
        messages,
        notes,
        activations,
        blocks,
        participant_boxes,
    }
}

fn build_participants(model: &Sequence, svg: &SvgSequenceLayout) -> Vec<MmdsParticipant> {
    model
        .participants
        .iter()
        .zip(svg.participants.iter())
        .map(|(p, sp)| MmdsParticipant {
            id: p.id.clone(),
            label: p.label.clone(),
            kind: participant_kind_str(&p.kind).to_string(),
            position: MmdsPosition {
                x: sp.rect.x,
                y: sp.rect.y,
            },
            size: MmdsSize {
                width: sp.rect.width,
                height: sp.rect.height,
            },
            lifeline_x: sp.center_x,
        })
        .collect()
}

fn build_messages_and_notes(svg: &SvgSequenceLayout) -> (Vec<MmdsMessage>, Vec<MmdsNote>) {
    let mut messages = Vec::new();
    let mut notes = Vec::new();
    let mut msg_idx = 0usize;

    let lifeline_xs: Vec<f64> = svg.participants.iter().map(|p| p.center_x).collect();

    for row in &svg.rows {
        match row {
            SvgRow::Message(m) => {
                let from = nearest_participant(&lifeline_xs, m.from_x);
                let to = nearest_participant(&lifeline_xs, m.to_x);
                messages.push(MmdsMessage {
                    id: format!("m{msg_idx}"),
                    from,
                    to,
                    line_style: line_style_str(m.line_style).to_string(),
                    arrow_head: arrow_head_str(m.arrow_head).to_string(),
                    text: m.label.clone(),
                    y: m.y,
                });
                msg_idx += 1;
            }
            SvgRow::SelfMessage(sm) => {
                let from = nearest_participant(&lifeline_xs, sm.x);
                messages.push(MmdsMessage {
                    id: format!("m{msg_idx}"),
                    from,
                    to: from,
                    line_style: line_style_str(sm.line_style).to_string(),
                    arrow_head: arrow_head_str(sm.arrow_head).to_string(),
                    text: sm.label.clone(),
                    y: sm.y,
                });
                msg_idx += 1;
            }
            SvgRow::Note(n) => {
                let participants = participants_for_note(&lifeline_xs, n);
                let placement = placement_for_note(&lifeline_xs, n, &participants);
                notes.push(MmdsNote {
                    placement: placement.to_string(),
                    participants,
                    text: n.text.clone(),
                    position: MmdsPosition {
                        x: n.rect.x,
                        y: n.rect.y,
                    },
                    size: MmdsSize {
                        width: n.rect.width,
                        height: n.rect.height,
                    },
                });
            }
        }
    }

    (messages, notes)
}

/// Find the participant index whose lifeline is closest to the given x.
fn nearest_participant(lifeline_xs: &[f64], x: f64) -> usize {
    lifeline_xs
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (x - *a).abs().partial_cmp(&(x - *b).abs()).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0)
}

/// Determine which participants a note is associated with based on its position.
fn participants_for_note(lifeline_xs: &[f64], note: &svg_layout::SvgNote) -> Vec<usize> {
    let note_left = note.rect.x;
    let note_right = note.rect.x + note.rect.width;

    // Check if the note spans between two participants (over A,B).
    let mut covered: Vec<usize> = lifeline_xs
        .iter()
        .enumerate()
        .filter(|(_, lx)| **lx >= note_left && **lx <= note_right)
        .map(|(i, _)| i)
        .collect();

    if covered.is_empty() {
        let note_center = note.rect.x + note.rect.width / 2.0;
        covered.push(nearest_participant(lifeline_xs, note_center));
    }

    covered
}

/// Determine note placement string from its position relative to participants.
fn placement_for_note(
    lifeline_xs: &[f64],
    note: &svg_layout::SvgNote,
    participants: &[usize],
) -> &'static str {
    if participants.len() > 1 {
        return "over";
    }
    let p_idx = participants[0];
    let lx = lifeline_xs[p_idx];
    let note_center = note.rect.x + note.rect.width / 2.0;
    if note_center < lx {
        "left_of"
    } else if note_center > lx {
        "right_of"
    } else {
        "over"
    }
}

fn build_activations(svg: &SvgSequenceLayout) -> Vec<MmdsActivation> {
    let lifeline_xs: Vec<f64> = svg.participants.iter().map(|p| p.center_x).collect();
    svg.activations
        .iter()
        .map(|a| {
            let participant = nearest_participant(&lifeline_xs, a.x);
            MmdsActivation {
                participant,
                y_start: a.y_start,
                y_end: a.y_end,
                depth: a.depth,
            }
        })
        .collect()
}

fn build_blocks(svg: &SvgSequenceLayout) -> Vec<MmdsBlock> {
    svg.blocks
        .iter()
        .map(|b: &SvgBlock| MmdsBlock {
            kind: block_kind_str(b.kind).to_string(),
            label: b.label.clone(),
            rect: MmdsRect {
                x: b.rect.x,
                y: b.rect.y,
                width: b.rect.width,
                height: b.rect.height,
            },
            dividers: b
                .dividers
                .iter()
                .map(|d| MmdsBlockDivider {
                    y: d.y,
                    kind: block_divider_kind_str(d.kind).to_string(),
                    label: d.label.clone(),
                })
                .collect(),
        })
        .collect()
}

fn build_participant_boxes(model: &Sequence, svg: &SvgSequenceLayout) -> Vec<MmdsParticipantBox> {
    model
        .participant_boxes
        .iter()
        .zip(svg.participant_boxes.iter())
        .map(|(pb, spb)| MmdsParticipantBox {
            label: pb.label.clone(),
            color: pb.color.clone(),
            participants: pb.participants.clone(),
            rect: MmdsRect {
                x: spb.rect.x,
                y: spb.rect.y,
                width: spb.rect.width,
                height: spb.rect.height,
            },
        })
        .collect()
}
