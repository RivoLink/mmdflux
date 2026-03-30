//! Sequence diagram AST types.
//!
//! These represent the raw parsed syntax before validation/compilation
//! into the `Sequence` model used by the layout engine.
pub use crate::timeline::sequence::model::{ArrowHead, LineStyle, NotePlacement, ParticipantKind};

/// Activation modifier from `+`/`-` shorthand on message arrows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivationModifier {
    /// `+` on arrow: activate the target participant.
    Activate,
    /// `-` on arrow: deactivate the target participant.
    Deactivate,
}

/// A parsed sequence diagram statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequenceStatement {
    /// `participant A` or `participant A as Alice`.
    Participant {
        kind: ParticipantKind,
        id: String,
        alias: Option<String>,
    },
    /// A message between participants (e.g., `A->>B: hello`).
    Message {
        from: String,
        to: String,
        line_style: LineStyle,
        arrow_head: ArrowHead,
        text: String,
        /// Optional activation modifier from `+`/`-` shorthand.
        activate: Option<ActivationModifier>,
    },
    /// `Note over A: text`, `Note left of A: text`, `Note right of A: text`,
    /// or `Note over A,B: text` (spanning).
    Note {
        placement: NotePlacement,
        participants: Vec<String>,
        text: String,
    },
    /// `activate <participant>`.
    Activate { participant: String },
    /// `deactivate <participant>`.
    Deactivate { participant: String },
    /// `autonumber`.
    Autonumber,
}
