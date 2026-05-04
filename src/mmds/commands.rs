use serde_json::Value;

use super::diff::{MmdsDiffEvent, MmdsDiffKind, MmdsDiffSubject};
use super::{Document, Edge, NODE_STYLE_EXTENSION_NAMESPACE, Node, Position, Size, Subgraph};
use crate::engines::graph::EngineAlgorithmId;
use crate::graph::GeometryLevel;
use crate::{OutputFormat, RenderConfig};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandApplyError {
    NodeNotFound {
        id: String,
    },
    SubgraphNotFound {
        id: String,
    },
    SubjectAlreadyExists {
        id: String,
    },
    EdgeSelectorNoMatch {
        selector: Box<EdgeSelector>,
    },
    EdgeSelectorAmbiguous {
        selector: Box<EdgeSelector>,
        matches: Vec<String>,
    },
    AddEdgeIdCollision {
        id: String,
    },
    AddEdgeIdUnsupported {
        id: String,
        expected: String,
    },
    RelayoutFailed {
        stage: String,
        source: String,
    },
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

pub(crate) fn apply(
    command: &Command,
    output: &mut Document,
) -> Result<Vec<MmdsDiffEvent>, CommandApplyError> {
    match command {
        Command::RemoveNode { id } => {
            node_index(output, id)?;
            apply_with_relayout_event(
                output,
                node_event(MmdsDiffKind::NodeRemoved, id.clone()),
                |candidate| {
                    candidate.nodes.retain(|node| node.id != *id);
                    candidate
                        .edges
                        .retain(|edge| edge.source != *id && edge.target != *id);
                    for subgraph in &mut candidate.subgraphs {
                        subgraph.children.retain(|child| child != id);
                        subgraph.concurrent_regions.retain(|region| region != id);
                    }
                    Ok(())
                },
            )
        }
        Command::ChangeEdgeLabel { edge, label } => {
            let edge_index = resolve_edge_index(output, edge)?;
            let edge_id = output.edges[edge_index].id.clone();
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeLabelChanged, edge_id),
                |candidate| {
                    let edge_index = resolve_edge_index(candidate, edge)?;
                    candidate.edges[edge_index].label = label.clone();
                    Ok(())
                },
            )
        }
        Command::AddEdge {
            id,
            source,
            target,
            from_subgraph,
            to_subgraph,
            label,
            stroke,
            arrow_start,
            arrow_end,
            minlen,
        } => {
            let edge_id = resolve_new_edge_id(output, id.as_deref())?;
            node_index(output, source)?;
            node_index(output, target)?;
            if let Some(from_subgraph) = from_subgraph {
                subgraph_index(output, from_subgraph)?;
            }
            if let Some(to_subgraph) = to_subgraph {
                subgraph_index(output, to_subgraph)?;
            }
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeAdded, edge_id.clone()),
                |candidate| {
                    candidate.edges.push(Edge {
                        id: edge_id,
                        source: source.clone(),
                        target: target.clone(),
                        from_subgraph: from_subgraph.clone(),
                        to_subgraph: to_subgraph.clone(),
                        label: label.clone(),
                        stroke: stroke.clone(),
                        arrow_start: arrow_start.clone(),
                        arrow_end: arrow_end.clone(),
                        minlen: *minlen,
                        path: None,
                        label_position: None,
                        is_backward: None,
                        source_port: None,
                        target_port: None,
                        label_side: None,
                        label_rect: None,
                    });
                    Ok(())
                },
            )
        }
        Command::RemoveEdge { edge } => {
            let edge_index = resolve_edge_index(output, edge)?;
            let edge_id = output.edges[edge_index].id.clone();
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeRemoved, edge_id),
                |candidate| {
                    let edge_index = resolve_edge_index(candidate, edge)?;
                    candidate.edges.remove(edge_index);
                    Ok(())
                },
            )
        }
        Command::ReconnectEdge {
            edge,
            source,
            target,
        } => {
            let edge_index = resolve_edge_index(output, edge)?;
            let edge_id = output.edges[edge_index].id.clone();
            node_index(output, source)?;
            node_index(output, target)?;
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeReconnected, edge_id),
                |candidate| {
                    let edge_index = resolve_edge_index(candidate, edge)?;
                    candidate.edges[edge_index].source = source.clone();
                    candidate.edges[edge_index].target = target.clone();
                    Ok(())
                },
            )
        }
        Command::SetEdgeEndpointIntent {
            edge,
            from_subgraph,
            to_subgraph,
        } => {
            let edge_index = resolve_edge_index(output, edge)?;
            let edge_id = output.edges[edge_index].id.clone();
            if let Some(from_subgraph) = from_subgraph {
                subgraph_index(output, from_subgraph)?;
            }
            if let Some(to_subgraph) = to_subgraph {
                subgraph_index(output, to_subgraph)?;
            }
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeEndpointIntentChanged, edge_id),
                |candidate| {
                    let edge_index = resolve_edge_index(candidate, edge)?;
                    candidate.edges[edge_index].from_subgraph = from_subgraph.clone();
                    candidate.edges[edge_index].to_subgraph = to_subgraph.clone();
                    Ok(())
                },
            )
        }
        Command::ChangeEdgeStyle {
            edge,
            stroke,
            arrow_start,
            arrow_end,
            minlen,
        } => {
            let edge_index = resolve_edge_index(output, edge)?;
            let edge_id = output.edges[edge_index].id.clone();
            apply_with_relayout_event(
                output,
                edge_event(MmdsDiffKind::EdgeStyleChanged, edge_id),
                |candidate| {
                    let edge_index = resolve_edge_index(candidate, edge)?;
                    if let Some(stroke) = stroke {
                        candidate.edges[edge_index].stroke = stroke.clone();
                    }
                    if let Some(arrow_start) = arrow_start {
                        candidate.edges[edge_index].arrow_start = arrow_start.clone();
                    }
                    if let Some(arrow_end) = arrow_end {
                        candidate.edges[edge_index].arrow_end = arrow_end.clone();
                    }
                    if let Some(minlen) = minlen {
                        candidate.edges[edge_index].minlen = *minlen;
                    }
                    Ok(())
                },
            )
        }
        Command::SetGeometryLevel { level } => {
            apply_with_relayout(output, MmdsDiffKind::GeometryLevelChanged, |candidate| {
                candidate.geometry_level = level.clone();
                Ok(())
            })
        }
        Command::SetDirection { direction } => {
            apply_with_relayout(output, MmdsDiffKind::DirectionChanged, |candidate| {
                candidate.metadata.direction = direction.clone();
                Ok(())
            })
        }
        Command::SetEngine { engine } => {
            apply_with_relayout(output, MmdsDiffKind::EngineChanged, |candidate| {
                candidate.metadata.engine = engine.clone();
                Ok(())
            })
        }
        Command::AddNode {
            id,
            label,
            shape,
            parent,
        } => {
            if output.nodes.iter().any(|node| node.id == *id) {
                return Err(CommandApplyError::SubjectAlreadyExists { id: id.clone() });
            }
            if let Some(parent) = parent {
                subgraph_index(output, parent)?;
            }
            apply_with_relayout_event(
                output,
                node_event(MmdsDiffKind::NodeAdded, id.clone()),
                |candidate| {
                    candidate.nodes.push(Node {
                        id: id.clone(),
                        label: label.clone(),
                        shape: shape.clone(),
                        parent: parent.clone(),
                        position: Position { x: 0.0, y: 0.0 },
                        size: Size {
                            width: 0.0,
                            height: 0.0,
                        },
                    });
                    Ok(())
                },
            )
        }
        Command::ChangeNodeLabel { node, label } => {
            node_index(output, node)?;
            apply_with_relayout_event(
                output,
                node_event(MmdsDiffKind::NodeLabelChanged, node.clone()),
                |candidate| {
                    let index = node_index(candidate, node)?;
                    candidate.nodes[index].label = label.clone();
                    Ok(())
                },
            )
        }
        Command::ChangeNodeShape { node, shape } => {
            node_index(output, node)?;
            apply_with_relayout_event(
                output,
                node_event(MmdsDiffKind::NodeShapeChanged, node.clone()),
                |candidate| {
                    let index = node_index(candidate, node)?;
                    candidate.nodes[index].shape = shape.clone();
                    Ok(())
                },
            )
        }
        Command::SetNodeParent { node, parent } => {
            node_index(output, node)?;
            if let Some(parent) = parent {
                subgraph_index(output, parent)?;
            }
            apply_with_relayout_event(
                output,
                node_event(MmdsDiffKind::NodeParentChanged, node.clone()),
                |candidate| {
                    let index = node_index(candidate, node)?;
                    candidate.nodes[index].parent = parent.clone();
                    Ok(())
                },
            )
        }
        Command::SetProfiles { profiles } => {
            output.profiles = profiles.clone();
            Ok(vec![document_event(MmdsDiffKind::ProfileChanged)])
        }
        Command::SetExtension { namespace, value } => {
            output.extensions.insert(
                namespace.clone(),
                value.as_object().cloned().unwrap_or_else(|| {
                    let mut payload = serde_json::Map::new();
                    payload.insert("value".to_string(), value.clone());
                    payload
                }),
            );
            Ok(vec![document_event(MmdsDiffKind::ExtensionChanged)])
        }
        Command::SetNodeStyleExtension { node, value } => {
            if !output.nodes.iter().any(|candidate| candidate.id == *node) {
                return Err(CommandApplyError::NodeNotFound { id: node.clone() });
            }
            set_node_style_extension(output, node, value.clone());
            Ok(vec![MmdsDiffEvent {
                kind: MmdsDiffKind::NodeStyleChanged,
                subject: MmdsDiffSubject::Node(node.clone()),
                evidence: Vec::new(),
                related_event_ids: Vec::new(),
            }])
        }
        Command::ChangeSubgraphTitle { subgraph, title } => {
            let subgraph_index = output
                .subgraphs
                .iter()
                .position(|candidate| candidate.id == *subgraph)
                .ok_or_else(|| CommandApplyError::SubgraphNotFound {
                    id: subgraph.clone(),
                })?;
            output.subgraphs[subgraph_index].title = title
                .clone()
                .unwrap_or_else(|| output.subgraphs[subgraph_index].id.clone());
            Ok(vec![MmdsDiffEvent {
                kind: MmdsDiffKind::SubgraphTitleChanged,
                subject: MmdsDiffSubject::Subgraph(subgraph.clone()),
                evidence: Vec::new(),
                related_event_ids: Vec::new(),
            }])
        }
        Command::AddSubgraph {
            id,
            title,
            parent,
            direction,
            children,
            concurrent_regions,
            invisible,
        } => {
            if output.subgraphs.iter().any(|subgraph| subgraph.id == *id) {
                return Err(CommandApplyError::SubjectAlreadyExists { id: id.clone() });
            }
            if let Some(parent) = parent {
                subgraph_index(output, parent)?;
            }
            for child in children {
                node_index(output, child)?;
            }
            for region in concurrent_regions {
                subgraph_index(output, region)?;
            }
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphAdded, id.clone()),
                |candidate| {
                    sync_subgraph_children_from_node_parents(candidate);
                    candidate.subgraphs.push(Subgraph {
                        id: id.clone(),
                        title: title.clone().unwrap_or_else(|| id.clone()),
                        children: Vec::new(),
                        parent: parent.clone(),
                        direction: direction.clone(),
                        bounds: None,
                        invisible: *invisible,
                        concurrent_regions: concurrent_regions.clone(),
                    });
                    for child in children {
                        let node_index = node_index(candidate, child)?;
                        candidate.nodes[node_index].parent = Some(id.clone());
                    }
                    for region in concurrent_regions {
                        let region_index = subgraph_index(candidate, region)?;
                        candidate.subgraphs[region_index].parent = Some(id.clone());
                    }
                    sync_subgraph_children_from_node_parents(candidate);
                    Ok(())
                },
            )
        }
        Command::RemoveSubgraph { id } => {
            subgraph_index(output, id)?;
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphRemoved, id.clone()),
                |candidate| {
                    candidate.subgraphs.retain(|subgraph| subgraph.id != *id);
                    for node in &mut candidate.nodes {
                        if node.parent.as_deref() == Some(id.as_str()) {
                            node.parent = None;
                        }
                    }
                    for subgraph in &mut candidate.subgraphs {
                        if subgraph.parent.as_deref() == Some(id.as_str()) {
                            subgraph.parent = None;
                        }
                        subgraph.concurrent_regions.retain(|region| region != id);
                    }
                    sync_subgraph_children_from_node_parents(candidate);
                    Ok(())
                },
            )
        }
        Command::SetSubgraphDirection {
            subgraph,
            direction,
        } => {
            subgraph_index(output, subgraph)?;
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphDirectionChanged, subgraph.clone()),
                |candidate| {
                    let subgraph_index = subgraph_index(candidate, subgraph)?;
                    candidate.subgraphs[subgraph_index].direction = direction.clone();
                    Ok(())
                },
            )
        }
        Command::SetSubgraphParent { subgraph, parent } => {
            subgraph_index(output, subgraph)?;
            if let Some(parent) = parent {
                subgraph_index(output, parent)?;
            }
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphParentChanged, subgraph.clone()),
                |candidate| {
                    let subgraph_index = subgraph_index(candidate, subgraph)?;
                    candidate.subgraphs[subgraph_index].parent = parent.clone();
                    Ok(())
                },
            )
        }
        Command::ChangeSubgraphMembership {
            subgraph,
            added_children,
            removed_children,
            added_concurrent_regions,
            removed_concurrent_regions,
        } => {
            subgraph_index(output, subgraph)?;
            for child in added_children.iter().chain(removed_children.iter()) {
                node_index(output, child)?;
            }
            for region in added_concurrent_regions
                .iter()
                .chain(removed_concurrent_regions.iter())
            {
                subgraph_index(output, region)?;
            }
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphMembershipChanged, subgraph.clone()),
                |candidate| {
                    sync_subgraph_children_from_node_parents(candidate);
                    for child in removed_children {
                        let node_index = node_index(candidate, child)?;
                        if candidate.nodes[node_index].parent.as_deref() == Some(subgraph.as_str())
                        {
                            candidate.nodes[node_index].parent = None;
                        }
                    }
                    for child in added_children {
                        let node_index = node_index(candidate, child)?;
                        candidate.nodes[node_index].parent = Some(subgraph.clone());
                    }
                    let target_subgraph_index = subgraph_index(candidate, subgraph)?;
                    for region in removed_concurrent_regions {
                        candidate.subgraphs[target_subgraph_index]
                            .concurrent_regions
                            .retain(|candidate| candidate != region);
                        let region_index = subgraph_index(candidate, region)?;
                        if candidate.subgraphs[region_index].parent.as_deref()
                            == Some(subgraph.as_str())
                        {
                            candidate.subgraphs[region_index].parent = None;
                        }
                    }
                    for region in added_concurrent_regions {
                        if !candidate.subgraphs[target_subgraph_index]
                            .concurrent_regions
                            .contains(region)
                        {
                            candidate.subgraphs[target_subgraph_index]
                                .concurrent_regions
                                .push(region.clone());
                        }
                        let region_index = subgraph_index(candidate, region)?;
                        candidate.subgraphs[region_index].parent = Some(subgraph.clone());
                    }
                    sync_subgraph_children_from_node_parents(candidate);
                    Ok(())
                },
            )
        }
        Command::SetSubgraphVisibility {
            subgraph,
            invisible,
        } => {
            subgraph_index(output, subgraph)?;
            apply_with_relayout_event(
                output,
                subgraph_event(MmdsDiffKind::SubgraphVisibilityChanged, subgraph.clone()),
                |candidate| {
                    let subgraph_index = subgraph_index(candidate, subgraph)?;
                    candidate.subgraphs[subgraph_index].invisible = *invisible;
                    Ok(())
                },
            )
        }
    }
}

fn apply_with_relayout(
    output: &mut Document,
    kind: MmdsDiffKind,
    mutate: impl FnOnce(&mut Document) -> Result<(), CommandApplyError>,
) -> Result<Vec<MmdsDiffEvent>, CommandApplyError> {
    apply_with_relayout_event(output, document_event(kind), mutate)
}

fn apply_with_relayout_event(
    output: &mut Document,
    event: MmdsDiffEvent,
    mutate: impl FnOnce(&mut Document) -> Result<(), CommandApplyError>,
) -> Result<Vec<MmdsDiffEvent>, CommandApplyError> {
    let mut candidate = output.clone();
    mutate(&mut candidate)?;
    let relaid = relayout_output_for_command_apply(&candidate)?;
    *output = relaid;
    Ok(vec![event])
}

fn set_node_style_extension(output: &mut Document, node: &str, value: Value) {
    let extension = output
        .extensions
        .entry(NODE_STYLE_EXTENSION_NAMESPACE.to_string())
        .or_default();
    let nodes = extension
        .entry("nodes".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !nodes.is_object() {
        *nodes = Value::Object(serde_json::Map::new());
    }
    nodes
        .as_object_mut()
        .expect("nodes style extension should be an object")
        .insert(node.to_string(), value);
}

fn document_event(kind: MmdsDiffKind) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Document,
        evidence: Vec::new(),
        related_event_ids: Vec::new(),
    }
}

fn node_event(kind: MmdsDiffKind, id: String) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Node(id),
        evidence: Vec::new(),
        related_event_ids: Vec::new(),
    }
}

fn edge_event(kind: MmdsDiffKind, id: String) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Edge(id),
        evidence: Vec::new(),
        related_event_ids: Vec::new(),
    }
}

fn subgraph_event(kind: MmdsDiffKind, id: String) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Subgraph(id),
        evidence: Vec::new(),
        related_event_ids: Vec::new(),
    }
}

fn node_index(output: &Document, id: &str) -> Result<usize, CommandApplyError> {
    output
        .nodes
        .iter()
        .position(|node| node.id == id)
        .ok_or_else(|| CommandApplyError::NodeNotFound { id: id.to_string() })
}

fn subgraph_index(output: &Document, id: &str) -> Result<usize, CommandApplyError> {
    output
        .subgraphs
        .iter()
        .position(|subgraph| subgraph.id == id)
        .ok_or_else(|| CommandApplyError::SubgraphNotFound { id: id.to_string() })
}

fn sync_subgraph_children_from_node_parents(output: &mut Document) {
    for subgraph in &mut output.subgraphs {
        subgraph.children.clear();
    }

    let memberships: Vec<(String, String)> = output
        .nodes
        .iter()
        .filter_map(|node| {
            node.parent
                .as_ref()
                .map(|parent| (node.id.clone(), parent.clone()))
        })
        .collect();

    for (node, parent) in memberships {
        if let Some(subgraph) = output
            .subgraphs
            .iter_mut()
            .find(|subgraph| subgraph.id == parent)
        {
            subgraph.children.push(node);
        }
    }
}

fn resolve_new_edge_id(
    output: &Document,
    requested: Option<&str>,
) -> Result<String, CommandApplyError> {
    let expected = format!("e{}", output.edges.len());
    match requested {
        None => Ok(expected),
        Some(id) if output.edges.iter().any(|edge| edge.id == id) => {
            Err(CommandApplyError::AddEdgeIdCollision { id: id.to_string() })
        }
        Some(id) if id == expected => Ok(expected),
        Some(id) => Err(CommandApplyError::AddEdgeIdUnsupported {
            id: id.to_string(),
            expected,
        }),
    }
}

fn resolve_edge_index(
    output: &Document,
    selector: &EdgeSelector,
) -> Result<usize, CommandApplyError> {
    match selector {
        EdgeSelector::Id(id) => output
            .edges
            .iter()
            .position(|edge| edge.id == *id)
            .ok_or_else(|| CommandApplyError::EdgeSelectorNoMatch {
                selector: Box::new(selector.clone()),
            }),
        EdgeSelector::Semantic { .. } => {
            let matches: Vec<usize> = output
                .edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    semantic_selector_matches(edge, selector).then_some(index)
                })
                .collect();

            match matches.as_slice() {
                [] => Err(CommandApplyError::EdgeSelectorNoMatch {
                    selector: Box::new(selector.clone()),
                }),
                [index] => Ok(*index),
                _ => Err(CommandApplyError::EdgeSelectorAmbiguous {
                    selector: Box::new(selector.clone()),
                    matches: matches
                        .into_iter()
                        .map(|index| output.edges[index].id.clone())
                        .collect(),
                }),
            }
        }
    }
}

fn semantic_selector_matches(edge: &super::Edge, selector: &EdgeSelector) -> bool {
    let EdgeSelector::Semantic {
        source,
        target,
        label,
        stroke,
        arrow_start,
        arrow_end,
        minlen,
    } = selector
    else {
        return false;
    };

    edge.source == *source
        && edge.target == *target
        && label
            .as_ref()
            .is_none_or(|expected| edge.label.as_ref() == Some(expected))
        && stroke
            .as_ref()
            .is_none_or(|expected| edge.stroke == *expected)
        && arrow_start
            .as_ref()
            .is_none_or(|expected| edge.arrow_start == *expected)
        && arrow_end
            .as_ref()
            .is_none_or(|expected| edge.arrow_end == *expected)
        && minlen.is_none_or(|expected| edge.minlen == expected)
}

pub(crate) fn relayout_output_for_command_apply(
    output: &Document,
) -> Result<Document, CommandApplyError> {
    let mut diagram = crate::mmds::from_document(output)
        .map_err(|err| relayout_failed("hydrate", err.to_string()))?;
    let geometry_level = output
        .geometry_level
        .parse::<GeometryLevel>()
        .map_err(|err| relayout_failed("config", err.to_string()))?;
    let layout_engine = output
        .metadata
        .engine
        .as_deref()
        .map(EngineAlgorithmId::parse)
        .transpose()
        .map_err(|err| relayout_failed("config", err.to_string()))?;
    let config = RenderConfig {
        geometry_level,
        layout_engine,
        ..RenderConfig::default()
    };
    let json = crate::runtime::graph_family::render_graph_family(
        &output.metadata.diagram_type,
        &mut diagram,
        OutputFormat::Mmds,
        &config,
    )
    .map_err(|err| relayout_failed("solve-render", err.to_string()))?;

    crate::mmds::parse_input(&json).map_err(|err| relayout_failed("parse", err.to_string()))
}

fn relayout_failed(stage: &str, source: String) -> CommandApplyError {
    CommandApplyError::RelayoutFailed {
        stage: stage.to_string(),
        source,
    }
}
