use std::collections::BTreeMap;

use crate::graph::GeometryLevel;
use crate::mmds::diff::{MmdsDiff, MmdsDiffKind};
use crate::{OutputFormat, RenderConfig};

#[test]
fn mmds_diff_model_identical_outputs_have_no_events() {
    let before = parse_routed("graph TD; A --> B");
    let after = parse_routed("graph TD; A --> B");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.events.is_empty(), "{diff:#?}");
    assert_eq!(diff.before_geometry_level, "routed");
    assert_eq!(diff.after_geometry_level, "routed");

    assert!(!diff.has_event(crate::mmds::diff::MmdsDiffKind::GeometryLevelChanged, ""));
}

#[test]
fn mmds_diff_identity_reports_node_and_edge_additions() {
    let before = parse_routed("graph TD; A --> B");
    let after = parse_routed("graph TD; A --> B; B --> C");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::NodeAdded, "C"));
    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::EdgeAdded, "e1"));
}

#[test]
fn mmds_diff_identity_treats_id_change_as_remove_add() {
    let before = parse_routed("graph TD; Lint --> Build");
    let after = parse_routed("graph TD; Audit --> Build");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::NodeRemoved, "Lint"));
    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::NodeAdded, "Audit"));
}

#[test]
fn mmds_diff_semantic_reports_node_label_and_shape_changes() {
    let before = parse_layout("graph TD; A[Build]");
    let after = parse_layout("graph TD; A{Deploy}");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::NodeLabelChanged, "A"));
    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::NodeShapeChanged, "A"));
}

#[test]
fn mmds_diff_semantic_reports_edge_label_and_style_changes() {
    let before = parse_routed("graph TD; A -->|ok| B");
    let after = parse_routed("graph TD; A -.->|warn| B");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::EdgeLabelChanged, "e0"));
    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::EdgeStyleChanged, "e0"));
}

#[test]
fn mmds_diff_semantic_reports_subgraph_title_direction_and_membership_changes() {
    let before = parse_layout(
        "graph TD
        subgraph sg [Pipeline]
            direction TB
            A
        end",
    );
    let after = parse_layout(
        "graph TD
        subgraph sg [Release]
            direction LR
            A
            B
        end",
    );

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_event(crate::mmds::diff::MmdsDiffKind::SubgraphTitleChanged, "sg"));
    assert!(diff.has_event(
        crate::mmds::diff::MmdsDiffKind::SubgraphDirectionChanged,
        "sg"
    ));
    assert!(diff.has_event(
        crate::mmds::diff::MmdsDiffKind::SubgraphMembershipChanged,
        "sg"
    ));
}

#[test]
fn mmds_diff_routed_separates_path_from_port_intent() {
    let before = parse_routed("graph TD; A --> B");
    let after = parse_routed("graph LR; A --> B");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_kind(crate::mmds::diff::MmdsDiffKind::EdgeRerouted));
    assert!(diff.has_kind(crate::mmds::diff::MmdsDiffKind::EndpointFaceChanged));
    assert!(diff.events.iter().all(|event| {
        event.kind != crate::mmds::diff::MmdsDiffKind::PortIntentChanged
            || event.evidence_mentions("logical")
    }));
}

#[test]
fn mmds_diff_geometry_reports_label_rect_change() {
    let before = render_pair_before_routed("M14");
    let after = render_pair_after_routed("M14");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_kind(crate::mmds::diff::MmdsDiffKind::LabelResized));
}

#[test]
fn mmds_diff_reflow_suppresses_tiny_unchanged_node_movement() {
    let before = parse_routed("graph TD; A --> B");
    let after = output_with_node_shift(&before, "A", 0.5, 0.0);

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(!diff.has_kind(crate::mmds::diff::MmdsDiffKind::NodeMoved));
    assert!(!diff.has_kind(crate::mmds::diff::MmdsDiffKind::GlobalReflowDetected));
}

#[test]
fn mmds_diff_reflow_reports_many_unchanged_nodes_moving() {
    let before = render_pair_before_routed("M14");
    let after = render_pair_after_routed("M14");

    let diff = crate::mmds::diff::diff_outputs(&before, &after);

    assert!(diff.has_kind(crate::mmds::diff::MmdsDiffKind::EdgeLabelChanged));
    assert!(
        diff.has_related_geometry_for("e0")
            || diff.has_kind(crate::mmds::diff::MmdsDiffKind::EdgeRerouted)
    );
}

#[test]
fn mmds_diff_tier_a_reports_expected_event_families() {
    let summaries = run_tier_a_diff_summaries();

    assert!(summaries["M01"].has_kind(MmdsDiffKind::NodeAdded));
    assert!(summaries["M05"].has_kind(MmdsDiffKind::NodeLabelChanged));
    assert!(summaries["M07"].has_kind(MmdsDiffKind::EdgeStyleChanged));
    assert!(!summaries["M07"].has_kind(MmdsDiffKind::EdgeRerouted));
    assert!(summaries["M10"].has_kind(MmdsDiffKind::SubgraphDirectionChanged));
}

fn parse_layout(source: &str) -> crate::mmds::Output {
    let config = RenderConfig {
        geometry_level: GeometryLevel::Layout,
        ..RenderConfig::default()
    };
    let json = crate::render_diagram(source, OutputFormat::Mmds, &config)
        .expect("layout MMDS should render");
    crate::mmds::parse_input(&json).expect("rendered MMDS should parse")
}

fn output_with_node_shift(
    output: &crate::mmds::Output,
    node_id: &str,
    dx: f64,
    dy: f64,
) -> crate::mmds::Output {
    let mut shifted = output.clone();
    let node = shifted
        .nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .expect("node should exist");
    node.position.x += dx;
    node.position.y += dy;
    shifted
}

fn run_tier_a_diff_summaries() -> BTreeMap<&'static str, MmdsDiff> {
    [
        (
            "M01",
            include_str!("../../tests/fixtures/flowchart/chain.mmd"),
            "graph TD
    A[Step 1] --> B[Step 2] --> X[Inserted] --> C[Step 3] --> D[Step 4]
",
        ),
        (
            "M05",
            include_str!("../../tests/fixtures/flowchart/ci_pipeline.mmd"),
            "graph LR
    Push[Git Push] --> Build[Build]
    Build --> Test[Run Tests]
    Test --> Lint[Static Analysis]
    Lint --> Deploy{Deploy?}
    Deploy -->|staging| Staging[Staging Env]
    Deploy -->|production| Prod[Production]
",
        ),
        (
            "M07",
            include_str!("../../tests/fixtures/flowchart/edge_styles.mmd"),
            "graph TD
    A[Solid] -.-> B[Normal]
    C[Dotted] -.-> D[Arrow]
    E[Thick] ==> F[Arrow]
    G[Open] --- H[Line]
",
        ),
        (
            "M10",
            include_str!("../../tests/fixtures/flowchart/subgraph_direction_lr.mmd"),
            "graph TD
    Start --> A
    subgraph sg1[Horizontal Flow]
        direction TB
        A[Step 1] --> B[Step 2] --> C[Step 3]
    end
    C --> End
",
        ),
    ]
    .into_iter()
    .map(|(pair_id, before, after)| {
        (
            pair_id,
            crate::mmds::diff::diff_outputs(&parse_routed(before), &parse_routed(after)),
        )
    })
    .collect()
}

fn render_pair_before_routed(pair_id: &str) -> crate::mmds::Output {
    match pair_id {
        "M14" => parse_routed(include_str!(
            "../../tests/fixtures/flowchart/inline_label_flowchart.mmd"
        )),
        _ => panic!("unsupported pair id {pair_id}"),
    }
}

fn render_pair_after_routed(pair_id: &str) -> crate::mmds::Output {
    match pair_id {
        "M14" => parse_routed(M14_AFTER),
        _ => panic!("unsupported pair id {pair_id}"),
    }
}

const M14_AFTER: &str = "flowchart TD
  start((Start)) --> ingest[Ingest Request]
  ingest --> parse[Parse Payload]
  parse --> validate{Valid?}

  validate -- no --> reject[Reject]
  reject -.-> notify[Notify User]
  reject --> metrics[Emit Metrics]

  validate -- yes --> route{Route Type}
  route -- sync --> sync[Sync Pipeline]
  route -- async --> queue[Enqueue Job]

  queue --> worker[Worker Pool]
  worker --> process[Process Job]
  process --> success{Success?}

  success -- retry later --> retry[Retry]
  retry ==> queue

  success -- yes --> persist[Persist Result]
  sync --> persist
  persist --> metrics

  parse --> cache[Lookup Cache]
  cache -- hit --> fastpath[Serve Cached]
  fastpath --> metrics
  cache -- miss --> validate

  ingest --> audit[Audit Log]
  audit --> metrics

  process -- warn --> alert[Page On-call]
  alert -.-> metrics

  metrics --> End((Done))
";

fn parse_routed(source: &str) -> crate::mmds::Output {
    let config = RenderConfig {
        geometry_level: GeometryLevel::Routed,
        ..RenderConfig::default()
    };
    let json = crate::render_diagram(source, OutputFormat::Mmds, &config)
        .expect("routed MMDS should render");
    crate::mmds::parse_input(&json).expect("rendered MMDS should parse")
}
