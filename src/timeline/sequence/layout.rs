//! Shared timeline layout engine for sequence diagrams.
//!
//! Computes character-grid positions for participants, messages,
//! and notes. Output is consumed by the text renderer.

use super::model::{
    ArrowHead, LineStyle, NotePlacement, Participant, ParticipantKind, Sequence, SequenceEvent,
};

/// Minimum gap between participant centers (characters).
const MIN_PARTICIPANT_GAP: usize = 20;

/// Extra padding around message labels.
pub(crate) const LABEL_PADDING: usize = 4;

/// Height of the participant header box (top border + label + bottom border).
pub(crate) const HEADER_HEIGHT: usize = 3;

/// Vertical gap between events.
pub(crate) const EVENT_GAP: usize = 1;

/// Height of a self-message (outgoing + vertical + return).
pub(crate) const SELF_MSG_HEIGHT: usize = 3;

/// Width of the self-message loop arm.
pub const SELF_MSG_WIDTH: usize = 4;

/// Layout data for one participant.
#[derive(Debug, Clone)]
pub struct ParticipantLayout {
    /// Center X column (where the lifeline is drawn).
    pub center_x: usize,
    /// Left X of the header box.
    pub box_x: usize,
    /// Width of the header box.
    pub box_width: usize,
    /// Display label.
    pub label: String,
    /// Participant or Actor.
    #[allow(dead_code)]
    pub kind: ParticipantKind,
}

/// A positioned event row in the layout.
#[derive(Debug, Clone)]
pub enum RowLayout {
    /// A message arrow between (or within) lifelines.
    Message {
        /// Y row where the arrow is drawn.
        y: usize,
        /// Source participant index.
        from_idx: usize,
        /// Target participant index.
        to_idx: usize,
        /// Solid or dashed line.
        line_style: LineStyle,
        /// Arrowhead shape.
        arrow_head: ArrowHead,
        /// Label text.
        text: String,
        /// Autonumber prefix if any.
        number: Option<usize>,
    },
    /// A note box positioned relative to participant(s).
    Note {
        /// Y row of the note top border.
        y: usize,
        /// Note placement mode.
        placement: NotePlacement,
        /// Participant indices (1 for left/right/over-single, 2 for spanning).
        participant_indices: Vec<usize>,
        /// Note text.
        text: String,
    },
}

/// An activation rectangle on a participant's lifeline.
#[derive(Debug, Clone)]
pub struct ActivationRect {
    /// Participant index.
    pub participant_idx: usize,
    /// Y row where the activation starts.
    pub y_start: usize,
    /// Y row where the activation ends (inclusive).
    pub y_end: usize,
    /// Nesting depth (0-based). Deeper activations render slightly offset.
    pub depth: usize,
}

/// Complete sequence diagram layout.
#[derive(Debug, Clone)]
pub struct SequenceLayout {
    /// Participant positions.
    pub participants: Vec<ParticipantLayout>,
    /// Positioned event rows.
    pub rows: Vec<RowLayout>,
    /// Activation rectangles to render on lifelines.
    pub activations: Vec<ActivationRect>,
    /// Total canvas width.
    pub width: usize,
    /// Total canvas height.
    pub height: usize,
}

/// Compute layout for a sequence model.
pub fn layout(model: &Sequence) -> SequenceLayout {
    if model.participants.is_empty() {
        return SequenceLayout {
            participants: Vec::new(),
            rows: Vec::new(),
            activations: Vec::new(),
            width: 0,
            height: 0,
        };
    }

    let participant_gap = compute_participant_gap(model);
    let left_margin = compute_left_note_margin(model);
    let participants = layout_participants(&model.participants, participant_gap, left_margin);

    let mut rows = Vec::new();
    let mut cursor_y = HEADER_HEIGHT + EVENT_GAP;

    // Activation tracking: stack of (y_start, depth) per participant
    let num_participants = model.participants.len();
    let mut activation_stacks: Vec<Vec<(usize, usize)>> = vec![Vec::new(); num_participants];
    let mut activation_depth: Vec<usize> = vec![0; num_participants];
    let mut activations: Vec<ActivationRect> = Vec::new();
    // Track the Y of the last message row so activation start/end aligns with
    // the triggering message rather than the cursor after it.
    let mut last_message_y = cursor_y;

    for event in &model.events {
        match event {
            SequenceEvent::Message {
                from,
                to,
                line_style,
                arrow_head,
                text,
                number,
            } => {
                let is_self = from == to;
                last_message_y = cursor_y;
                if is_self {
                    rows.push(RowLayout::Message {
                        y: cursor_y,
                        from_idx: *from,
                        to_idx: *to,
                        line_style: *line_style,
                        arrow_head: *arrow_head,
                        text: text.clone(),
                        number: *number,
                    });
                    cursor_y += SELF_MSG_HEIGHT + EVENT_GAP;
                } else {
                    rows.push(RowLayout::Message {
                        y: cursor_y,
                        from_idx: *from,
                        to_idx: *to,
                        line_style: *line_style,
                        arrow_head: *arrow_head,
                        text: text.clone(),
                        number: *number,
                    });
                    cursor_y += 1 + EVENT_GAP;
                }
            }
            SequenceEvent::Note {
                placement,
                participants: indices,
                text,
            } => {
                rows.push(RowLayout::Note {
                    y: cursor_y,
                    placement: *placement,
                    participant_indices: indices.clone(),
                    text: text.clone(),
                });
                cursor_y += 3 + EVENT_GAP;
            }
            SequenceEvent::ActivateStart { participant } => {
                let depth = activation_depth[*participant];
                activation_stacks[*participant].push((last_message_y, depth));
                activation_depth[*participant] += 1;
            }
            SequenceEvent::ActivateEnd { participant } => {
                if let Some((y_start, depth)) = activation_stacks[*participant].pop() {
                    // End at the last message row (inclusive).
                    let y_end = last_message_y.max(y_start);
                    activations.push(ActivationRect {
                        participant_idx: *participant,
                        y_start,
                        y_end,
                        depth,
                    });
                    activation_depth[*participant] =
                        activation_depth[*participant].saturating_sub(1);
                }
            }
        }
    }

    // Close any unclosed activations at the bottom of the diagram
    for (pidx, stack) in activation_stacks.iter_mut().enumerate() {
        while let Some((y_start, depth)) = stack.pop() {
            let y_end = cursor_y.saturating_sub(1).max(y_start);
            activations.push(ActivationRect {
                participant_idx: pidx,
                y_start,
                y_end,
                depth,
            });
        }
    }

    // Compute total dimensions
    let max_participant_right = participants
        .iter()
        .map(|p| p.box_x + p.box_width)
        .max()
        .unwrap_or(0);

    // Account for self-message loops + labels extending right
    let self_msg_extra = model
        .events
        .iter()
        .filter_map(|e| match e {
            SequenceEvent::Message {
                from,
                to,
                text,
                number,
                ..
            } if from == to => {
                let prefix_len = number.map(|n| format!("{n}. ").len()).unwrap_or(0);
                let label_len = text.len() + prefix_len;
                // arm (SELF_MSG_WIDTH) + gap (2) + label
                Some(SELF_MSG_WIDTH + 2 + label_len)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0);

    // Account for note boxes extending right
    let note_extra = model
        .events
        .iter()
        .filter_map(|e| match e {
            SequenceEvent::Note {
                placement,
                participants: indices,
                text,
            } => {
                let box_width = text.len() + 4;
                let box_right = match placement {
                    NotePlacement::LeftOf => {
                        // Box right edge at center_x - 1
                        participants[indices[0]].center_x
                    }
                    NotePlacement::RightOf => {
                        let center_x = participants[indices[0]].center_x;
                        center_x + 1 + box_width
                    }
                    NotePlacement::Over if indices.len() == 2 => {
                        let cx1 = participants[indices[0]].center_x;
                        let cx2 = participants[indices[1]].center_x;
                        let mid = (cx1 + cx2) / 2;
                        let span_width = box_width.max(cx1.abs_diff(cx2) + 4);
                        mid.saturating_sub(span_width / 2) + span_width
                    }
                    NotePlacement::Over => {
                        let center_x = participants[indices[0]].center_x;
                        center_x.saturating_sub(box_width / 2) + box_width
                    }
                };
                Some(box_right)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0);

    let base_width = max_participant_right.max(note_extra);
    let width = base_width + self_msg_extra + 2; // +2 for right margin
    let height = cursor_y;

    SequenceLayout {
        participants,
        rows,
        activations,
        width,
        height,
    }
}

/// Compute the gap between participant centers based on message labels.
fn compute_participant_gap(model: &Sequence) -> usize {
    let max_label_len = model
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
                let prefix_len = number.map(|n| format!("{n}. ").len()).unwrap_or(0);
                Some(text.len() + prefix_len)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0);

    (max_label_len + LABEL_PADDING).max(MIN_PARTICIPANT_GAP)
}

/// Compute extra left margin needed for left-of notes.
///
/// A left-of note on the first participant needs space to the left of the
/// lifeline. Without this, the note box would overflow into negative x
/// (clamped to 0 by saturating_sub), overlapping the diagram.
fn compute_left_note_margin(model: &Sequence) -> usize {
    // Tentative center_x for each participant with default margin of 1
    let mut centers = Vec::with_capacity(model.participants.len());
    let mut x = 1usize;
    for (i, p) in model.participants.iter().enumerate() {
        let box_width = p.label.len() + 4;
        centers.push(x + box_width / 2);
        if i < model.participants.len() - 1 {
            let next_bw = model.participants[i + 1].label.len() + 4;
            x = centers[i] + MIN_PARTICIPANT_GAP - next_bw / 2;
        }
    }

    let mut max_overhang = 0usize;
    for event in &model.events {
        if let SequenceEvent::Note {
            placement: NotePlacement::LeftOf,
            participants: indices,
            text,
        } = event
        {
            let box_width = text.len() + 4;
            let center_x = centers[indices[0]];
            // The renderer places the box at center_x - (box_width + 1)
            let needed = box_width + 1;
            if needed > center_x {
                max_overhang = max_overhang.max(needed - center_x);
            }
        }
    }
    max_overhang
}

/// Compute horizontal positions for all participants.
fn layout_participants(
    participants: &[Participant],
    gap: usize,
    left_margin: usize,
) -> Vec<ParticipantLayout> {
    let mut result = Vec::with_capacity(participants.len());
    let mut x = 1 + left_margin; // left margin + space for left-of notes

    for (i, p) in participants.iter().enumerate() {
        let box_width = p.label.len() + 4; // | + space + label + space + |
        let center_x = x + box_width / 2;

        result.push(ParticipantLayout {
            center_x,
            box_x: x,
            box_width,
            label: p.label.clone(),
            kind: p.kind.clone(),
        });

        if i < participants.len() - 1 {
            // Next participant starts at center_x + gap - half of next box
            let next_label_len = participants[i + 1].label.len();
            let next_box_width = next_label_len + 4;
            x = center_x + gap - next_box_width / 2;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_empty_model() {
        let layout = layout(&Sequence {
            participants: Vec::new(),
            events: Vec::new(),
            autonumber: false,
        });
        assert!(layout.participants.is_empty());
        assert!(layout.rows.is_empty());
        assert_eq!(layout.width, 0);
        assert_eq!(layout.height, 0);
    }
}
