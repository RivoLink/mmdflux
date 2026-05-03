use serde_json::Value;

use super::diff::MmdsDiffKind;

const COMMANDABLE_DIFF_KINDS: &[MmdsDiffKind] = &[
    MmdsDiffKind::GeometryLevelChanged,
    MmdsDiffKind::DirectionChanged,
    MmdsDiffKind::EngineChanged,
    MmdsDiffKind::NodeAdded,
    MmdsDiffKind::NodeRemoved,
    MmdsDiffKind::EdgeAdded,
    MmdsDiffKind::EdgeRemoved,
    MmdsDiffKind::SubgraphAdded,
    MmdsDiffKind::SubgraphRemoved,
    MmdsDiffKind::NodeLabelChanged,
    MmdsDiffKind::NodeShapeChanged,
    MmdsDiffKind::NodeParentChanged,
    MmdsDiffKind::NodeStyleChanged,
    MmdsDiffKind::EdgeReconnected,
    MmdsDiffKind::EdgeEndpointIntentChanged,
    MmdsDiffKind::EdgeLabelChanged,
    MmdsDiffKind::EdgeStyleChanged,
    MmdsDiffKind::SubgraphTitleChanged,
    MmdsDiffKind::SubgraphDirectionChanged,
    MmdsDiffKind::SubgraphParentChanged,
    MmdsDiffKind::SubgraphMembershipChanged,
    MmdsDiffKind::SubgraphVisibilityChanged,
    MmdsDiffKind::ProfileChanged,
    MmdsDiffKind::ExtensionChanged,
];

const GEOMETRY_EFFECT_DIFF_KINDS: &[MmdsDiffKind] = &[
    MmdsDiffKind::NodeMoved,
    MmdsDiffKind::NodeResized,
    MmdsDiffKind::CanvasResized,
    MmdsDiffKind::SubgraphBoundsChanged,
    MmdsDiffKind::EdgeRerouted,
    MmdsDiffKind::EndpointFaceChanged,
    MmdsDiffKind::PortIntentChanged,
    MmdsDiffKind::LabelMoved,
    MmdsDiffKind::LabelResized,
    MmdsDiffKind::LabelSideChanged,
    MmdsDiffKind::PathPortDivergenceChanged,
    MmdsDiffKind::GlobalReflowDetected,
];

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Command {
    SetGeometryLevel {
        level: String,
    },
    SetDirection {
        direction: String,
    },
    SetEngine {
        engine: Option<String>,
    },
    AddNode {
        id: String,
        label: String,
        shape: String,
        parent: Option<String>,
    },
    RemoveNode {
        id: String,
    },
    ChangeNodeLabel {
        node: String,
        label: String,
    },
    ChangeNodeShape {
        node: String,
        shape: String,
    },
    SetNodeParent {
        node: String,
        parent: Option<String>,
    },
    SetNodeStyleExtension {
        node: String,
        value: Value,
    },
    SetProfiles {
        profiles: Vec<String>,
    },
    SetExtension {
        namespace: String,
        value: Value,
    },
    AddEdge {
        id: Option<String>,
        source: String,
        target: String,
        from_subgraph: Option<String>,
        to_subgraph: Option<String>,
        label: Option<String>,
        stroke: String,
        arrow_start: String,
        arrow_end: String,
        minlen: i32,
    },
    RemoveEdge {
        edge: EdgeSelector,
    },
    ReconnectEdge {
        edge: EdgeSelector,
        source: String,
        target: String,
    },
    SetEdgeEndpointIntent {
        edge: EdgeSelector,
        from_subgraph: Option<String>,
        to_subgraph: Option<String>,
    },
    ChangeEdgeLabel {
        edge: EdgeSelector,
        label: Option<String>,
    },
    ChangeEdgeStyle {
        edge: EdgeSelector,
        stroke: Option<String>,
        arrow_start: Option<String>,
        arrow_end: Option<String>,
        minlen: Option<i32>,
    },
    AddSubgraph {
        id: String,
        title: Option<String>,
        parent: Option<String>,
        direction: Option<String>,
        children: Vec<String>,
        concurrent_regions: Vec<String>,
        invisible: bool,
    },
    RemoveSubgraph {
        id: String,
    },
    ChangeSubgraphTitle {
        subgraph: String,
        title: Option<String>,
    },
    SetSubgraphDirection {
        subgraph: String,
        direction: Option<String>,
    },
    SetSubgraphParent {
        subgraph: String,
        parent: Option<String>,
    },
    ChangeSubgraphMembership {
        subgraph: String,
        added_children: Vec<String>,
        removed_children: Vec<String>,
        added_concurrent_regions: Vec<String>,
        removed_concurrent_regions: Vec<String>,
    },
    SetSubgraphVisibility {
        subgraph: String,
        invisible: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EdgeSelector {
    Id(String),
    Semantic {
        source: String,
        target: String,
        label: Option<String>,
        stroke: Option<String>,
        arrow_start: Option<String>,
        arrow_end: Option<String>,
        minlen: Option<i32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MmdsDiffKindRole {
    Commandable,
    GeometryEffect,
}

pub(crate) fn commandable_diff_kinds() -> &'static [MmdsDiffKind] {
    COMMANDABLE_DIFF_KINDS
}

pub(crate) fn geometry_effect_diff_kinds() -> &'static [MmdsDiffKind] {
    GEOMETRY_EFFECT_DIFF_KINDS
}

pub(crate) fn diff_kind_role(kind: MmdsDiffKind) -> MmdsDiffKindRole {
    let role = match kind {
        MmdsDiffKind::GeometryLevelChanged
        | MmdsDiffKind::DirectionChanged
        | MmdsDiffKind::EngineChanged
        | MmdsDiffKind::NodeAdded
        | MmdsDiffKind::NodeRemoved
        | MmdsDiffKind::EdgeAdded
        | MmdsDiffKind::EdgeRemoved
        | MmdsDiffKind::SubgraphAdded
        | MmdsDiffKind::SubgraphRemoved
        | MmdsDiffKind::NodeLabelChanged
        | MmdsDiffKind::NodeShapeChanged
        | MmdsDiffKind::NodeParentChanged
        | MmdsDiffKind::NodeStyleChanged
        | MmdsDiffKind::EdgeReconnected
        | MmdsDiffKind::EdgeEndpointIntentChanged
        | MmdsDiffKind::EdgeLabelChanged
        | MmdsDiffKind::EdgeStyleChanged
        | MmdsDiffKind::SubgraphTitleChanged
        | MmdsDiffKind::SubgraphDirectionChanged
        | MmdsDiffKind::SubgraphParentChanged
        | MmdsDiffKind::SubgraphMembershipChanged
        | MmdsDiffKind::SubgraphVisibilityChanged
        | MmdsDiffKind::ProfileChanged
        | MmdsDiffKind::ExtensionChanged => MmdsDiffKindRole::Commandable,
        MmdsDiffKind::NodeMoved
        | MmdsDiffKind::NodeResized
        | MmdsDiffKind::CanvasResized
        | MmdsDiffKind::SubgraphBoundsChanged
        | MmdsDiffKind::EdgeRerouted
        | MmdsDiffKind::EndpointFaceChanged
        | MmdsDiffKind::PortIntentChanged
        | MmdsDiffKind::LabelMoved
        | MmdsDiffKind::LabelResized
        | MmdsDiffKind::LabelSideChanged
        | MmdsDiffKind::PathPortDivergenceChanged
        | MmdsDiffKind::GlobalReflowDetected => MmdsDiffKindRole::GeometryEffect,
    };

    debug_assert_eq!(
        kind.is_geometry_effect(),
        role == MmdsDiffKindRole::GeometryEffect
    );
    role
}

pub(crate) fn diff_kind_for_command(command: &Command) -> MmdsDiffKind {
    match command {
        Command::SetGeometryLevel { .. } => MmdsDiffKind::GeometryLevelChanged,
        Command::SetDirection { .. } => MmdsDiffKind::DirectionChanged,
        Command::SetEngine { .. } => MmdsDiffKind::EngineChanged,
        Command::AddNode { .. } => MmdsDiffKind::NodeAdded,
        Command::RemoveNode { .. } => MmdsDiffKind::NodeRemoved,
        Command::ChangeNodeLabel { .. } => MmdsDiffKind::NodeLabelChanged,
        Command::ChangeNodeShape { .. } => MmdsDiffKind::NodeShapeChanged,
        Command::SetNodeParent { .. } => MmdsDiffKind::NodeParentChanged,
        Command::SetNodeStyleExtension { .. } => MmdsDiffKind::NodeStyleChanged,
        Command::SetProfiles { .. } => MmdsDiffKind::ProfileChanged,
        Command::SetExtension { .. } => MmdsDiffKind::ExtensionChanged,
        Command::AddEdge { .. } => MmdsDiffKind::EdgeAdded,
        Command::RemoveEdge { .. } => MmdsDiffKind::EdgeRemoved,
        Command::ReconnectEdge { .. } => MmdsDiffKind::EdgeReconnected,
        Command::SetEdgeEndpointIntent { .. } => MmdsDiffKind::EdgeEndpointIntentChanged,
        Command::ChangeEdgeLabel { .. } => MmdsDiffKind::EdgeLabelChanged,
        Command::ChangeEdgeStyle { .. } => MmdsDiffKind::EdgeStyleChanged,
        Command::AddSubgraph { .. } => MmdsDiffKind::SubgraphAdded,
        Command::RemoveSubgraph { .. } => MmdsDiffKind::SubgraphRemoved,
        Command::ChangeSubgraphTitle { .. } => MmdsDiffKind::SubgraphTitleChanged,
        Command::SetSubgraphDirection { .. } => MmdsDiffKind::SubgraphDirectionChanged,
        Command::SetSubgraphParent { .. } => MmdsDiffKind::SubgraphParentChanged,
        Command::ChangeSubgraphMembership { .. } => MmdsDiffKind::SubgraphMembershipChanged,
        Command::SetSubgraphVisibility { .. } => MmdsDiffKind::SubgraphVisibilityChanged,
    }
}
