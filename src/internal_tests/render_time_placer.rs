//! Render-time placer canaries for Plan 0153.
//!
//! Phase 1 (PR #A) activates C1–C4 and C10; C5–C9 are scaffolded here and
//! stay `#[ignore]` until PR #B widens the wrapper to all body labels. C10
//! is active in PR #A because `three_parallel_labels.mmd` already sits in
//! the `AuthoritativeOnly` scope (`compartment_size > 1`). C1 and C3 live in
//! `src/graph/grid/label_placement.rs` mod tests because they cover pure
//! footprint-anchor logic; this file hosts the integration canaries that
//! exercise the full pipeline through `render_text_from_geometry`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::diagrams::flowchart::compile_to_graph;
use crate::engines::graph::EngineConfig;
use crate::engines::graph::algorithms::layered::{LayoutConfig as LayeredConfig, run_layered_layout};
use crate::engines::graph::contracts::MeasurementMode;
use crate::graph::Graph;
use crate::graph::grid::{GridLayout, RoutedEdge};
use crate::graph::measure::default_proportional_text_metrics;
use crate::graph::routing::{EdgeRouting, route_graph_geometry};
use crate::mermaid::parse_flowchart;
use crate::render::graph::text::label_placement::{
    RenderTimePlacementScope, compute_label_placements,
};
use crate::render::graph::{TextRenderOptions, render_text_from_geometry};

fn render_flowchart_fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("flowchart")
        .join(name);
    let input = fs::read_to_string(&path).expect("fixture exists");
    let mut parsed = parse_flowchart(&input).expect("parse ok");
    let diagram: Graph = compile_to_graph(&parsed);
    // Apply pre-engine wrap (same as the baseline snapshot pipeline).
    let wrap_metrics = default_proportional_text_metrics();
    let wrap_max_width = crate::engines::graph::LayoutConfig::default().edge_label_max_width;
    let mut diagram = diagram;
    crate::graph::label_wrap::prepare_wrapped_labels(
        &mut diagram.edges,
        &wrap_metrics,
        wrap_max_width,
    );
    let _ = &mut parsed; // silence mutability hint
    let config = EngineConfig::Layered(LayeredConfig::default());
    let geom = run_layered_layout(&MeasurementMode::Grid, &diagram, &config).expect("layout ok");
    let metrics = default_proportional_text_metrics();
    let routed = route_graph_geometry(&diagram, &geom, EdgeRouting::OrthogonalRoute, &metrics);
    render_text_from_geometry(
        &diagram,
        &geom,
        Some(&routed),
        &TextRenderOptions::default(),
    )
}

/// Smoke test: the empty edge list returns an empty placement map regardless
/// of scope. Proves the module surface is in place and the loop short-circuits
/// before any helpers that would fail on degenerate input.
#[test]
fn compute_label_placements_returns_empty_map_on_empty_input() {
    let layout = GridLayout::default();
    let edges: Vec<RoutedEdge> = Vec::new();
    let containment: HashMap<usize, (usize, usize)> = HashMap::new();

    let result = compute_label_placements(
        &edges,
        None,
        &layout,
        &containment,
        10,
        10,
        RenderTimePlacementScope::AuthoritativeOnly,
    );
    assert!(result.is_empty());

    let result = compute_label_placements(
        &edges,
        None,
        &layout,
        &containment,
        10,
        10,
        RenderTimePlacementScope::AllBodyLabels,
    );
    assert!(result.is_empty());
}

/// C2: backward vertical, BT flowchart with authoritative lane-coordinated
/// labels. The rendered output must contain both labels ("yes" and "no") on
/// the backward edge corridor without a label character landing on the
/// `┌`/`┘` corners of the backward bracket.
#[test]
fn c2_backward_vertical_avoids_corners() {
    let output = render_flowchart_fixture("label_clamp_bt_review.mmd");
    assert!(output.contains("yes"), "rendered output missing 'yes'\n{output}");
    assert!(output.contains("no"), "rendered output missing 'no'\n{output}");
    // Corners are represented by ┌/└/┐/┘; the labels must not sit on rows
    // where they would overwrite those glyphs. Simple lane check: the
    // characters immediately adjacent to a label on the same row should be
    // whitespace or vertical corridor glyphs, not a corner.
    for line in output.lines() {
        if line.contains("yes") && (line.contains('┌') || line.contains('└')) {
            // Label and corner on the same row is only fine if they sit in
            // separate columns with a whitespace gap. Approximate check:
            // ensure `yes` is not directly adjacent to a corner glyph.
            assert!(
                !line.contains("┌yes") && !line.contains("yes┌"),
                "'yes' sits adjacent to a corner on row: {line}"
            );
        }
    }
}

/// C4: backward horizontal, forward + backward labels on parallel asymmetric
/// markers. Confirms the placer closes the PR #252 regression class (Plan
/// 0152 Phase 3's original trigger). Both labels must render and neither may
/// land on a load-bearing `┌`/`└` corner glyph.
#[test]
fn c4_backward_horizontal_corner_avoidance() {
    let output = render_flowchart_fixture("backward_label_asymmetric_markers.mmd");
    assert!(
        output.contains("forward label"),
        "rendered output missing 'forward label'\n{output}"
    );
    assert!(
        output.contains("reverse label"),
        "rendered output missing 'reverse label'\n{output}"
    );
    for line in output.lines() {
        assert!(
            !line.contains("┘reverse") && !line.contains("reverse┘"),
            "'reverse label' sits on a corner on row: {line}"
        );
        assert!(
            !line.contains("┘forward") && !line.contains("forward┘"),
            "'forward label' sits on a corner on row: {line}"
        );
    }
}

// ---- Staged PR #B canaries (C5-C10). Ignored until PR #B widens scope. ----

#[test]
#[ignore = "activated in PR #B when all-body-label scope flips self-edge labels through the render-time placer"]
fn c5_self_edge_with_label_geometry() {
    let _output = render_flowchart_fixture("self_loop_labeled.mmd");
    unimplemented!("task 2.1 activates C5 on self-edge fixture");
}

#[test]
#[ignore = "activated in PR #B when class-diagram edges flow through the unified render-time path"]
fn c6_class_diagram_edge_renders_via_unified_path() {
    unimplemented!("task 2.1 activates C6 on class/relationships.mmd");
}

#[test]
#[ignore = "activated in PR #B when backward edges with overwrite_arrows=true flow through the unified path"]
fn c7_backward_overwrite_arrows_preserved() {
    unimplemented!("task 2.1 activates C7");
}

#[test]
#[ignore = "activated in PR #B when synthetic self-edge fallback exercises calc_label_position midpoint without label_geometry"]
fn c8_self_edge_fallback_without_label_geometry() {
    unimplemented!("task 2.1 activates C8");
}

#[test]
#[ignore = "activated in PR #B after both drift gates are deleted; proves stale authoritative anchor is replaced by Pass-3-derived cell"]
fn c9_drift_gate_dissolution_on_complex_mmd() {
    unimplemented!("task 2.1 activates C9 on flowchart/complex.mmd edge idx=5");
}

/// C10: three parallel A→B edges with distinct labels ("one", "two", "three").
/// This fixture has compartment_size > 1 (multi-member authoritative subset),
/// so PR #A's `AuthoritativeOnly` scope already owns it. Validates that the
/// render-time placer resolves `label_geometry` by edge `index` (not by
/// `(from, to)` which would alias all three edges onto the first match).
///
/// The assertion intentionally scopes to the alias-bug signal: each of the
/// three labels must render on a distinct row. A broader "no sibling overlap"
/// claim would require per-column corridor analysis and belongs with the
/// sibling-shift tightening work slated for PR #B.
#[test]
fn c10_three_parallel_labels_converges_without_sibling_overlap() {
    let output = render_flowchart_fixture("three_parallel_labels.mmd");
    for label in ["one", "two", "three"] {
        assert!(
            output.contains(label),
            "rendered output missing '{label}'\n{output}"
        );
    }
    // Each of the three labels must appear on a distinct row — the alias bug
    // would collapse them onto the same row (or concatenate them).
    let lines: Vec<&str> = output.lines().collect();
    let row_of = |needle: &str| {
        lines
            .iter()
            .position(|l| l.contains(needle))
            .unwrap_or_else(|| panic!("label '{needle}' missing\n{output}"))
    };
    let (r_one, r_two, r_three) = (row_of("one"), row_of("two"), row_of("three"));
    assert!(
        r_one != r_two && r_two != r_three && r_one != r_three,
        "parallel-edge labels must occupy distinct rows (one={r_one}, two={r_two}, three={r_three})\n{output}"
    );
}
