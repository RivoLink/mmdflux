use mmdflux_wasm::{detect, render, render_with_browser_text_metrics, validate, version};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn error_debug(error: wasm_bindgen::JsError) -> String {
    format!("{error:?}")
}

fn strip_ansi(input: &str) -> String {
    let mut stripped = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && matches!(chars.peek(), Some('[')) {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }

        stripped.push(ch);
    }

    stripped
}

fn dynamic_metrics_json_fixture() -> &'static str {
    r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24}"#
}

fn callback(body: &str) -> js_sys::Function {
    js_sys::Function::new_with_args("text, cssFont", body)
}

#[wasm_bindgen_test]
fn renders_flowchart_text() {
    let output = render("graph TD\nA-->B", "text", "{}").expect("render should succeed");
    assert!(output.contains("A"));
    assert!(output.contains("B"));
}

#[wasm_bindgen_test]
fn renders_flowchart_text_with_color_policy_config() {
    let input = include_str!("../../../tests/fixtures/flowchart/style-basic.mmd");
    let plain = render(input, "text", r#"{"color":"off"}"#)
        .expect("render with plain color config should succeed");
    let auto = render(input, "text", r#"{"color":"auto"}"#)
        .expect("render with auto color should succeed");
    let ansi = render(input, "text", r#"{"color":"always"}"#)
        .expect("render with ansi color config should succeed");

    assert!(!plain.contains("\u{1b}["));
    assert!(!auto.contains("\u{1b}["));
    assert!(ansi.contains("38;2;"));
    assert!(ansi.contains("48;2;"));
    assert_eq!(plain, auto);
    assert_eq!(strip_ansi(&ansi), plain);
}

#[wasm_bindgen_test]
fn renders_flowchart_svg() {
    let output = render("graph TD\nA-->B", "svg", "{}").expect("svg render should succeed");
    assert!(output.contains("<svg"));
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_changes_svg_geometry() {
    let input = "graph TD\nA -->|mmmm| B";
    let static_output = render(input, "svg", "{}").expect("static svg should render");
    let measure = callback("return text.includes('m') ? 100 : 8;");
    let dynamic_output = render_with_browser_text_metrics(
        input,
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        &measure,
    )
    .expect("dynamic svg should render");

    assert_ne!(dynamic_output, static_output);
    assert!(dynamic_output.contains("<svg"));
    assert!(dynamic_output.contains("width=\"108.00\""));
}

#[wasm_bindgen_test]
fn static_render_export_stays_byte_stable_after_dynamic_render() {
    let input = include_str!("../../../tests/fixtures/flowchart/labeled_edges.mmd");
    let before = render(input, "svg", "{}").expect("static svg should render before dynamic");
    let measure = callback("return text.length * 9;");
    let dynamic = render_with_browser_text_metrics(
        "graph TD\nA -->|mmmm| B",
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        &measure,
    )
    .expect("dynamic svg should render");
    assert!(dynamic.contains("<svg"));
    let after = render(input, "svg", "{}").expect("static svg should render after dynamic");

    assert_eq!(after, before);
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_throw_errors() {
    let measure = callback("throw new Error('canvas failed');");
    let err = render_with_browser_text_metrics(
        "graph TD\nA-->B",
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        &measure,
    )
    .expect_err("callback throw should fail");

    assert!(error_debug(err).contains("canvas failed"));
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_rejects_non_number_returns() {
    for (body, expected) in [
        ("return Promise.resolve(12);", "synchronous"),
        ("return { width: 12 };", "return a number"),
        ("return '12';", "return a number"),
        ("return null;", "return a number"),
        ("return undefined;", "return a number"),
    ] {
        let measure = callback(body);
        let err = render_with_browser_text_metrics(
            "graph TD\nA-->B",
            "svg",
            "{}",
            dynamic_metrics_json_fixture(),
            &measure,
        )
        .expect_err("non-number callback return should fail");

        let message = error_debug(err);
        assert!(message.contains(expected), "{message}");
    }
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_rejects_invalid_widths() {
    for body in ["return Number.NaN;", "return Infinity;", "return -1;"] {
        let measure = callback(body);
        let err = render_with_browser_text_metrics(
            "graph TD\nA-->B",
            "svg",
            "{}",
            dynamic_metrics_json_fixture(),
            &measure,
        )
        .expect_err("invalid width should fail");

        let message = error_debug(err);
        assert!(message.contains("finite non-negative"), "{message}");
    }
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_failure_does_not_fallback_to_static_metrics() {
    let measure = callback(
        "if (text === 'mmmm') { throw new Error('missing glyph mmmm'); } return text.length * 8;",
    );
    let err = render_with_browser_text_metrics(
        "graph TD\nA -->|mmmm| B",
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        &measure,
    )
    .expect_err("callback failure should fail the dynamic render");

    let message = error_debug(err);
    assert!(message.contains("missing glyph mmmm"), "{message}");
    assert!(message.contains("mmmm"), "{message}");
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_reentry_errors_cleanly() {
    let inner = callback("return 8;");
    let closure = wasm_bindgen::closure::Closure::<dyn FnMut(String, String) -> f64>::new(
        move |_text, _css_font| {
            let _ = render_with_browser_text_metrics(
                "graph TD\nA-->B",
                "svg",
                "{}",
                dynamic_metrics_json_fixture(),
                &inner,
            );
            8.0
        },
    );
    let measure = closure.as_ref().unchecked_ref::<js_sys::Function>();
    let err = render_with_browser_text_metrics(
        "graph TD\nA-->B",
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        measure,
    )
    .expect_err("dynamic render re-entry should fail");

    let message = error_debug(err);
    assert!(message.contains("re-entered"), "{message}");
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_callback_can_call_validate_without_corruption() {
    let closure = wasm_bindgen::closure::Closure::<dyn FnMut(String, String) -> f64>::new(
        move |_text, _css_font| {
            let result = validate("graph TD\nA-->B");
            assert!(result.contains("\"valid\":true"));
            8.0
        },
    );
    let measure = closure.as_ref().unchecked_ref::<js_sys::Function>();
    let output = render_with_browser_text_metrics(
        "graph TD\nA-->B",
        "svg",
        "{}",
        dynamic_metrics_json_fixture(),
        measure,
    )
    .expect("validate re-entry should not corrupt dynamic render");

    assert!(output.contains("<svg"));
}

#[wasm_bindgen_test]
fn dynamic_text_metrics_rejects_mmds_output() {
    let measure = callback("return text.length * 8;");
    let err = render_with_browser_text_metrics(
        "graph TD\nA-->B",
        "mmds",
        "{}",
        dynamic_metrics_json_fixture(),
        &measure,
    )
    .expect_err("dynamic browser metrics should remain SVG-only");

    let message = error_debug(err);
    assert!(message.contains("only supports SVG output"), "{message}");
}

#[wasm_bindgen_test]
fn renders_svg_with_font_metrics_profile_config() {
    for profile in ["mmdflux-heuristic-proportional-v1", "mmdflux-sans-v1"] {
        let output = render(
            "graph TD\nA[mmmm]-->B[iiii]",
            "svg",
            &format!(r#"{{"fontMetricsProfile":"{profile}"}}"#),
        )
        .expect("font metrics profile config should render");
        assert!(output.contains("<svg"));
    }
}

#[wasm_bindgen_test]
fn rejects_unsupported_font_metrics_profile_config() {
    let error = render(
        "graph TD\nA-->B",
        "svg",
        r#"{"fontMetricsProfile":"unknown-profile-v1"}"#,
    )
    .expect_err("unsupported font metrics profile should fail");
    assert!(error_debug(error).contains("unsupported text metrics profile 'unknown-profile-v1'"));
}

#[wasm_bindgen_test]
fn rejects_legacy_edge_routing_config_key() {
    let error = render(
        "graph TD\nA-->B",
        "svg",
        r#"{"edgeRouting":"orthogonal-preview"}"#,
    )
    .expect_err("legacy edgeRouting should be rejected");
    assert!(error_debug(error).contains("unknown field"));
}

#[wasm_bindgen_test]
fn detect_returns_flowchart_id() {
    assert_eq!(detect("graph TD\nA-->B"), Some("flowchart".to_string()));
}

#[wasm_bindgen_test]
fn detect_returns_none_for_unknown_input() {
    assert_eq!(detect("this is not mermaid"), None);
}

#[wasm_bindgen_test]
fn rejects_unknown_format() {
    let error = render("graph TD\nA-->B", "nope", "{}").expect_err("unknown format must error");
    assert!(error_debug(error).contains("unknown output format"));
}

#[wasm_bindgen_test]
fn rejects_unknown_diagram_type() {
    let error = render("not mermaid at all", "text", "{}").expect_err("unknown diagram must fail");
    assert!(error_debug(error).contains("unknown diagram type"));
}

#[wasm_bindgen_test]
fn rejects_invalid_config_json() {
    let error = render("graph TD\nA-->B", "text", "{").expect_err("invalid config must fail");
    assert!(error_debug(error).contains("invalid config_json"));
}

#[wasm_bindgen_test]
fn rejects_legacy_edge_style_config_key() {
    let error = render("graph TD\nA-->B", "svg", r#"{"edgeStyle":"straight"}"#)
        .expect_err("legacy edgeStyle should be rejected");
    assert!(error_debug(error).contains("unknown field"));
}

#[wasm_bindgen_test]
fn applies_geometry_level_and_path_simplification_for_mmds() {
    let output = render(
        "graph TD\nA-->B",
        "mmds",
        r#"{"geometryLevel":"routed","pathSimplification":"minimal"}"#,
    )
    .expect("mmds render with geometry/path config should succeed");
    assert!(output.contains("\"path\""));
}

#[wasm_bindgen_test]
fn version_matches_package_version() {
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}
