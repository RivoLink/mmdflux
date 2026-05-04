use serde_json::json;

use crate::mmds::Document;
use crate::mmds::commands::{Command, apply};
use crate::mmds::diff::{MmdsDiffEvent, MmdsDiffKind, MmdsDiffSubject, diff_outputs};
use crate::views::{Selector, ViewEvent, ViewSpec, ViewStatement, apply_view};
use crate::{RenderConfig, materialize_diagram};

#[derive(Debug)]
struct ProbeScenario {
    initial: Document,
    commands: Vec<ProbeCommandStep>,
    view_specs: Vec<ProbeViewSpec>,
}

#[derive(Debug)]
struct ProbeCommandStep {
    name: &'static str,
    command: Command,
    expected_kind: MmdsDiffKind,
    expected_subject: MmdsDiffSubject,
}

#[derive(Debug)]
struct ProbeViewSpec {
    name: &'static str,
    spec: ViewSpec,
}

#[derive(Debug)]
struct CommandProbeResult {
    command_observations: Vec<CommandObservation>,
    unexpected_failures: Vec<String>,
    final_document: Document,
}

#[derive(Debug)]
struct CommandObservation {
    command_name: &'static str,
    returned_event_pairs: Vec<(MmdsDiffKind, MmdsDiffSubject)>,
    returned_expected_primary: bool,
    error: Option<String>,
}

#[derive(Debug)]
struct CompositionProbeResult {
    command_result: CommandProbeResult,
    diff_observations: DiffObservations,
    view_observations: Vec<ViewObservation>,
    composition_observations: CompositionObservations,
}

#[derive(Debug)]
struct ViewObservation {
    name: &'static str,
    node_ids: Vec<String>,
    edge_ids: Vec<String>,
    subgraph_ids: Vec<String>,
    node_count: usize,
    metadata_bounds: (f64, f64),
    events: Vec<ViewEvent>,
    sparse_surviving_edge_ids_preserved: bool,
    error: Option<String>,
}

impl ViewObservation {
    fn contains_node(&self, node_id: &str) -> bool {
        self.node_ids.iter().any(|id| id == node_id)
    }
}

#[derive(Debug)]
struct CompositionObservations {
    unexpected_command_failures: Vec<String>,
    unexpected_missing_semantic_events: Vec<&'static str>,
    temporal_collapses: Vec<&'static str>,
    view_materialization_failures: Vec<String>,
    private_internal_requirements: Vec<String>,
    gating_notes: Vec<GatingNote>,
}

impl CompositionObservations {
    fn no_unexpected_failures(&self) -> bool {
        self.unexpected_command_failures.is_empty()
            && self.unexpected_missing_semantic_events.is_empty()
            && self.view_materialization_failures.is_empty()
    }
}

#[derive(Debug)]
struct GatingNote {
    primitive: &'static str,
    surface: &'static str,
}

#[derive(Debug)]
struct DiffObservations {
    raw_diff_event_pairs: Vec<(MmdsDiffKind, MmdsDiffSubject)>,
    command_classifications: Vec<CommandDiffClassification>,
    unexpected_extra_semantic_events: Vec<(MmdsDiffKind, MmdsDiffSubject)>,
    has_expected_stable_headlines: bool,
}

#[derive(Debug)]
struct CommandDiffClassification {
    command_name: &'static str,
    expected_kind: MmdsDiffKind,
    expected_subject: MmdsDiffSubject,
    classification: CommandDiffClassificationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandDiffClassificationKind {
    PresentInBoth,
    GeometryOnlySideEffect,
    TemporalCollapse,
    UnexpectedMissing,
}

fn build_probe_scenario() -> ProbeScenario {
    ProbeScenario {
        initial: document_from_mermaid(
            "graph TD
    gateway[Gateway] --> users[User Service]
    users --> db[(Database)]
    gateway --> billing[Billing Service]
    billing --> ledger[(Ledger)]
    docs[Docs Site] --> gateway
",
        ),
        commands: vec![
            ProbeCommandStep {
                name: "add-auth-subgraph",
                command: Command::AddSubgraph {
                    id: "auth".to_string(),
                    title: Some("Auth".to_string()),
                    parent: None,
                    direction: Some("LR".to_string()),
                    children: Vec::new(),
                    concurrent_regions: Vec::new(),
                    invisible: false,
                },
                expected_kind: MmdsDiffKind::SubgraphAdded,
                expected_subject: MmdsDiffSubject::Subgraph("auth".to_string()),
            },
            ProbeCommandStep {
                name: "add-login",
                command: Command::AddNode {
                    id: "login".to_string(),
                    label: "Login".to_string(),
                    shape: "rectangle".to_string(),
                    parent: Some("auth".to_string()),
                },
                expected_kind: MmdsDiffKind::NodeAdded,
                expected_subject: MmdsDiffSubject::Node("login".to_string()),
            },
            ProbeCommandStep {
                name: "move-users-into-auth",
                command: Command::ChangeSubgraphMembership {
                    subgraph: "auth".to_string(),
                    added_children: vec!["users".to_string()],
                    removed_children: Vec::new(),
                    added_concurrent_regions: Vec::new(),
                    removed_concurrent_regions: Vec::new(),
                },
                expected_kind: MmdsDiffKind::SubgraphMembershipChanged,
                expected_subject: MmdsDiffSubject::Subgraph("auth".to_string()),
            },
            ProbeCommandStep {
                name: "add-login-db-edge",
                command: Command::AddEdge {
                    id: None,
                    source: "login".to_string(),
                    target: "db".to_string(),
                    from_subgraph: None,
                    to_subgraph: None,
                    label: Some("reads".to_string()),
                    stroke: "solid".to_string(),
                    arrow_start: "none".to_string(),
                    arrow_end: "normal".to_string(),
                    minlen: 1,
                },
                expected_kind: MmdsDiffKind::EdgeAdded,
                expected_subject: MmdsDiffSubject::Edge("e5".to_string()),
            },
            ProbeCommandStep {
                name: "change-login-label",
                command: Command::ChangeNodeLabel {
                    node: "login".to_string(),
                    label: "Login Service :8443".to_string(),
                },
                expected_kind: MmdsDiffKind::NodeLabelChanged,
                expected_subject: MmdsDiffSubject::Node("login".to_string()),
            },
            ProbeCommandStep {
                name: "set-review-profile",
                command: Command::SetProfiles {
                    profiles: vec!["composition-probe".to_string()],
                },
                expected_kind: MmdsDiffKind::ProfileChanged,
                expected_subject: MmdsDiffSubject::Document,
            },
            ProbeCommandStep {
                name: "style-login",
                command: Command::SetNodeStyleExtension {
                    node: "login".to_string(),
                    value: json!({
                        "fill": "#fdf6e3",
                        "stroke": "#586e75",
                    }),
                },
                expected_kind: MmdsDiffKind::NodeStyleChanged,
                expected_subject: MmdsDiffSubject::Node("login".to_string()),
            },
        ],
        view_specs: vec![
            ProbeViewSpec {
                name: "auth-subgraph",
                spec: auth_view_spec(),
            },
            ProbeViewSpec {
                name: "full-diagram",
                spec: full_view_spec(),
            },
        ],
    }
}

fn document_from_mermaid(input: &str) -> Document {
    materialize_diagram(input, &RenderConfig::default()).expect("scenario should materialize")
}

fn auth_view_spec() -> ViewSpec {
    ViewSpec {
        statements: vec![ViewStatement::Include(Selector::SubgraphDescendants(
            "auth".to_string(),
        ))],
        ..ViewSpec::default()
    }
}

fn full_view_spec() -> ViewSpec {
    ViewSpec {
        statements: vec![ViewStatement::Include(Selector::All)],
        ..ViewSpec::default()
    }
}

fn run_command_probe(scenario: &ProbeScenario) -> CommandProbeResult {
    let mut document = scenario.initial.clone();
    let mut command_observations = Vec::new();
    let mut unexpected_failures = Vec::new();

    for step in &scenario.commands {
        match apply(&step.command, &mut document) {
            Ok(events) => {
                let returned_expected_primary =
                    has_event_pair(&events, step.expected_kind, &step.expected_subject);
                command_observations.push(CommandObservation {
                    command_name: step.name,
                    returned_event_pairs: event_pairs(&events),
                    returned_expected_primary,
                    error: None,
                });
            }
            Err(error) => {
                let message = format!("{} failed: {error:?}", step.name);
                unexpected_failures.push(message.clone());
                command_observations.push(CommandObservation {
                    command_name: step.name,
                    returned_event_pairs: Vec::new(),
                    returned_expected_primary: false,
                    error: Some(message),
                });
            }
        }
    }

    CommandProbeResult {
        command_observations,
        unexpected_failures,
        final_document: document,
    }
}

fn run_composed_probe(scenario: &ProbeScenario) -> CompositionProbeResult {
    let command_result = run_command_probe(scenario);
    let diff = diff_outputs(&scenario.initial, &command_result.final_document);
    let raw_diff_event_pairs = event_pairs(&diff.events);
    let semantic_diff_event_pairs = semantic_event_pairs(&diff.events);
    let command_expected_pairs: Vec<_> = scenario
        .commands
        .iter()
        .map(|step| (step.expected_kind, step.expected_subject.clone()))
        .collect();
    let mut created_subjects = Vec::new();
    let mut command_classifications = Vec::new();

    for step in &scenario.commands {
        let classification = classify_command_diff_relationship(
            step,
            &semantic_diff_event_pairs,
            &diff.events,
            &created_subjects,
        );

        if matches!(
            step.expected_kind,
            MmdsDiffKind::NodeAdded | MmdsDiffKind::EdgeAdded | MmdsDiffKind::SubgraphAdded
        ) {
            created_subjects.push(step.expected_subject.clone());
        }

        command_classifications.push(CommandDiffClassification {
            command_name: step.name,
            expected_kind: step.expected_kind,
            expected_subject: step.expected_subject.clone(),
            classification,
        });
    }

    let unexpected_extra_semantic_events = semantic_diff_event_pairs
        .iter()
        .filter(|(kind, subject)| {
            !command_expected_pairs
                .iter()
                .any(|(expected_kind, expected_subject)| {
                    kind == expected_kind && subject == expected_subject
                })
        })
        .cloned()
        .collect();
    let has_expected_stable_headlines = command_classifications.iter().all(|classification| {
        !matches!(
            classification.classification,
            CommandDiffClassificationKind::UnexpectedMissing
        )
    });

    let diff_observations = DiffObservations {
        raw_diff_event_pairs,
        command_classifications,
        unexpected_extra_semantic_events,
        has_expected_stable_headlines,
    };
    let view_observations = collect_view_observations(&command_result.final_document, scenario);
    let composition_observations =
        summarize_composition(&command_result, &diff_observations, &view_observations);

    CompositionProbeResult {
        command_result,
        diff_observations,
        view_observations,
        composition_observations,
    }
}

fn summarize_composition(
    command_result: &CommandProbeResult,
    diff_observations: &DiffObservations,
    view_observations: &[ViewObservation],
) -> CompositionObservations {
    CompositionObservations {
        unexpected_command_failures: command_result.unexpected_failures.clone(),
        unexpected_missing_semantic_events: diff_observations
            .command_classifications
            .iter()
            .filter(|classification| {
                matches!(
                    classification.classification,
                    CommandDiffClassificationKind::UnexpectedMissing
                )
            })
            .map(|classification| classification.command_name)
            .collect(),
        temporal_collapses: diff_observations
            .command_classifications
            .iter()
            .filter(|classification| {
                matches!(
                    classification.classification,
                    CommandDiffClassificationKind::TemporalCollapse
                )
            })
            .map(|classification| classification.command_name)
            .collect(),
        view_materialization_failures: view_observations
            .iter()
            .filter_map(|observation| {
                observation
                    .error
                    .as_ref()
                    .map(|error| format!("{} failed: {error}", observation.name))
            })
            .collect(),
        private_internal_requirements: vec![
            "diff_outputs is crate-internal and test-gated".to_string(),
            "commands::apply is crate-internal and test-gated".to_string(),
        ],
        gating_notes: vec![
            GatingNote {
                primitive: "views",
                surface: "public",
            },
            GatingNote {
                primitive: "diff",
                surface: "test-gated",
            },
            GatingNote {
                primitive: "commands",
                surface: "test-gated",
            },
        ],
    }
}

fn collect_view_observations(
    final_document: &Document,
    scenario: &ProbeScenario,
) -> Vec<ViewObservation> {
    scenario
        .view_specs
        .iter()
        .map(
            |probe_view| match apply_view(final_document, &probe_view.spec) {
                Ok((view, events)) => {
                    let canonical_edge_ids = edge_ids(final_document);
                    let retained_edge_ids = edge_ids(&view);
                    ViewObservation {
                        name: probe_view.name,
                        node_ids: node_ids(&view),
                        edge_ids: retained_edge_ids.clone(),
                        subgraph_ids: subgraph_ids(&view),
                        node_count: view.nodes.len(),
                        metadata_bounds: (view.metadata.bounds.width, view.metadata.bounds.height),
                        events,
                        sparse_surviving_edge_ids_preserved: retained_edge_ids
                            .iter()
                            .all(|id| canonical_edge_ids.contains(id))
                            && retained_edge_ids
                                .iter()
                                .enumerate()
                                .any(|(index, id)| id != &format!("e{index}")),
                        error: None,
                    }
                }
                Err(error) => ViewObservation {
                    name: probe_view.name,
                    node_ids: Vec::new(),
                    edge_ids: Vec::new(),
                    subgraph_ids: Vec::new(),
                    node_count: 0,
                    metadata_bounds: (0.0, 0.0),
                    events: Vec::new(),
                    sparse_surviving_edge_ids_preserved: false,
                    error: Some(format!("{error:?}")),
                },
            },
        )
        .collect()
}

fn classify_command_diff_relationship(
    step: &ProbeCommandStep,
    semantic_diff_event_pairs: &[(MmdsDiffKind, MmdsDiffSubject)],
    raw_diff_events: &[MmdsDiffEvent],
    created_subjects: &[MmdsDiffSubject],
) -> CommandDiffClassificationKind {
    if event_pair_is_present(
        semantic_diff_event_pairs,
        step.expected_kind,
        &step.expected_subject,
    ) {
        return CommandDiffClassificationKind::PresentInBoth;
    }

    if raw_diff_events
        .iter()
        .any(|event| event.kind.is_geometry_effect() && event.subject == step.expected_subject)
    {
        return CommandDiffClassificationKind::GeometryOnlySideEffect;
    }

    if created_subjects
        .iter()
        .any(|subject| subject == &step.expected_subject)
    {
        return CommandDiffClassificationKind::TemporalCollapse;
    }

    CommandDiffClassificationKind::UnexpectedMissing
}

fn has_event_pair(events: &[MmdsDiffEvent], kind: MmdsDiffKind, subject: &MmdsDiffSubject) -> bool {
    events
        .iter()
        .any(|event| event.kind == kind && &event.subject == subject)
}

fn event_pairs(events: &[MmdsDiffEvent]) -> Vec<(MmdsDiffKind, MmdsDiffSubject)> {
    events
        .iter()
        .map(|event| (event.kind, event.subject.clone()))
        .collect()
}

fn semantic_event_pairs(events: &[MmdsDiffEvent]) -> Vec<(MmdsDiffKind, MmdsDiffSubject)> {
    event_pairs(events)
        .into_iter()
        .filter(|(kind, _)| !kind.is_geometry_effect())
        .collect()
}

fn event_pair_is_present(
    pairs: &[(MmdsDiffKind, MmdsDiffSubject)],
    kind: MmdsDiffKind,
    subject: &MmdsDiffSubject,
) -> bool {
    pairs
        .iter()
        .any(|(event_kind, event_subject)| *event_kind == kind && event_subject == subject)
}

fn node_ids(document: &Document) -> Vec<String> {
    document.nodes.iter().map(|node| node.id.clone()).collect()
}

fn edge_ids(document: &Document) -> Vec<String> {
    document.edges.iter().map(|edge| edge.id.clone()).collect()
}

fn subgraph_ids(document: &Document) -> Vec<String> {
    document
        .subgraphs
        .iter()
        .map(|subgraph| subgraph.id.clone())
        .collect()
}

#[test]
fn composition_probe_scenario_covers_required_surfaces() {
    let scenario = build_probe_scenario();

    assert!(scenario.initial.nodes.len() >= 5);
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::AddNode { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::AddSubgraph { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::ChangeSubgraphMembership { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::AddEdge { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::ChangeNodeLabel { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::SetProfiles { .. }))
    );
    assert!(
        scenario
            .commands
            .iter()
            .any(|step| matches!(step.command, Command::SetNodeStyleExtension { .. }))
    );
    assert_eq!(scenario.view_specs.len(), 2);

    let add_login = scenario
        .commands
        .iter()
        .find(|step| step.name == "add-login")
        .expect("add-login command exists");
    assert_eq!(add_login.expected_kind, MmdsDiffKind::NodeAdded);
    assert_eq!(
        add_login.expected_subject,
        MmdsDiffSubject::Node("login".to_string())
    );
}

#[test]
fn composition_probe_command_log_records_expected_primary_events() {
    let scenario = build_probe_scenario();
    let result = run_command_probe(&scenario);

    assert!(result.unexpected_failures.is_empty(), "{result:#?}");
    assert!(
        result
            .final_document
            .nodes
            .iter()
            .any(|node| node.id == "login")
    );
    for observation in &result.command_observations {
        assert!(!observation.command_name.is_empty());
        assert!(observation.error.is_none(), "{observation:#?}");
        assert!(!observation.returned_event_pairs.is_empty());
        assert!(observation.returned_expected_primary, "{observation:#?}");
    }
}

#[test]
fn composition_probe_diff_cumulative_classifies_command_log_relationship() {
    let scenario = build_probe_scenario();
    let result = run_composed_probe(&scenario);

    assert!(
        result.diff_observations.has_expected_stable_headlines,
        "{result:#?}"
    );
    assert!(!result.diff_observations.raw_diff_event_pairs.is_empty());
    assert!(
        result
            .diff_observations
            .unexpected_extra_semantic_events
            .iter()
            .any(|(kind, subject)| {
                *kind == MmdsDiffKind::ExtensionChanged && *subject == MmdsDiffSubject::Document
            })
    );
    assert!(
        result
            .diff_observations
            .command_classifications
            .iter()
            .any(|classification| classification.command_name == "change-login-label"),
        "{result:#?}"
    );
    let label_classification = result
        .diff_observations
        .command_classifications
        .iter()
        .find(|classification| classification.command_name == "change-login-label")
        .expect("change-login-label classification exists");
    assert_eq!(
        label_classification.expected_kind,
        MmdsDiffKind::NodeLabelChanged
    );
    assert_eq!(
        label_classification.expected_subject,
        MmdsDiffSubject::Node("login".to_string())
    );
}

#[test]
fn composition_probe_view_materializes_filtered_and_full_views() {
    let scenario = build_probe_scenario();
    let result = run_composed_probe(&scenario);

    assert_eq!(result.view_observations.len(), 2, "{result:#?}");
    assert!(
        result
            .view_observations
            .iter()
            .any(|view| { view.name == "auth-subgraph" && view.contains_node("login") })
    );
    let auth_view = result
        .view_observations
        .iter()
        .find(|view| view.name == "auth-subgraph")
        .expect("auth-subgraph view exists");
    assert!(auth_view.edge_ids.is_empty());
    assert_eq!(auth_view.subgraph_ids, vec!["auth".to_string()]);
    assert!(auth_view.metadata_bounds.0 > 0.0);
    assert!(auth_view.metadata_bounds.1 > 0.0);
    assert!(!auth_view.events.is_empty());
    assert!(!auth_view.sparse_surviving_edge_ids_preserved);
    assert!(result.view_observations.iter().any(|view| {
        view.name == "full-diagram"
            && view.node_count == result.command_result.final_document.nodes.len()
    }));
}

#[test]
fn composition_probe_observations_include_gating_asymmetry() {
    let scenario = build_probe_scenario();
    let result = run_composed_probe(&scenario);

    assert!(
        result.composition_observations.no_unexpected_failures(),
        "{result:#?}"
    );
    assert!(
        result
            .composition_observations
            .gating_notes
            .iter()
            .any(|note| note.primitive == "views" && note.surface == "public")
    );
    assert!(
        result
            .composition_observations
            .gating_notes
            .iter()
            .any(|note| note.primitive == "diff" && note.surface == "test-gated")
    );
    assert!(
        result
            .composition_observations
            .gating_notes
            .iter()
            .any(|note| note.primitive == "commands" && note.surface == "test-gated")
    );
    assert!(
        result
            .composition_observations
            .temporal_collapses
            .contains(&"change-login-label")
    );
    assert!(
        result
            .composition_observations
            .private_internal_requirements
            .iter()
            .any(|requirement| requirement.contains("diff_outputs"))
    );

    eprintln!("{result:#?}");
}
