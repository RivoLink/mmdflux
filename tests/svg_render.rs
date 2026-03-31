use std::fs;
use std::path::Path;

use mmdflux::format::{CornerStyle, Curve, RoutingStyle};
use mmdflux::simplification::PathSimplification;
use mmdflux::{EngineAlgorithmId, OutputFormat, RenderConfig, render_diagram};

fn load_flowchart_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("flowchart")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read flowchart fixture {}: {e}", path.display()))
}

fn load_mmds_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("mmds")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read MMDS fixture {}: {e}", path.display()))
}

fn render_svg(input: &str, config: &RenderConfig) -> String {
    render_diagram(input, OutputFormat::Svg, config).expect("SVG render should succeed")
}

#[test]
fn basic_flowchart_svg_has_root_text_and_arrow_marker() {
    let input = "graph TD\nA[Start] --> B[End]\n";
    let svg = render_svg(input, &RenderConfig::default());

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Start"));
    assert!(svg.contains("End"));
    assert!(svg.contains("marker-end="));
    assert!(svg.contains("<path d=\""));
}

#[test]
fn svg_runtime_honors_supported_style_options() {
    let input = load_flowchart_fixture("complex.mmd");
    let svg = render_svg(
        &input,
        &RenderConfig {
            layout_engine: Some(
                EngineAlgorithmId::parse("flux-layered")
                    .expect("flux-layered engine id should parse"),
            ),
            routing_style: Some(RoutingStyle::Orthogonal),
            curve: Some(Curve::Linear(CornerStyle::Rounded)),
            path_simplification: PathSimplification::None,
            ..RenderConfig::default()
        },
    );

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<text"));
    assert!(svg.contains("<path d=\""));
}

#[test]
fn basic_sequence_svg_has_participants_and_arrows() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Bob-->>Alice: Hi\n";
    let svg = render_svg(input, &RenderConfig::default());

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob"));
    assert!(svg.contains("Hello"));
    assert!(svg.contains("Hi"));
    assert!(svg.contains("marker-end="));
    assert!(svg.contains("stroke-dasharray=\"5,5\"")); // lifelines
    assert!(svg.contains("stroke-dasharray=\"6,4\"")); // dashed message
}

#[test]
fn sequence_svg_self_message_renders_path() {
    let input = "sequenceDiagram\n    Alice->>Alice: Think\n";
    let svg = render_svg(input, &RenderConfig::default());

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<path d=\"M"));
    assert!(svg.contains("Think"));
}

#[test]
fn sequence_svg_note_renders_note_box() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    Note right of Bob: Important\n";
    let svg = render_svg(input, &RenderConfig::default());

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Important"));
    assert!(svg.contains("#ffffcc")); // note fill color
}

#[test]
fn sequence_svg_activation_renders_rect() {
    let input = "sequenceDiagram\n    Alice->>Bob: Hello\n    activate Bob\n    Bob-->>Alice: Hi\n    deactivate Bob\n";
    let svg = render_svg(input, &RenderConfig::default());

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("activations")); // group class
    assert!(svg.contains("#ddd")); // activation fill
}

#[test]
fn positioned_mmds_payload_renders_svg_through_runtime() {
    let payload = load_mmds_fixture("positioned/routed-fan-in-ports.json");
    let svg = render_svg(
        &payload,
        &RenderConfig {
            path_simplification: PathSimplification::None,
            ..RenderConfig::default()
        },
    );

    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("marker-end="));
    assert!(svg.contains("<path d=\""));
}
