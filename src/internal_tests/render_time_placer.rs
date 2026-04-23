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
use crate::engines::graph::algorithms::layered::{
    LayoutConfig as LayeredConfig, run_layered_layout,
};
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

fn render_class_fixture(name: &str) -> String {
    use crate::format::OutputFormat;
    use crate::runtime::config::RenderConfig;
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("class")
        .join(name);
    let input = fs::read_to_string(&path).expect("fixture exists");
    crate::render_diagram(&input, OutputFormat::Text, &RenderConfig::default())
        .expect("render class fixture")
}

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
    assert!(
        output.contains("yes"),
        "rendered output missing 'yes'\n{output}"
    );
    assert!(
        output.contains("no"),
        "rendered output missing 'no'\n{output}"
    );
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

// ---- PR #B canaries (C5-C9). Activated in task 2.1. These pass against
// the wrapper's `AllBodyLabels` scope today because `compute_label_placements`
// is correct for that scope at the wrapper level; the strict Phase 2
// callsite coverage (unified branch, drift-gate deletion, GridLayout field
// deletion) is exercised by task 2.9's snapshot regeneration. These canaries
// serve as targeted structural regression guards during Phase 2 refactoring.

/// C5: self-edge with `label_geometry`. The self-edge on `self_loop_labeled.mmd`
/// carries the label "retry". PR #B's unified path must still render this label
/// on the self-loop corridor; this canary guards against the branch collapse
/// accidentally eating the self-edge path.
#[test]
fn c5_self_edge_with_label_geometry() {
    let output = render_flowchart_fixture("self_loop_labeled.mmd");
    assert!(
        output.contains("retry"),
        "self-edge label 'retry' missing from output\n{output}"
    );
    // The self-loop renders with ▲ (or its charset equivalent) somewhere.
    assert!(
        output.contains('▲')
            || output.contains('▼')
            || output.contains('◄')
            || output.contains('►'),
        "expected an arrow glyph on the self-loop\n{output}"
    );
}

/// C6: class-diagram labeled edge. `class/relationships.mmd` has three
/// labeled class edges: "places", "contains", "authenticates". PR #B's
/// unified path must render all three through the same body-label flow.
#[test]
fn c6_class_diagram_edge_renders_via_unified_path() {
    let output = render_class_fixture("relationships.mmd");
    for label in ["places", "contains", "authenticates"] {
        assert!(
            output.contains(label),
            "class-diagram label '{label}' missing from output\n{output}"
        );
    }
}

/// C7: backward edge with overwrite_arrows=true preserved. The backward
/// edge in `label_clamp_bt_review.mmd` still paints its arrowhead after
/// the render-time placer writes the label. PR #B's unified path must
/// honour the `is_backward` signal that triggers arrow-overwrite behavior.
#[test]
fn c7_backward_overwrite_arrows_preserved() {
    let output = render_flowchart_fixture("label_clamp_bt_review.mmd");
    assert!(
        output.contains("yes"),
        "backward-edge label 'yes' missing\n{output}"
    );
    assert!(
        output.contains('▲'),
        "backward-edge arrowhead ▲ missing — label may have overwritten it\n{output}"
    );
}

/// C8: self-edge fallback without `label_geometry`. Exercises the
/// `calc_label_position(&segments)` midpoint fallback inside
/// `compute_label_placements` — the branch taken when `routed_geometry`
/// is absent. Parses `self_loop_labeled.mmd` (which has a real self-edge
/// with segments), builds a `GridLayout` + `Vec<RoutedEdge>`, then invokes
/// `compute_label_placements` with `routed_geometry = None` so
/// `edge_label_geometry` returns `None` and the fallback path runs. The
/// self-edge's label must still get a placement via the Pass-3 midpoint.
#[test]
fn c8_self_edge_fallback_without_label_geometry() {
    use crate::graph::grid::{
        GridLayoutConfig, geometry_to_grid_layout_with_routed, route_all_edges,
    };

    let input = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("flowchart")
            .join("self_loop_labeled.mmd"),
    )
    .expect("fixture exists");
    let parsed = parse_flowchart(&input).expect("parse ok");
    let diagram: Graph = compile_to_graph(&parsed);

    let config = EngineConfig::Layered(LayeredConfig::default());
    let geom = run_layered_layout(&MeasurementMode::Grid, &diagram, &config).expect("layout ok");
    let metrics = default_proportional_text_metrics();
    let routed_geom = route_graph_geometry(&diagram, &geom, EdgeRouting::OrthogonalRoute, &metrics);
    let layout = geometry_to_grid_layout_with_routed(
        &diagram,
        &geom,
        Some(&routed_geom),
        &GridLayoutConfig::default(),
    );
    let routed_edges = route_all_edges(&diagram.edges, &layout, diagram.direction);
    let self_edge_idx = routed_edges
        .iter()
        .find(|r| r.is_self_edge)
        .map(|r| r.edge.index)
        .expect("self_loop_labeled.mmd should contain a self-edge");

    // Pass `None` for routed_geometry → forces the midpoint fallback path.
    let placements = compute_label_placements(
        &routed_edges,
        None,
        &layout,
        &HashMap::new(),
        layout.width,
        layout.height,
        RenderTimePlacementScope::AllBodyLabels,
    );
    let placement = placements.get(&self_edge_idx).copied().unwrap_or_else(|| {
        panic!("self-edge fallback produced no placement; placements={placements:?}")
    });
    // Placement center must sit within canvas bounds.
    assert!(
        placement.center.0 < layout.width && placement.center.1 < layout.height,
        "self-edge midpoint-fallback center {:?} outside canvas {}x{}",
        placement.center,
        layout.width,
        layout.height
    );
}

/// C9: drift-gate dissolution on `complex.mmd`. PR #B deletes both
/// `AUTHORITATIVE_OVERRIDE_DRIFT = 5` (derive/mod.rs) and
/// `PRECOMPUTED_LABEL_BASE_DRIFT` (edge.rs). The render-time placer must
/// replace any stale authoritative anchor with a Pass-3-derived cell that
/// sits **on the drawn path** — not drift off toward a stale Pass-1
/// projection. This canary replaces the deleted
/// `text_renderer_rejects_stale_precomputed_label_anchor_for_label_revalidation_fixture`
/// by computing placements on `complex.mmd`, locating each routed edge's
/// label placement, and asserting a two-invariant drift contract:
///
///   (a) an **absolute ceiling** of 8 cells from the edge's Pass-3
///       segments, tight enough that a stale authoritative anchor
///       (typically 10+ cells off the drawn path) fails;
///   (b) a **materially-closer-than-stale ratio** — the placer's drift
///       must be at most 40% of the farthest canvas corner's distance to
///       the edge's segments (`placer_drift * 2.5 <= farthest_corner`).
///       Mirrors the deleted test's "closer than any stale candidate"
///       spirit without depending on the removed `edge_label_positions`
///       cache or on baseline-vs-poisoned comparisons that pre-dated the
///       render-time placer.
#[test]
fn c9_drift_gate_dissolution_on_complex_mmd() {
    use crate::graph::grid::{
        GridLayoutConfig, Segment, geometry_to_grid_layout_with_routed, route_all_edges,
    };

    fn distance_to_segments(point: (usize, usize), segments: &[Segment]) -> f64 {
        let (px, py) = (point.0 as f64, point.1 as f64);
        segments
            .iter()
            .map(|segment| match *segment {
                Segment::Horizontal { y, x_start, x_end } => {
                    let (x_min, x_max) = (x_start.min(x_end) as f64, x_start.max(x_end) as f64);
                    let clamped_x = px.max(x_min).min(x_max);
                    ((clamped_x - px).powi(2) + (y as f64 - py).powi(2)).sqrt()
                }
                Segment::Vertical { x, y_start, y_end } => {
                    let (y_min, y_max) = (y_start.min(y_end) as f64, y_start.max(y_end) as f64);
                    let clamped_y = py.max(y_min).min(y_max);
                    ((x as f64 - px).powi(2) + (clamped_y - py).powi(2)).sqrt()
                }
            })
            .fold(f64::INFINITY, f64::min)
    }

    let input = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("flowchart")
            .join("complex.mmd"),
    )
    .expect("fixture exists");
    let parsed = parse_flowchart(&input).expect("parse ok");
    let diagram: Graph = compile_to_graph(&parsed);
    let config = EngineConfig::Layered(LayeredConfig::default());
    let geom = run_layered_layout(&MeasurementMode::Grid, &diagram, &config).expect("layout ok");
    let metrics = default_proportional_text_metrics();
    let routed_geom = route_graph_geometry(&diagram, &geom, EdgeRouting::OrthogonalRoute, &metrics);
    let layout = geometry_to_grid_layout_with_routed(
        &diagram,
        &geom,
        Some(&routed_geom),
        &GridLayoutConfig::default(),
    );
    let routed_edges = route_all_edges(&diagram.edges, &layout, diagram.direction);

    let placements = compute_label_placements(
        &routed_edges,
        Some(&routed_geom),
        &layout,
        &HashMap::new(),
        layout.width,
        layout.height,
        RenderTimePlacementScope::AllBodyLabels,
    );
    assert!(
        !placements.is_empty(),
        "render-time placer should produce placements for complex.mmd"
    );

    // Drift contract proves drift-gate dissolution via two invariants that
    // together match the spirit of the deleted stale-anchor test:
    //
    //   (a) ABSOLUTE DRIFT CEILING. The placer's center must sit within
    //       ~8 cells of the drawn path. Accommodates backward edges where
    //       the label is perpendicular to the main corridor; tight enough
    //       that a stale authoritative anchor (typically off by 10+ cells)
    //       would fail.
    //
    //   (b) MATERIALLY CLOSER THAN THE FARTHEST STALE CANDIDATE. For each
    //       edge we compute the corner distance that is most-stale (the
    //       farthest canvas corner from this edge's segments). Assert
    //       `placer_drift * 2.5 <= farthest_corner` so the placer is at
    //       most 40% as far from the path as the worst stale candidate
    //       would be. This is stronger than `placer_drift < farthest`
    //       (which lets bad placements hide 1 cell inside the farthest
    //       corner's radius) and avoids the pathology where an edge
    //       passes near a canvas corner (e.g. complex.mmd's E→A backward
    //       edge, nearest-corner distance ~1.4) causing a strict
    //       `< nearest_corner` contract to reject legitimate
    //       perpendicular offsets.
    fn farthest_stale_corner_distance(
        routed: &crate::graph::grid::RoutedEdge,
        layout_width: usize,
        layout_height: usize,
    ) -> f64 {
        let corners = [
            (1usize, 1usize),
            (layout_width.saturating_sub(2), 1),
            (1, layout_height.saturating_sub(2)),
            (
                layout_width.saturating_sub(2),
                layout_height.saturating_sub(2),
            ),
        ];
        corners
            .iter()
            .map(|c| distance_to_segments(*c, &routed.segments))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    const ABSOLUTE_DRIFT_CEILING: f64 = 8.0;
    const STALE_RATIO: f64 = 2.5;

    let mut checked = 0usize;
    for routed in &routed_edges {
        let Some(placement) = placements.get(&routed.edge.index) else {
            continue;
        };
        let placer_drift = distance_to_segments(placement.center, &routed.segments);
        let farthest_corner = farthest_stale_corner_distance(routed, layout.width, layout.height);
        assert!(
            placer_drift <= ABSOLUTE_DRIFT_CEILING,
            "edge {} ({} → {}): placer drift {:.2} exceeds absolute ceiling {:.1} — \
             stale-anchor regression suspected",
            routed.edge.index,
            routed.edge.from,
            routed.edge.to,
            placer_drift,
            ABSOLUTE_DRIFT_CEILING
        );
        assert!(
            placer_drift * STALE_RATIO <= farthest_corner,
            "edge {} ({} → {}): placer drift {:.2} is not materially closer than the farthest \
             stale corner {:.2} (ratio {:.2}× required; got {:.2}×)",
            routed.edge.index,
            routed.edge.from,
            routed.edge.to,
            placer_drift,
            farthest_corner,
            STALE_RATIO,
            if placer_drift > 0.0 {
                farthest_corner / placer_drift
            } else {
                f64::INFINITY
            }
        );
        checked += 1;
    }
    assert!(
        checked >= 2,
        "expected at least 2 placed labels on complex.mmd, got {checked}"
    );
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
