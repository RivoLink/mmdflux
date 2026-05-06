//! Runtime rendering for timeline-family payloads (sequence diagrams).
//!
//! Parallel to `graph_family` — computes SVG layout and serializes to MMDS
//! JSON via the types defined in `mmds::sequence`.

use crate::graph::GeometryLevel;
use crate::graph::measure::ProportionalTextMetrics;
use crate::mmds::sequence::{
    self, MmdsActivation, MmdsArrowHead, MmdsBlock, MmdsBlockDivider, MmdsBlockDividerKind,
    MmdsBlockKind, MmdsBounds, MmdsLineStyle, MmdsMessage, MmdsNote, MmdsNotePlacement,
    MmdsParticipant, MmdsParticipantBox, MmdsParticipantKind, MmdsPosition, MmdsRect, MmdsSize,
    SequenceDiagramType, SequenceDocument, SequenceMetadata,
};
use crate::render::timeline::svg_layout::{
    self as svg_layout, SvgBlock, SvgRow, SvgSequenceLayout,
};
use crate::timeline::sequence::model::{
    ArrowHead, BlockDividerKind, BlockKind, LineStyle, ParticipantKind, Sequence,
};

// ---------------------------------------------------------------------------
// MMDS vocabulary conversion helpers
// ---------------------------------------------------------------------------

fn mmds_line_style(s: LineStyle) -> MmdsLineStyle {
    match s {
        LineStyle::Solid => MmdsLineStyle::Solid,
        LineStyle::Dashed => MmdsLineStyle::Dashed,
    }
}

fn mmds_arrow_head(a: ArrowHead) -> MmdsArrowHead {
    match a {
        ArrowHead::None => MmdsArrowHead::None,
        ArrowHead::Filled => MmdsArrowHead::Filled,
        ArrowHead::Cross => MmdsArrowHead::Cross,
        ArrowHead::Async => MmdsArrowHead::Async,
    }
}

fn mmds_participant_kind(k: &ParticipantKind) -> MmdsParticipantKind {
    match k {
        ParticipantKind::Participant => MmdsParticipantKind::Participant,
        ParticipantKind::Actor => MmdsParticipantKind::Actor,
    }
}

fn mmds_block_kind(k: BlockKind) -> MmdsBlockKind {
    match k {
        BlockKind::Loop => MmdsBlockKind::Loop,
        BlockKind::Alt => MmdsBlockKind::Alt,
        BlockKind::Opt => MmdsBlockKind::Opt,
        BlockKind::Par => MmdsBlockKind::Par,
        BlockKind::Critical => MmdsBlockKind::Critical,
        BlockKind::Break => MmdsBlockKind::Break,
        BlockKind::Rect => MmdsBlockKind::Rect,
    }
}

fn mmds_block_divider_kind(k: BlockDividerKind) -> MmdsBlockDividerKind {
    match k {
        BlockDividerKind::Else => MmdsBlockDividerKind::Else,
        BlockDividerKind::And => MmdsBlockDividerKind::And,
        BlockDividerKind::Option => MmdsBlockDividerKind::Option,
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Serialize a sequence diagram model to MMDS JSON using proportional layout
/// positions from the SVG layout engine.
pub(in crate::runtime) fn to_json(model: &Sequence, metrics: &ProportionalTextMetrics) -> String {
    let svg = svg_layout::layout(model, metrics, "sans-serif");
    let document = build_document(model, &svg);
    sequence::serialize(&document)
}

fn build_document(model: &Sequence, svg: &SvgSequenceLayout) -> SequenceDocument {
    let participants = build_participants(model, svg);
    let (messages, notes) = build_messages_and_notes(svg);
    let activations = build_activations(svg);
    let blocks = build_blocks(svg);
    let participant_boxes = build_participant_boxes(model, svg);

    SequenceDocument {
        version: 1,
        geometry_level: GeometryLevel::Layout,
        metadata: SequenceMetadata {
            diagram_type: SequenceDiagramType::Sequence,
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
            kind: mmds_participant_kind(&p.kind),
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
                    line_style: mmds_line_style(m.line_style),
                    arrow_head: mmds_arrow_head(m.arrow_head),
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
                    line_style: mmds_line_style(sm.line_style),
                    arrow_head: mmds_arrow_head(sm.arrow_head),
                    text: sm.label.clone(),
                    y: sm.y,
                });
                msg_idx += 1;
            }
            SvgRow::Note(n) => {
                let participants = participants_for_note(&lifeline_xs, n);
                let placement = placement_for_note(&lifeline_xs, n, &participants);
                notes.push(MmdsNote {
                    placement,
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
) -> MmdsNotePlacement {
    if participants.len() > 1 {
        return MmdsNotePlacement::Over;
    }
    let p_idx = participants[0];
    let lx = lifeline_xs[p_idx];
    let note_center = note.rect.x + note.rect.width / 2.0;
    if note_center < lx {
        MmdsNotePlacement::LeftOf
    } else if note_center > lx {
        MmdsNotePlacement::RightOf
    } else {
        MmdsNotePlacement::Over
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
            kind: mmds_block_kind(b.kind),
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
                    kind: mmds_block_divider_kind(d.kind),
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
