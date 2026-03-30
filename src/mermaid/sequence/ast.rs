//! Sequence diagram AST types.
//!
//! These represent the raw parsed syntax before validation/compilation
//! into the `Sequence` model used by the layout engine.
pub use crate::timeline::sequence::model::{ArrowHead, LineStyle, ParticipantKind};

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
    },
    /// `Note over A: text`.
    Note { over: String, text: String },
    /// `autonumber`.
    Autonumber,
}
