//! MMDS document types for sequence diagrams.
//!
//! Defines the serde-serializable envelope for sequence MMDS JSON. The builder
//! that populates these types from layout data lives in `runtime` (which has
//! access to both `render` and `timeline`).

use serde::Serialize;

// ---------------------------------------------------------------------------
// Document types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct SequenceDocument {
    pub version: u32,
    pub geometry_level: String,
    pub metadata: SequenceMetadata,
    // Envelope compat — always empty for sequence diagrams.
    pub nodes: Vec<()>,
    pub edges: Vec<()>,
    // Sequence-specific body.
    pub participants: Vec<MmdsParticipant>,
    pub messages: Vec<MmdsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<MmdsNote>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub activations: Vec<MmdsActivation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<MmdsBlock>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participant_boxes: Vec<MmdsParticipantBox>,
}

#[derive(Serialize)]
pub(crate) struct SequenceMetadata {
    pub diagram_type: String,
    pub bounds: MmdsBounds,
}

#[derive(Serialize)]
pub(crate) struct MmdsBounds {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsParticipant {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub position: MmdsPosition,
    pub size: MmdsSize,
    pub lifeline_x: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsMessage {
    pub id: String,
    pub from: usize,
    pub to: usize,
    pub line_style: String,
    pub arrow_head: String,
    pub text: String,
    pub y: f64,
}

#[derive(Serialize)]
pub(crate) struct MmdsNote {
    pub placement: String,
    pub participants: Vec<usize>,
    pub text: String,
    pub position: MmdsPosition,
    pub size: MmdsSize,
}

#[derive(Serialize)]
pub(crate) struct MmdsActivation {
    pub participant: usize,
    pub y_start: f64,
    pub y_end: f64,
    pub depth: usize,
}

#[derive(Serialize)]
pub(crate) struct MmdsBlockDivider {
    pub y: f64,
    pub kind: String,
    pub label: String,
}

#[derive(Serialize)]
pub(crate) struct MmdsBlock {
    pub kind: String,
    pub label: String,
    pub rect: MmdsRect,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dividers: Vec<MmdsBlockDivider>,
}

#[derive(Serialize)]
pub(crate) struct MmdsParticipantBox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub participants: Vec<usize>,
    pub rect: MmdsRect,
}

/// Serialize a pre-built sequence document to pretty-printed JSON.
pub(crate) fn serialize(document: &SequenceDocument) -> String {
    serde_json::to_string_pretty(document).expect("sequence MMDS serialization should not fail")
}
