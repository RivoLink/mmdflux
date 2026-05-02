use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::output::NODE_STYLE_EXTENSION_NAMESPACE;
use super::{Bounds, Edge, Node, Output, Port, Position, Rect, Subgraph};

const COORD_EPS: f64 = 0.01;
const DISPLAY_EPS: f64 = 1.0;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MmdsDiff {
    pub(crate) before_geometry_level: String,
    pub(crate) after_geometry_level: String,
    pub(crate) events: Vec<MmdsDiffEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MmdsDiffEvent {
    pub(crate) kind: MmdsDiffKind,
    pub(crate) subject: MmdsDiffSubject,
    // Semantic events are the headlines; geometry events can be linked back as evidence.
    pub(crate) evidence: Vec<String>,
    pub(crate) related_event_ids: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MmdsDiffKind {
    GeometryLevelChanged,
    DirectionChanged,
    EngineChanged,
    NodeAdded,
    NodeRemoved,
    EdgeAdded,
    EdgeRemoved,
    SubgraphAdded,
    SubgraphRemoved,
    NodeLabelChanged,
    NodeShapeChanged,
    NodeParentChanged,
    NodeStyleChanged,
    EdgeReconnected,
    EdgeEndpointIntentChanged,
    EdgeLabelChanged,
    EdgeStyleChanged,
    SubgraphTitleChanged,
    SubgraphDirectionChanged,
    SubgraphParentChanged,
    SubgraphMembershipChanged,
    SubgraphVisibilityChanged,
    ProfileChanged,
    ExtensionChanged,
    NodeMoved,
    NodeResized,
    CanvasResized,
    SubgraphBoundsChanged,
    EdgeRerouted,
    EndpointFaceChanged,
    PortIntentChanged,
    LabelMoved,
    LabelResized,
    LabelSideChanged,
    PathPortDivergenceChanged,
    GlobalReflowDetected,
}

impl MmdsDiffKind {
    fn is_geometry_effect(self) -> bool {
        matches!(
            self,
            Self::NodeMoved
                | Self::NodeResized
                | Self::CanvasResized
                | Self::SubgraphBoundsChanged
                | Self::EdgeRerouted
                | Self::EndpointFaceChanged
                | Self::PortIntentChanged
                | Self::LabelMoved
                | Self::LabelResized
                | Self::LabelSideChanged
                | Self::PathPortDivergenceChanged
                | Self::GlobalReflowDetected
        )
    }

    fn can_have_related_geometry(self) -> bool {
        !self.is_geometry_effect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MmdsDiffSubject {
    Document,
    Node(String),
    Edge(String),
    Subgraph(String),
}

pub(crate) fn diff_outputs(before: &Output, after: &Output) -> MmdsDiff {
    let mut events = Vec::new();

    if before.geometry_level != after.geometry_level {
        events.push(document_event(MmdsDiffKind::GeometryLevelChanged));
    }
    if before.metadata.direction != after.metadata.direction {
        events.push(document_event(MmdsDiffKind::DirectionChanged));
    }
    if before.metadata.engine != after.metadata.engine {
        events.push(document_event(MmdsDiffKind::EngineChanged));
    }
    if bounds_changed(&before.metadata.bounds, &after.metadata.bounds) {
        events.push(document_event_with_evidence(
            MmdsDiffKind::CanvasResized,
            vec![bounds_evidence(
                "canvas",
                &before.metadata.bounds,
                &after.metadata.bounds,
            )],
        ));
    }
    if before.profiles != after.profiles {
        events.push(document_event(MmdsDiffKind::ProfileChanged));
    }
    if before.extensions != after.extensions {
        events.push(document_event(MmdsDiffKind::ExtensionChanged));
    }

    let before_nodes = nodes_by_id(before);
    let after_nodes = nodes_by_id(after);
    push_removed_added(
        &mut events,
        &before_nodes,
        &after_nodes,
        MmdsDiffKind::NodeRemoved,
        MmdsDiffKind::NodeAdded,
        MmdsDiffSubject::Node,
    );
    push_node_semantic_events(&mut events, before, after, &before_nodes, &after_nodes);
    push_node_geometry_events(&mut events, &before_nodes, &after_nodes);
    push_global_reflow_event(&mut events, before, after, &before_nodes, &after_nodes);

    let before_edges = edges_by_id(before);
    let after_edges = edges_by_id(after);
    push_removed_added(
        &mut events,
        &before_edges,
        &after_edges,
        MmdsDiffKind::EdgeRemoved,
        MmdsDiffKind::EdgeAdded,
        MmdsDiffSubject::Edge,
    );
    push_edge_semantic_events(&mut events, &before_edges, &after_edges);
    push_edge_geometry_events(
        &mut events,
        &before_edges,
        &after_edges,
        &before_nodes,
        &after_nodes,
    );

    let before_subgraphs = subgraphs_by_id(before);
    let after_subgraphs = subgraphs_by_id(after);
    push_removed_added(
        &mut events,
        &before_subgraphs,
        &after_subgraphs,
        MmdsDiffKind::SubgraphRemoved,
        MmdsDiffKind::SubgraphAdded,
        MmdsDiffSubject::Subgraph,
    );
    push_subgraph_semantic_events(&mut events, &before_subgraphs, &after_subgraphs);
    push_subgraph_geometry_events(&mut events, &before_subgraphs, &after_subgraphs);

    link_related_geometry(&mut events);

    MmdsDiff {
        before_geometry_level: before.geometry_level.clone(),
        after_geometry_level: after.geometry_level.clone(),
        events,
    }
}

impl MmdsDiff {
    pub(crate) fn has_event(&self, kind: MmdsDiffKind, subject_id: &str) -> bool {
        self.events
            .iter()
            .any(|event| event.kind == kind && event.subject.matches_id(subject_id))
    }

    pub(crate) fn has_kind(&self, kind: MmdsDiffKind) -> bool {
        self.events.iter().any(|event| event.kind == kind)
    }

    pub(crate) fn has_related_geometry_for(&self, subject_id: &str) -> bool {
        self.events.iter().any(|event| {
            event.subject.matches_id(subject_id)
                && (event.kind.is_geometry_effect() || !event.related_event_ids.is_empty())
        })
    }
}

impl MmdsDiffEvent {
    pub(crate) fn evidence_mentions(&self, needle: &str) -> bool {
        self.evidence
            .iter()
            .any(|evidence| evidence.contains(needle))
    }
}

impl MmdsDiffSubject {
    fn matches_id(&self, subject_id: &str) -> bool {
        match self {
            Self::Document => subject_id.is_empty(),
            Self::Node(id) | Self::Edge(id) | Self::Subgraph(id) => id == subject_id,
        }
    }
}

fn document_event(kind: MmdsDiffKind) -> MmdsDiffEvent {
    document_event_with_evidence(kind, Vec::new())
}

fn document_event_with_evidence(kind: MmdsDiffKind, evidence: Vec<String>) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Document,
        evidence,
        related_event_ids: Vec::new(),
    }
}

fn nodes_by_id(output: &Output) -> BTreeMap<String, &Node> {
    output
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node))
        .collect()
}

fn edges_by_id(output: &Output) -> BTreeMap<String, &Edge> {
    output
        .edges
        .iter()
        .map(|edge| (edge.id.clone(), edge))
        .collect()
}

fn subgraphs_by_id(output: &Output) -> BTreeMap<String, &Subgraph> {
    output
        .subgraphs
        .iter()
        .map(|subgraph| (subgraph.id.clone(), subgraph))
        .collect()
}

fn push_removed_added<T>(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, T>,
    after: &BTreeMap<String, T>,
    removed_kind: MmdsDiffKind,
    added_kind: MmdsDiffKind,
    subject: fn(String) -> MmdsDiffSubject,
) {
    for id in before.keys().filter(|id| !after.contains_key(*id)) {
        events.push(MmdsDiffEvent {
            kind: removed_kind,
            subject: subject(id.clone()),
            evidence: Vec::new(),
            related_event_ids: Vec::new(),
        });
    }

    for id in after.keys().filter(|id| !before.contains_key(*id)) {
        events.push(MmdsDiffEvent {
            kind: added_kind,
            subject: subject(id.clone()),
            evidence: Vec::new(),
            related_event_ids: Vec::new(),
        });
    }
}

fn push_node_semantic_events(
    events: &mut Vec<MmdsDiffEvent>,
    before_output: &Output,
    after_output: &Output,
    before: &BTreeMap<String, &Node>,
    after: &BTreeMap<String, &Node>,
) {
    for (id, before_node) in before {
        let Some(after_node) = after.get(id) else {
            continue;
        };

        if before_node.label != after_node.label {
            events.push(node_event(MmdsDiffKind::NodeLabelChanged, id));
        }
        if before_node.shape != after_node.shape {
            events.push(node_event(MmdsDiffKind::NodeShapeChanged, id));
        }
        if before_node.parent != after_node.parent {
            events.push(node_event(MmdsDiffKind::NodeParentChanged, id));
        }
        if node_style_payload(before_output, id) != node_style_payload(after_output, id) {
            events.push(node_event(MmdsDiffKind::NodeStyleChanged, id));
        }
    }
}

fn push_edge_semantic_events(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, &Edge>,
    after: &BTreeMap<String, &Edge>,
) {
    for (id, before_edge) in before {
        let Some(after_edge) = after.get(id) else {
            continue;
        };

        if before_edge.source != after_edge.source || before_edge.target != after_edge.target {
            events.push(edge_event(MmdsDiffKind::EdgeReconnected, id));
        }
        if before_edge.from_subgraph != after_edge.from_subgraph
            || before_edge.to_subgraph != after_edge.to_subgraph
        {
            events.push(edge_event(MmdsDiffKind::EdgeEndpointIntentChanged, id));
        }
        if before_edge.label != after_edge.label {
            events.push(edge_event(MmdsDiffKind::EdgeLabelChanged, id));
        }
        if edge_style_changed(before_edge, after_edge) {
            events.push(edge_event(MmdsDiffKind::EdgeStyleChanged, id));
        }
    }
}

fn push_node_geometry_events(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, &Node>,
    after: &BTreeMap<String, &Node>,
) {
    for (id, before_node) in before {
        let Some(after_node) = after.get(id) else {
            continue;
        };

        let dx = after_node.position.x - before_node.position.x;
        let dy = after_node.position.y - before_node.position.y;
        let distance = dx.hypot(dy);
        if distance > DISPLAY_EPS {
            events.push(node_event_with_evidence(
                MmdsDiffKind::NodeMoved,
                id,
                vec![format!(
                    "displacement dx={dx:.2}; dy={dy:.2}; distance={distance:.2}"
                )],
            ));
        }

        let width_delta = after_node.size.width - before_node.size.width;
        let height_delta = after_node.size.height - before_node.size.height;
        if width_delta.abs() > DISPLAY_EPS || height_delta.abs() > DISPLAY_EPS {
            events.push(node_event_with_evidence(
                MmdsDiffKind::NodeResized,
                id,
                vec![format!(
                    "size width_delta={width_delta:.2}; height_delta={height_delta:.2}"
                )],
            ));
        }
    }
}

fn push_global_reflow_event(
    events: &mut Vec<MmdsDiffEvent>,
    before_output: &Output,
    after_output: &Output,
    before: &BTreeMap<String, &Node>,
    after: &BTreeMap<String, &Node>,
) {
    let mut unchanged_count = 0usize;
    let mut moved_ids = Vec::new();

    for (id, before_node) in before {
        let Some(after_node) = after.get(id) else {
            continue;
        };
        if !node_semantics_same(before_output, after_output, id, before_node, after_node) {
            continue;
        }

        unchanged_count += 1;
        let dx = after_node.position.x - before_node.position.x;
        let dy = after_node.position.y - before_node.position.y;
        if dx.hypot(dy) > DISPLAY_EPS {
            moved_ids.push(id.clone());
        }
    }

    if moved_ids.len() >= 3 && moved_ids.len() * 5 >= unchanged_count.max(1) {
        events.push(document_event_with_evidence(
            MmdsDiffKind::GlobalReflowDetected,
            vec![format!(
                "unchanged_nodes_moved={}/{}; threshold=min(3 nodes, 20 percent); sample={:?}",
                moved_ids.len(),
                unchanged_count,
                moved_ids.iter().take(5).collect::<Vec<_>>()
            )],
        ));
    }
}

fn node_semantics_same(
    before_output: &Output,
    after_output: &Output,
    id: &str,
    before: &Node,
    after: &Node,
) -> bool {
    before.label == after.label
        && before.shape == after.shape
        && before.parent == after.parent
        && node_style_payload(before_output, id) == node_style_payload(after_output, id)
}

fn push_edge_geometry_events(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, &Edge>,
    after: &BTreeMap<String, &Edge>,
    before_nodes: &BTreeMap<String, &Node>,
    after_nodes: &BTreeMap<String, &Node>,
) {
    for (id, before_edge) in before {
        let Some(after_edge) = after.get(id) else {
            continue;
        };

        let path = path_delta(before_edge, after_edge);
        if path.changed {
            events.push(edge_event_with_evidence(
                MmdsDiffKind::EdgeRerouted,
                id,
                vec![format!(
                    "path point_count_delta={}; bend_count_delta={}; length_delta={:.2}; envelope_changed={}",
                    path.point_count_delta, path.bend_count_delta, path.length_delta, path.envelope_changed
                )],
            ));
        }

        for (endpoint, before_face, after_face) in [
            (
                "source",
                endpoint_face(before_edge, before_nodes, Endpoint::Source),
                endpoint_face(after_edge, after_nodes, Endpoint::Source),
            ),
            (
                "target",
                endpoint_face(before_edge, before_nodes, Endpoint::Target),
                endpoint_face(after_edge, after_nodes, Endpoint::Target),
            ),
        ] {
            if before_face != after_face {
                events.push(edge_event_with_evidence(
                    MmdsDiffKind::EndpointFaceChanged,
                    id,
                    vec![format!(
                        "visible {endpoint} endpoint face {before_face}->{after_face}"
                    )],
                ));
            }
        }

        for (endpoint, before_port, after_port) in [
            (
                "source",
                before_edge.source_port.as_ref(),
                after_edge.source_port.as_ref(),
            ),
            (
                "target",
                before_edge.target_port.as_ref(),
                after_edge.target_port.as_ref(),
            ),
        ] {
            if !same_port_intent(before_port, after_port) {
                events.push(edge_event_with_evidence(
                    MmdsDiffKind::PortIntentChanged,
                    id,
                    vec![format!(
                        "logical {endpoint}_port {}->{}",
                        port_summary(before_port),
                        port_summary(after_port)
                    )],
                ));
            }
        }

        push_label_geometry_events(events, id, before_edge, after_edge);
        push_path_port_divergence_events(
            events,
            id,
            before_edge,
            after_edge,
            before_nodes,
            after_nodes,
        );
    }
}

fn push_subgraph_semantic_events(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, &Subgraph>,
    after: &BTreeMap<String, &Subgraph>,
) {
    for (id, before_subgraph) in before {
        let Some(after_subgraph) = after.get(id) else {
            continue;
        };

        if before_subgraph.title != after_subgraph.title {
            events.push(subgraph_event(MmdsDiffKind::SubgraphTitleChanged, id));
        }
        if before_subgraph.direction != after_subgraph.direction {
            events.push(subgraph_event(MmdsDiffKind::SubgraphDirectionChanged, id));
        }
        if before_subgraph.parent != after_subgraph.parent {
            events.push(subgraph_event(MmdsDiffKind::SubgraphParentChanged, id));
        }
        if string_set(&before_subgraph.children) != string_set(&after_subgraph.children)
            || string_set(&before_subgraph.concurrent_regions)
                != string_set(&after_subgraph.concurrent_regions)
        {
            events.push(subgraph_event(MmdsDiffKind::SubgraphMembershipChanged, id));
        }
        if before_subgraph.invisible != after_subgraph.invisible {
            events.push(subgraph_event(MmdsDiffKind::SubgraphVisibilityChanged, id));
        }
    }
}

fn push_subgraph_geometry_events(
    events: &mut Vec<MmdsDiffEvent>,
    before: &BTreeMap<String, &Subgraph>,
    after: &BTreeMap<String, &Subgraph>,
) {
    for (id, before_subgraph) in before {
        let Some(after_subgraph) = after.get(id) else {
            continue;
        };

        if option_bounds_changed(
            before_subgraph.bounds.as_ref(),
            after_subgraph.bounds.as_ref(),
        ) {
            events.push(subgraph_event_with_evidence(
                MmdsDiffKind::SubgraphBoundsChanged,
                id,
                vec![option_bounds_evidence(
                    "subgraph_bounds",
                    before_subgraph.bounds.as_ref(),
                    after_subgraph.bounds.as_ref(),
                )],
            ));
        }
    }
}

fn push_label_geometry_events(
    events: &mut Vec<MmdsDiffEvent>,
    edge_id: &str,
    before: &Edge,
    after: &Edge,
) {
    if before.label_side != after.label_side {
        events.push(edge_event_with_evidence(
            MmdsDiffKind::LabelSideChanged,
            edge_id,
            vec![format!(
                "label_side {:?}->{:?}",
                before.label_side, after.label_side
            )],
        ));
    }

    match (&before.label_rect, &after.label_rect) {
        (Some(before_rect), Some(after_rect)) => {
            let (before_x, before_y) = rect_center(before_rect);
            let (after_x, after_y) = rect_center(after_rect);
            let dx = after_x - before_x;
            let dy = after_y - before_y;
            let width_delta = after_rect.width - before_rect.width;
            let height_delta = after_rect.height - before_rect.height;

            if dx.hypot(dy) > DISPLAY_EPS {
                events.push(edge_event_with_evidence(
                    MmdsDiffKind::LabelMoved,
                    edge_id,
                    vec![format!("label_rect center_dx={dx:.2}; center_dy={dy:.2}")],
                ));
            }
            if width_delta.abs() > DISPLAY_EPS || height_delta.abs() > DISPLAY_EPS {
                events.push(edge_event_with_evidence(
                    MmdsDiffKind::LabelResized,
                    edge_id,
                    vec![format!(
                        "label_rect width_delta={width_delta:.2}; height_delta={height_delta:.2}"
                    )],
                ));
            }
        }
        (None, Some(_)) | (Some(_), None) => {
            events.push(edge_event_with_evidence(
                MmdsDiffKind::LabelMoved,
                edge_id,
                vec!["label_rect presence changed".to_string()],
            ));
            events.push(edge_event_with_evidence(
                MmdsDiffKind::LabelResized,
                edge_id,
                vec!["label_rect presence changed".to_string()],
            ));
        }
        (None, None) => {
            let before_pos = before.label_position.as_ref();
            let after_pos = after.label_position.as_ref();
            if option_position_moved(before_pos, after_pos) {
                events.push(edge_event_with_evidence(
                    MmdsDiffKind::LabelMoved,
                    edge_id,
                    vec!["label_position changed without label_rect".to_string()],
                ));
            }
        }
    }
}

fn push_path_port_divergence_events(
    events: &mut Vec<MmdsDiffEvent>,
    edge_id: &str,
    before_edge: &Edge,
    after_edge: &Edge,
    before_nodes: &BTreeMap<String, &Node>,
    after_nodes: &BTreeMap<String, &Node>,
) {
    for (endpoint, before_path_face, after_path_face, before_port_face, after_port_face) in [
        (
            "source",
            endpoint_face(before_edge, before_nodes, Endpoint::Source),
            endpoint_face(after_edge, after_nodes, Endpoint::Source),
            before_edge
                .source_port
                .as_ref()
                .map(|port| port.face.as_str()),
            after_edge
                .source_port
                .as_ref()
                .map(|port| port.face.as_str()),
        ),
        (
            "target",
            endpoint_face(before_edge, before_nodes, Endpoint::Target),
            endpoint_face(after_edge, after_nodes, Endpoint::Target),
            before_edge
                .target_port
                .as_ref()
                .map(|port| port.face.as_str()),
            after_edge
                .target_port
                .as_ref()
                .map(|port| port.face.as_str()),
        ),
    ] {
        let before_diverged = path_port_diverged(before_path_face.as_str(), before_port_face);
        let after_diverged = path_port_diverged(after_path_face.as_str(), after_port_face);
        if before_diverged != after_diverged {
            events.push(edge_event_with_evidence(
                MmdsDiffKind::PathPortDivergenceChanged,
                edge_id,
                vec![format!(
                    "path_port_divergence {endpoint} {before_diverged}->{after_diverged}; visible {before_path_face}->{after_path_face}; logical {endpoint}_port {:?}->{:?}",
                    before_port_face, after_port_face
                )],
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Endpoint {
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DerivedEndpointFace {
    Face(String),
    Ambiguous,
    Missing,
}

impl DerivedEndpointFace {
    fn as_str(&self) -> &str {
        match self {
            Self::Face(face) => face.as_str(),
            Self::Ambiguous => "ambiguous",
            Self::Missing => "missing",
        }
    }
}

impl std::fmt::Display for DerivedEndpointFace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy)]
struct PathDelta {
    point_count_delta: isize,
    bend_count_delta: isize,
    length_delta: f64,
    envelope_changed: bool,
    changed: bool,
}

fn path_delta(before: &Edge, after: &Edge) -> PathDelta {
    let before_path = before.path.as_deref().unwrap_or(&[]);
    let after_path = after.path.as_deref().unwrap_or(&[]);
    let point_count_delta = after_path.len() as isize - before_path.len() as isize;
    let bend_count_delta = bend_count(after_path) as isize - bend_count(before_path) as isize;
    let length_delta = path_length(after_path) - path_length(before_path);
    let envelope_changed = path_envelope_changed(before_path, after_path);
    let changed = point_count_delta != 0
        || bend_count_delta != 0
        || length_delta.abs() > DISPLAY_EPS
        || envelope_changed;

    PathDelta {
        point_count_delta,
        bend_count_delta,
        length_delta,
        envelope_changed,
        changed,
    }
}

fn path_length(path: &[[f64; 2]]) -> f64 {
    path.windows(2)
        .map(|window| {
            let dx = window[1][0] - window[0][0];
            let dy = window[1][1] - window[0][1];
            dx.hypot(dy)
        })
        .sum()
}

fn bend_count(path: &[[f64; 2]]) -> usize {
    path.windows(3)
        .filter(|window| {
            let first = segment_axis(window[0], window[1]);
            let second = segment_axis(window[1], window[2]);
            first != SegmentAxis::Point && second != SegmentAxis::Point && first != second
        })
        .count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentAxis {
    Horizontal,
    Vertical,
    Diagonal,
    Point,
}

fn segment_axis(a: [f64; 2], b: [f64; 2]) -> SegmentAxis {
    let dx = (b[0] - a[0]).abs();
    let dy = (b[1] - a[1]).abs();

    match (dx <= COORD_EPS, dy <= COORD_EPS) {
        (true, true) => SegmentAxis::Point,
        (false, true) => SegmentAxis::Horizontal,
        (true, false) => SegmentAxis::Vertical,
        (false, false) => SegmentAxis::Diagonal,
    }
}

fn path_envelope_changed(before: &[[f64; 2]], after: &[[f64; 2]]) -> bool {
    match (path_bounds(before), path_bounds(after)) {
        (Some(before), Some(after)) => bounds_changed(&before, &after),
        (None, None) => false,
        _ => true,
    }
}

fn path_bounds(path: &[[f64; 2]]) -> Option<Bounds> {
    let first = path.first()?;
    let (mut min_x, mut max_x) = (first[0], first[0]);
    let (mut min_y, mut max_y) = (first[1], first[1]);

    for point in path.iter().skip(1) {
        min_x = min_x.min(point[0]);
        max_x = max_x.max(point[0]);
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }

    Some(Bounds {
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

fn endpoint_face(
    edge: &Edge,
    nodes: &BTreeMap<String, &Node>,
    endpoint: Endpoint,
) -> DerivedEndpointFace {
    let Some(path) = &edge.path else {
        return DerivedEndpointFace::Missing;
    };
    let point = match endpoint {
        Endpoint::Source => path.first(),
        Endpoint::Target => path.last(),
    };
    let Some(point) = point else {
        return DerivedEndpointFace::Missing;
    };
    let node_id = match endpoint {
        Endpoint::Source => &edge.source,
        Endpoint::Target => &edge.target,
    };
    let Some(node) = nodes.get(node_id) else {
        return DerivedEndpointFace::Missing;
    };

    derive_face_for_point(*point, node)
}

fn derive_face_for_point(point: [f64; 2], node: &Node) -> DerivedEndpointFace {
    let left = node.position.x - node.size.width / 2.0;
    let right = node.position.x + node.size.width / 2.0;
    let top = node.position.y - node.size.height / 2.0;
    let bottom = node.position.y + node.size.height / 2.0;
    let candidates = [
        ("left", (point[0] - left).abs()),
        ("right", (point[0] - right).abs()),
        ("top", (point[1] - top).abs()),
        ("bottom", (point[1] - bottom).abs()),
    ]
    .into_iter()
    .filter(|(_, distance)| *distance <= COORD_EPS)
    .map(|(face, _)| face)
    .collect::<Vec<_>>();

    match candidates.as_slice() {
        [] => DerivedEndpointFace::Missing,
        [face] => DerivedEndpointFace::Face((*face).to_string()),
        _ => DerivedEndpointFace::Ambiguous,
    }
}

fn same_port_intent(before: Option<&Port>, after: Option<&Port>) -> bool {
    match (before, after) {
        (None, None) => true,
        (Some(before), Some(after)) => {
            before.face == after.face
                && (before.fraction - after.fraction).abs() <= COORD_EPS
                && before.group_size == after.group_size
        }
        _ => false,
    }
}

fn port_summary(port: Option<&Port>) -> String {
    port.map(|port| format!("{}@{:.3}/{}", port.face, port.fraction, port.group_size))
        .unwrap_or_else(|| "none".to_string())
}

fn path_port_diverged(path_face: &str, port_face: Option<&str>) -> bool {
    match (path_face, port_face) {
        ("ambiguous", Some(_)) => true,
        ("missing", _) | (_, None) => false,
        (face, Some(port)) => face != port,
    }
}

fn rect_center(rect: &Rect) -> (f64, f64) {
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn option_position_moved(before: Option<&Position>, after: Option<&Position>) -> bool {
    match (before, after) {
        (Some(before), Some(after)) => {
            let dx = after.x - before.x;
            let dy = after.y - before.y;
            dx.hypot(dy) > DISPLAY_EPS
        }
        (None, None) => false,
        _ => true,
    }
}

fn bounds_changed(before: &Bounds, after: &Bounds) -> bool {
    (after.width - before.width).abs() > DISPLAY_EPS
        || (after.height - before.height).abs() > DISPLAY_EPS
}

fn bounds_evidence(label: &str, before: &Bounds, after: &Bounds) -> String {
    format!(
        "{label} width_delta={:.2}; height_delta={:.2}",
        after.width - before.width,
        after.height - before.height
    )
}

fn option_bounds_changed(before: Option<&Bounds>, after: Option<&Bounds>) -> bool {
    match (before, after) {
        (Some(before), Some(after)) => bounds_changed(before, after),
        (None, None) => false,
        _ => true,
    }
}

fn option_bounds_evidence(label: &str, before: Option<&Bounds>, after: Option<&Bounds>) -> String {
    match (before, after) {
        (Some(before), Some(after)) => bounds_evidence(label, before, after),
        (None, Some(after)) => {
            format!(
                "{label} added width={:.2}; height={:.2}",
                after.width, after.height
            )
        }
        (Some(before), None) => {
            format!(
                "{label} removed width={:.2}; height={:.2}",
                before.width, before.height
            )
        }
        (None, None) => format!("{label} unchanged"),
    }
}

fn link_related_geometry(events: &mut [MmdsDiffEvent]) {
    let geometry_events = events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.kind.is_geometry_effect())
        .map(|(index, event)| (index, event.subject.clone(), event.kind))
        .collect::<Vec<_>>();

    for event in events.iter_mut() {
        if !event.kind.can_have_related_geometry() {
            continue;
        }

        for (index, subject, kind) in &geometry_events {
            if *subject == event.subject {
                event.related_event_ids.push(*index);
                event
                    .evidence
                    .push(format!("related_geometry event_id={index}; kind={kind:?}"));
            }
        }
    }
}

fn node_style_payload<'a>(output: &'a Output, node_id: &str) -> Option<&'a Value> {
    output
        .extensions
        .get(NODE_STYLE_EXTENSION_NAMESPACE)?
        .get("nodes")?
        .as_object()?
        .get(node_id)
}

fn edge_style_changed(before: &Edge, after: &Edge) -> bool {
    before.stroke != after.stroke
        || before.arrow_start != after.arrow_start
        || before.arrow_end != after.arrow_end
        || before.minlen != after.minlen
}

fn string_set(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn node_event(kind: MmdsDiffKind, id: &str) -> MmdsDiffEvent {
    node_event_with_evidence(kind, id, Vec::new())
}

fn node_event_with_evidence(kind: MmdsDiffKind, id: &str, evidence: Vec<String>) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Node(id.to_string()),
        evidence,
        related_event_ids: Vec::new(),
    }
}

fn edge_event(kind: MmdsDiffKind, id: &str) -> MmdsDiffEvent {
    edge_event_with_evidence(kind, id, Vec::new())
}

fn edge_event_with_evidence(kind: MmdsDiffKind, id: &str, evidence: Vec<String>) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Edge(id.to_string()),
        evidence,
        related_event_ids: Vec::new(),
    }
}

fn subgraph_event(kind: MmdsDiffKind, id: &str) -> MmdsDiffEvent {
    subgraph_event_with_evidence(kind, id, Vec::new())
}

fn subgraph_event_with_evidence(
    kind: MmdsDiffKind,
    id: &str,
    evidence: Vec<String>,
) -> MmdsDiffEvent {
    MmdsDiffEvent {
        kind,
        subject: MmdsDiffSubject::Subgraph(id.to_string()),
        evidence,
        related_event_ids: Vec::new(),
    }
}
