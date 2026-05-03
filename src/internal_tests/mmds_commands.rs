use serde_json::json;

use crate::mmds::commands::{
    Command, EdgeSelector, MmdsDiffKindRole, commandable_diff_kinds, diff_kind_for_command,
    diff_kind_role, geometry_effect_diff_kinds,
};
use crate::mmds::diff::MmdsDiffKind;

#[test]
fn mmds_command_vocabulary_classifies_all_diff_kinds() {
    let cases = [
        (
            MmdsDiffKind::GeometryLevelChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::DirectionChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (MmdsDiffKind::EngineChanged, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::NodeAdded, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::NodeRemoved, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::EdgeAdded, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::EdgeRemoved, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::SubgraphAdded, MmdsDiffKindRole::Commandable),
        (MmdsDiffKind::SubgraphRemoved, MmdsDiffKindRole::Commandable),
        (
            MmdsDiffKind::NodeLabelChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::NodeShapeChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::NodeParentChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::NodeStyleChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (MmdsDiffKind::EdgeReconnected, MmdsDiffKindRole::Commandable),
        (
            MmdsDiffKind::EdgeEndpointIntentChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::EdgeLabelChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::EdgeStyleChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::SubgraphTitleChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::SubgraphDirectionChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::SubgraphParentChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::SubgraphMembershipChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (
            MmdsDiffKind::SubgraphVisibilityChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (MmdsDiffKind::ProfileChanged, MmdsDiffKindRole::Commandable),
        (
            MmdsDiffKind::ExtensionChanged,
            MmdsDiffKindRole::Commandable,
        ),
        (MmdsDiffKind::NodeMoved, MmdsDiffKindRole::GeometryEffect),
        (MmdsDiffKind::NodeResized, MmdsDiffKindRole::GeometryEffect),
        (
            MmdsDiffKind::CanvasResized,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (
            MmdsDiffKind::SubgraphBoundsChanged,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (MmdsDiffKind::EdgeRerouted, MmdsDiffKindRole::GeometryEffect),
        (
            MmdsDiffKind::EndpointFaceChanged,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (
            MmdsDiffKind::PortIntentChanged,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (MmdsDiffKind::LabelMoved, MmdsDiffKindRole::GeometryEffect),
        (MmdsDiffKind::LabelResized, MmdsDiffKindRole::GeometryEffect),
        (
            MmdsDiffKind::LabelSideChanged,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (
            MmdsDiffKind::PathPortDivergenceChanged,
            MmdsDiffKindRole::GeometryEffect,
        ),
        (
            MmdsDiffKind::GlobalReflowDetected,
            MmdsDiffKindRole::GeometryEffect,
        ),
    ];

    // The exhaustive matches in `mmds::commands` are the compile-time drift
    // guard. This count keeps the test fixture's explicit variant list honest.
    assert_eq!(cases.len(), 36);

    for (kind, expected_role) in cases {
        assert_eq!(diff_kind_role(kind), expected_role, "{kind:?}");
        assert_eq!(
            kind.is_geometry_effect(),
            expected_role == MmdsDiffKindRole::GeometryEffect,
            "{kind:?}"
        );
    }
}

#[test]
fn mmds_command_vocabulary_document_node_examples_map_to_diff_kinds() {
    for (command, expected_kind) in document_node_command_examples() {
        assert_eq!(
            diff_kind_for_command(&command),
            expected_kind,
            "{command:?}"
        );
    }
}

#[test]
fn mmds_command_vocabulary_document_node_commands_are_semantic_only() {
    for (command, _) in document_node_command_examples() {
        match command {
            Command::SetGeometryLevel { level } => assert_eq!(level, "routed"),
            Command::SetDirection { direction } => assert_eq!(direction, "LR"),
            Command::SetEngine { engine } => assert_eq!(engine.as_deref(), Some("flux-layered")),
            Command::AddNode {
                id,
                label,
                shape,
                parent,
            } => {
                assert_eq!(id, "A");
                assert_eq!(label, "Alpha");
                assert_eq!(shape, "stadium");
                assert_eq!(parent.as_deref(), Some("cluster_0"));
            }
            Command::RemoveNode { id } => assert_eq!(id, "A"),
            Command::ChangeNodeLabel { node, label } => {
                assert_eq!(node, "A");
                assert_eq!(label, "Alpha");
            }
            Command::ChangeNodeShape { node, shape } => {
                assert_eq!(node, "A");
                assert_eq!(shape, "stadium");
            }
            Command::SetNodeParent { node, parent } => {
                assert_eq!(node, "A");
                assert_eq!(parent.as_deref(), Some("cluster_0"));
            }
            Command::SetNodeStyleExtension { node, value } => {
                assert_eq!(node, "A");
                assert_eq!(value["fill"], "#fff");
            }
            Command::SetProfiles { profiles } => {
                assert_eq!(profiles, vec!["mmds-core-v1".to_string()]);
            }
            Command::SetExtension { namespace, value } => {
                assert_eq!(namespace, "example.extension");
                assert_eq!(value["enabled"], true);
            }
            other => panic!("unexpected non-document/node command: {other:?}"),
        }
    }
}

fn document_node_command_examples() -> Vec<(Command, MmdsDiffKind)> {
    vec![
        (
            Command::SetGeometryLevel {
                level: "routed".to_string(),
            },
            MmdsDiffKind::GeometryLevelChanged,
        ),
        (
            Command::SetDirection {
                direction: "LR".to_string(),
            },
            MmdsDiffKind::DirectionChanged,
        ),
        (
            Command::SetEngine {
                engine: Some("flux-layered".to_string()),
            },
            MmdsDiffKind::EngineChanged,
        ),
        (
            Command::AddNode {
                id: "A".to_string(),
                label: "Alpha".to_string(),
                shape: "stadium".to_string(),
                parent: Some("cluster_0".to_string()),
            },
            MmdsDiffKind::NodeAdded,
        ),
        (
            Command::RemoveNode {
                id: "A".to_string(),
            },
            MmdsDiffKind::NodeRemoved,
        ),
        (
            Command::ChangeNodeLabel {
                node: "A".to_string(),
                label: "Alpha".to_string(),
            },
            MmdsDiffKind::NodeLabelChanged,
        ),
        (
            Command::ChangeNodeShape {
                node: "A".to_string(),
                shape: "stadium".to_string(),
            },
            MmdsDiffKind::NodeShapeChanged,
        ),
        (
            Command::SetNodeParent {
                node: "A".to_string(),
                parent: Some("cluster_0".to_string()),
            },
            MmdsDiffKind::NodeParentChanged,
        ),
        (
            Command::SetNodeStyleExtension {
                node: "A".to_string(),
                value: json!({ "fill": "#fff" }),
            },
            MmdsDiffKind::NodeStyleChanged,
        ),
        (
            Command::SetProfiles {
                profiles: vec!["mmds-core-v1".to_string()],
            },
            MmdsDiffKind::ProfileChanged,
        ),
        (
            Command::SetExtension {
                namespace: "example.extension".to_string(),
                value: json!({ "enabled": true }),
            },
            MmdsDiffKind::ExtensionChanged,
        ),
    ]
}

#[test]
fn mmds_command_vocabulary_edge_examples_map_to_diff_kinds() {
    for (command, expected_kind) in edge_command_examples() {
        assert_eq!(
            diff_kind_for_command(&command),
            expected_kind,
            "{command:?}"
        );
    }
}

#[test]
fn mmds_command_vocabulary_edge_selector_names_identity_without_layout() {
    let selector = EdgeSelector::Semantic {
        source: "A".to_string(),
        target: "B".to_string(),
        label: Some("calls".to_string()),
        stroke: Some("dotted".to_string()),
        arrow_start: Some("none".to_string()),
        arrow_end: Some("normal".to_string()),
        minlen: Some(2),
    };

    match selector {
        EdgeSelector::Semantic {
            source,
            target,
            label,
            stroke,
            arrow_start,
            arrow_end,
            minlen,
        } => {
            assert_eq!(source, "A");
            assert_eq!(target, "B");
            assert_eq!(label.as_deref(), Some("calls"));
            assert_eq!(stroke.as_deref(), Some("dotted"));
            assert_eq!(arrow_start.as_deref(), Some("none"));
            assert_eq!(arrow_end.as_deref(), Some("normal"));
            assert_eq!(minlen, Some(2));
        }
        EdgeSelector::Id(_) => panic!("expected semantic selector"),
    }
}

fn edge_command_examples() -> Vec<(Command, MmdsDiffKind)> {
    let selector = EdgeSelector::Id("e0".to_string());

    vec![
        (
            Command::AddEdge {
                id: Some("e0".to_string()),
                source: "A".to_string(),
                target: "B".to_string(),
                from_subgraph: None,
                to_subgraph: None,
                label: Some("calls".to_string()),
                stroke: "solid".to_string(),
                arrow_start: "none".to_string(),
                arrow_end: "normal".to_string(),
                minlen: 1,
            },
            MmdsDiffKind::EdgeAdded,
        ),
        (
            Command::RemoveEdge {
                edge: selector.clone(),
            },
            MmdsDiffKind::EdgeRemoved,
        ),
        (
            Command::ReconnectEdge {
                edge: selector.clone(),
                source: "A".to_string(),
                target: "C".to_string(),
            },
            MmdsDiffKind::EdgeReconnected,
        ),
        (
            Command::SetEdgeEndpointIntent {
                edge: selector.clone(),
                from_subgraph: Some("cluster_a".to_string()),
                to_subgraph: Some("cluster_b".to_string()),
            },
            MmdsDiffKind::EdgeEndpointIntentChanged,
        ),
        (
            Command::ChangeEdgeLabel {
                edge: selector.clone(),
                label: Some("calls".to_string()),
            },
            MmdsDiffKind::EdgeLabelChanged,
        ),
        (
            Command::ChangeEdgeStyle {
                edge: selector,
                stroke: Some("dotted".to_string()),
                arrow_start: Some("none".to_string()),
                arrow_end: Some("normal".to_string()),
                minlen: Some(2),
            },
            MmdsDiffKind::EdgeStyleChanged,
        ),
    ]
}

#[test]
fn mmds_command_vocabulary_subgraph_examples_map_to_diff_kinds() {
    for (command, expected_kind) in subgraph_command_examples() {
        assert_eq!(
            diff_kind_for_command(&command),
            expected_kind,
            "{command:?}"
        );
    }
}

#[test]
fn mmds_command_vocabulary_subgraph_membership_is_canonical() {
    let command = Command::ChangeSubgraphMembership {
        subgraph: "cluster_0".to_string(),
        added_children: vec!["A".to_string()],
        removed_children: vec!["B".to_string()],
        added_concurrent_regions: vec!["region_1".to_string()],
        removed_concurrent_regions: vec!["region_0".to_string()],
    };

    match command {
        Command::ChangeSubgraphMembership {
            subgraph,
            added_children,
            removed_children,
            added_concurrent_regions,
            removed_concurrent_regions,
        } => {
            assert_eq!(subgraph, "cluster_0");
            assert_eq!(added_children, vec!["A".to_string()]);
            assert_eq!(removed_children, vec!["B".to_string()]);
            assert_eq!(added_concurrent_regions, vec!["region_1".to_string()]);
            assert_eq!(removed_concurrent_regions, vec!["region_0".to_string()]);
        }
        other => panic!("unexpected command: {other:?}"),
    }
}

fn subgraph_command_examples() -> Vec<(Command, MmdsDiffKind)> {
    vec![
        (
            Command::AddSubgraph {
                id: "cluster_0".to_string(),
                title: Some("Cluster".to_string()),
                parent: None,
                direction: Some("LR".to_string()),
                children: vec!["A".to_string()],
                concurrent_regions: vec!["region_0".to_string()],
                invisible: false,
            },
            MmdsDiffKind::SubgraphAdded,
        ),
        (
            Command::RemoveSubgraph {
                id: "cluster_0".to_string(),
            },
            MmdsDiffKind::SubgraphRemoved,
        ),
        (
            Command::ChangeSubgraphTitle {
                subgraph: "cluster_0".to_string(),
                title: Some("Cluster".to_string()),
            },
            MmdsDiffKind::SubgraphTitleChanged,
        ),
        (
            Command::SetSubgraphDirection {
                subgraph: "cluster_0".to_string(),
                direction: Some("LR".to_string()),
            },
            MmdsDiffKind::SubgraphDirectionChanged,
        ),
        (
            Command::SetSubgraphParent {
                subgraph: "cluster_0".to_string(),
                parent: Some("root".to_string()),
            },
            MmdsDiffKind::SubgraphParentChanged,
        ),
        (
            Command::ChangeSubgraphMembership {
                subgraph: "cluster_0".to_string(),
                added_children: vec!["A".to_string()],
                removed_children: vec!["B".to_string()],
                added_concurrent_regions: vec!["region_1".to_string()],
                removed_concurrent_regions: vec!["region_0".to_string()],
            },
            MmdsDiffKind::SubgraphMembershipChanged,
        ),
        (
            Command::SetSubgraphVisibility {
                subgraph: "cluster_0".to_string(),
                invisible: true,
            },
            MmdsDiffKind::SubgraphVisibilityChanged,
        ),
    ]
}

#[test]
fn mmds_command_vocabulary_is_symmetric_with_diff_kinds() {
    let command_examples = all_command_examples();
    let command_kinds: Vec<MmdsDiffKind> = command_examples
        .iter()
        .map(|(command, _)| diff_kind_for_command(command))
        .collect();

    assert_same_kinds(&command_kinds, commandable_diff_kinds());
    assert_same_kinds(geometry_effect_diff_kinds(), &geometry_effect_kinds());

    for kind in all_diff_kinds() {
        let is_effect = contains_kind(geometry_effect_diff_kinds(), kind);
        let is_commandable = contains_kind(commandable_diff_kinds(), kind);

        assert_ne!(is_effect, is_commandable, "{kind:?}");
        assert_eq!(kind.is_geometry_effect(), is_effect, "{kind:?}");
    }
}

fn all_command_examples() -> Vec<(Command, MmdsDiffKind)> {
    let mut examples = document_node_command_examples();
    examples.extend(edge_command_examples());
    examples.extend(subgraph_command_examples());
    examples
}

fn all_diff_kinds() -> [MmdsDiffKind; 36] {
    [
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
    ]
}

fn geometry_effect_kinds() -> [MmdsDiffKind; 12] {
    [
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
    ]
}

fn assert_same_kinds(left: &[MmdsDiffKind], right: &[MmdsDiffKind]) {
    assert_eq!(left.len(), right.len());
    for kind in left {
        assert!(contains_kind(right, *kind), "missing {kind:?}");
    }
    for kind in right {
        assert!(contains_kind(left, *kind), "unexpected {kind:?}");
    }
}

fn contains_kind(kinds: &[MmdsDiffKind], needle: MmdsDiffKind) -> bool {
    kinds.contains(&needle)
}
