//! Experimental dynamic text metrics contract for browser/WASM adapters.
//!
//! This module is feature-gated and doc-hidden because its callback-backed
//! provider API is not part of the stable Rust facade.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;

use serde::Deserialize;

use crate::builtins::default_registry;
use crate::errors::RenderError;
use crate::format::OutputFormat;
use crate::frontends::{InputFrontend, detect_input_frontend};
use crate::graph::measure::{
    DEFAULT_LABEL_PADDING_X, DEFAULT_LABEL_PADDING_Y, DEFAULT_PROPORTIONAL_NODE_PADDING_X,
    DEFAULT_PROPORTIONAL_NODE_PADDING_Y, TextMetricsProvider,
};
use crate::payload::Diagram;
use crate::registry::DiagramFamily;
use crate::runtime::config::RenderConfig;
use crate::runtime::config_input::apply_svg_surface_defaults;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DynamicMetricsInput {
    pub css_font: String,
    pub font_family: String,
    pub font_size_px: f64,
    pub line_height_px: f64,
}

impl DynamicMetricsInput {
    pub fn validate(&self) -> Result<(), RenderError> {
        validate_non_empty("cssFont", &self.css_font)?;
        validate_non_empty("fontFamily", &self.font_family)?;
        validate_positive_finite("fontSizePx", self.font_size_px)?;
        validate_positive_finite("lineHeightPx", self.line_height_px)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicTextMetricsError {
    message: String,
}

impl DynamicTextMetricsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DynamicTextMetricsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for DynamicTextMetricsError {}

pub struct CallbackTextMetricsProvider<F>
where
    F: FnMut(&str, &str) -> Result<f64, DynamicTextMetricsError>,
{
    input: DynamicMetricsInput,
    callback: RefCell<F>,
    cache: RefCell<HashMap<String, f64>>,
    first_error: RefCell<Option<DynamicTextMetricsError>>,
    in_callback: Cell<bool>,
    node_padding_x: f64,
    node_padding_y: f64,
}

impl<F> CallbackTextMetricsProvider<F>
where
    F: FnMut(&str, &str) -> Result<f64, DynamicTextMetricsError>,
{
    pub fn new(input: DynamicMetricsInput, callback: F) -> Self {
        Self::with_node_padding(
            input,
            DEFAULT_PROPORTIONAL_NODE_PADDING_X,
            DEFAULT_PROPORTIONAL_NODE_PADDING_Y,
            callback,
        )
    }

    pub(crate) fn with_node_padding(
        input: DynamicMetricsInput,
        node_padding_x: f64,
        node_padding_y: f64,
        callback: F,
    ) -> Self {
        Self {
            input,
            callback: RefCell::new(callback),
            cache: RefCell::new(HashMap::new()),
            first_error: RefCell::new(None),
            in_callback: Cell::new(false),
            node_padding_x,
            node_padding_y,
        }
    }

    pub fn finish(&self) -> Result<(), RenderError> {
        match self.first_error.borrow().clone() {
            Some(error) => Err(RenderError {
                message: error.to_string(),
            }),
            None => Ok(()),
        }
    }

    fn measure_text(&self, text: &str) -> f64 {
        if let Some(width) = self.cache.borrow().get(text).copied() {
            return width;
        }

        if self.in_callback.get() {
            self.record_error(DynamicTextMetricsError::new(format!(
                "dynamic text measurement callback re-entered while measuring {text:?} with font {:?}",
                self.input.css_font
            )));
            return 0.0;
        }

        self.in_callback.set(true);
        let result = {
            let mut callback = self.callback.borrow_mut();
            callback(text, &self.input.css_font)
        };
        self.in_callback.set(false);

        let validated = match result {
            Ok(width) => self.validate_width(text, width),
            Err(error) => Err(self.contextualize_callback_error(text, error)),
        };

        match validated {
            Ok(width) => {
                self.cache.borrow_mut().insert(text.to_string(), width);
                width
            }
            Err(error) => {
                self.record_error(error);
                0.0
            }
        }
    }

    fn contextualize_callback_error(
        &self,
        text: &str,
        error: DynamicTextMetricsError,
    ) -> DynamicTextMetricsError {
        DynamicTextMetricsError::new(format!(
            "dynamic text measurement callback failed for {text:?} with font {:?}: {error}",
            self.input.css_font
        ))
    }

    fn validate_width(&self, text: &str, width: f64) -> Result<f64, DynamicTextMetricsError> {
        if !width.is_finite() || width < 0.0 {
            return Err(DynamicTextMetricsError::new(format!(
                "dynamic text measurement for {text:?} with font {:?} must return a finite non-negative width",
                self.input.css_font
            )));
        }
        Ok(width)
    }

    fn record_error(&self, error: DynamicTextMetricsError) {
        let mut first_error = self.first_error.borrow_mut();
        if first_error.is_none() {
            *first_error = Some(error);
        }
    }
}

impl<F> TextMetricsProvider for CallbackTextMetricsProvider<F>
where
    F: FnMut(&str, &str) -> Result<f64, DynamicTextMetricsError>,
{
    fn measure_line_width(&self, text: &str) -> f64 {
        self.measure_text(text)
    }

    fn measure_scalar_width(&self, ch: char) -> f64 {
        let mut text = [0_u8; 4];
        self.measure_text(ch.encode_utf8(&mut text))
    }

    fn font_size(&self) -> f64 {
        self.input.font_size_px
    }

    fn line_height(&self) -> f64 {
        self.input.line_height_px
    }

    fn node_padding_x(&self) -> f64 {
        self.node_padding_x
    }

    fn node_padding_y(&self) -> f64 {
        self.node_padding_y
    }

    fn label_padding_x(&self) -> f64 {
        DEFAULT_LABEL_PADDING_X
    }

    fn label_padding_y(&self) -> f64 {
        DEFAULT_LABEL_PADDING_Y
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), RenderError> {
    if value.trim().is_empty() {
        return Err(RenderError {
            message: format!("dynamic text metrics field `{field}` must not be empty"),
        });
    }
    Ok(())
}

fn validate_positive_finite(field: &str, value: f64) -> Result<(), RenderError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(RenderError {
            message: format!(
                "dynamic text metrics field `{field}` must be a finite positive number"
            ),
        });
    }
    Ok(())
}

pub fn render_graph_family_svg_with_dynamic_text_metrics<F>(
    input: &str,
    config: &RenderConfig,
    dynamic_input: DynamicMetricsInput,
    callback: F,
) -> Result<String, RenderError>
where
    F: FnMut(&str, &str) -> Result<f64, DynamicTextMetricsError>,
{
    render_graph_family_with_dynamic_text_metrics(
        input,
        OutputFormat::Svg,
        config,
        dynamic_input,
        callback,
    )
}

pub fn render_graph_family_with_dynamic_text_metrics<F>(
    input: &str,
    format: OutputFormat,
    config: &RenderConfig,
    dynamic_input: DynamicMetricsInput,
    callback: F,
) -> Result<String, RenderError>
where
    F: FnMut(&str, &str) -> Result<f64, DynamicTextMetricsError>,
{
    if !matches!(format, OutputFormat::Svg) {
        return Err(RenderError {
            message: format!(
                "browser dynamic text metrics only supports SVG output (requested {format})"
            ),
        });
    }
    if config.layout_engine.is_some() {
        return Err(RenderError {
            message: "browser dynamic text metrics does not accept layoutEngine; it always uses flux-layered SVG"
                .to_string(),
        });
    }
    if config.font_metrics_profile.is_some() {
        return Err(RenderError {
            message: "browser dynamic text metrics does not accept fontMetricsProfile; dynamic provider measurement is selected by this API"
                .to_string(),
        });
    }

    dynamic_input.validate()?;
    super::validate_render_config(config)?;

    match detect_input_frontend(input) {
        Some(InputFrontend::Mmds) => {
            return Err(RenderError {
                message: "browser dynamic text metrics does not support MMDS input".to_string(),
            });
        }
        Some(InputFrontend::Mermaid) => {}
        None => {
            return Err(RenderError {
                message: "unknown diagram type".to_string(),
            });
        }
    }

    let registry = default_registry();
    let resolved = registry.resolve(input).ok_or_else(|| RenderError {
        message: "unknown diagram type".to_string(),
    })?;
    if !matches!(resolved.family(), DiagramFamily::Graph) {
        return Err(RenderError {
            message: format!(
                "browser dynamic text metrics only supports graph-family Mermaid diagrams (detected {})",
                resolved.diagram_id()
            ),
        });
    }
    if !resolved.supported_formats().contains(&OutputFormat::Svg) {
        return Err(RenderError {
            message: format!(
                "{} diagrams do not support SVG output",
                resolved.diagram_id()
            ),
        });
    }

    let instance = registry
        .create(resolved.diagram_id())
        .ok_or_else(|| RenderError {
            message: format!(
                "no implementation for diagram type: {}",
                resolved.diagram_id()
            ),
        })?;
    let parsed = instance.parse(input).map_err(|error| RenderError {
        message: format!("parse error: {error}"),
    })?;

    let mut effective_config = super::effective_render_config(input, OutputFormat::Svg, config);
    // Match the existing WASM SVG surface while keeping browser-measured layout
    // on this separate export instead of accepting a caller layoutEngine knob.
    apply_svg_surface_defaults(OutputFormat::Svg, &mut effective_config, true);
    let mut options = effective_config.svg_render_options();
    options.font_family = dynamic_input.font_family.clone();
    options.font_size = dynamic_input.font_size_px;

    let node_padding_x = effective_config
        .svg_node_padding_x
        .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_X);
    let node_padding_y = effective_config
        .svg_node_padding_y
        .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_Y);
    let provider = CallbackTextMetricsProvider::with_node_padding(
        dynamic_input,
        node_padding_x,
        node_padding_y,
        callback,
    );

    let payload =
        super::payload::prepare_payload_for_render(parsed.into_payload()?, &effective_config);
    let rendered = match payload {
        Diagram::Flowchart(mut graph) => {
            crate::runtime::graph_family::render_graph_family_svg_with_provider(
                "flowchart",
                &mut graph,
                &effective_config,
                &options,
                &provider,
            )
        }
        Diagram::Class(mut graph) => {
            crate::runtime::graph_family::render_graph_family_svg_with_provider(
                "class",
                &mut graph,
                &effective_config,
                &options,
                &provider,
            )
        }
        Diagram::State(mut graph) => {
            crate::runtime::graph_family::render_graph_family_svg_with_provider(
                "state",
                &mut graph,
                &effective_config,
                &options,
                &provider,
            )
        }
        Diagram::Sequence(_) => Err(RenderError {
            message: "browser dynamic text metrics only supports graph-family Mermaid diagrams"
                .to_string(),
        }),
    }?;

    provider.finish()?;
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::engines::graph::EngineAlgorithmId;
    use crate::format::OutputFormat;
    use crate::graph::measure::{
        DEFAULT_GRAPH_FONT_FAMILY, TextMetricsProfileConfig, TextMetricsProvider,
        resolve_text_metrics_profile,
    };
    use crate::runtime::config::RenderConfig;
    use crate::runtime::render_diagram;

    fn valid_input() -> DynamicMetricsInput {
        serde_json::from_str(
            r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24}"#,
        )
        .unwrap()
    }

    fn static_equivalent_input() -> DynamicMetricsInput {
        let metrics = resolve_text_metrics_profile(TextMetricsProfileConfig::default())
            .expect("default recorded text metrics should resolve")
            .metrics;
        DynamicMetricsInput {
            css_font: format!("{}px {}", metrics.font_size(), DEFAULT_GRAPH_FONT_FAMILY),
            font_family: DEFAULT_GRAPH_FONT_FAMILY.to_string(),
            font_size_px: metrics.font_size(),
            line_height_px: metrics.line_height(),
        }
    }

    #[test]
    fn dynamic_metrics_input_rejects_unknown_fields() {
        let err = serde_json::from_str::<DynamicMetricsInput>(
            r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24,"extra":true}"#,
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("unknown field"), "{err}");
        assert!(err.contains("extra"), "{err}");
    }

    #[test]
    fn dynamic_metrics_input_validates_required_style_fields() {
        valid_input().validate().expect("valid input should pass");

        for (field, input) in [
            (
                "cssFont",
                DynamicMetricsInput {
                    css_font: " ".to_string(),
                    ..valid_input()
                },
            ),
            (
                "fontFamily",
                DynamicMetricsInput {
                    font_family: " ".to_string(),
                    ..valid_input()
                },
            ),
            (
                "fontSizePx",
                DynamicMetricsInput {
                    font_size_px: 0.0,
                    ..valid_input()
                },
            ),
            (
                "lineHeightPx",
                DynamicMetricsInput {
                    line_height_px: f64::INFINITY,
                    ..valid_input()
                },
            ),
        ] {
            let err = input.validate().expect_err("invalid field should fail");
            assert!(err.message.contains(field), "{err}");
        }
    }

    #[test]
    fn callback_provider_caches_repeated_measurements() {
        let calls = Rc::new(Cell::new(0));
        let observed_calls = Rc::clone(&calls);
        let provider = CallbackTextMetricsProvider::new(valid_input(), move |text, _css_font| {
            calls.set(calls.get() + 1);
            Ok(text.len() as f64)
        });

        assert_eq!(provider.measure_line_width("Alpha"), 5.0);
        assert_eq!(provider.measure_line_width("Alpha"), 5.0);
        provider.finish().expect("cached measurements should pass");
        assert_eq!(observed_calls.get(), 1);
    }

    #[test]
    fn callback_provider_records_invalid_width_errors_with_text_and_font() {
        for width in [f64::NAN, f64::INFINITY, -1.0] {
            let provider =
                CallbackTextMetricsProvider::new(valid_input(), move |_text, _font| Ok(width));

            assert_eq!(provider.measure_line_width("Alpha"), 0.0);
            let err = provider
                .finish()
                .expect_err("invalid width should be recorded");
            assert!(err.message.contains("Alpha"), "{err}");
            assert!(err.message.contains("16px Inter"), "{err}");
            assert!(err.message.contains("finite non-negative width"), "{err}");
        }
    }

    #[test]
    fn callback_provider_records_callback_errors_with_text_and_font() {
        let provider = CallbackTextMetricsProvider::new(valid_input(), |_text, _font| {
            Err(DynamicTextMetricsError::new("canvas failed"))
        });

        assert_eq!(provider.measure_line_width("Beta"), 0.0);
        let err = provider
            .finish()
            .expect_err("callback error should be recorded");
        assert!(err.message.contains("Beta"), "{err}");
        assert!(err.message.contains("16px Inter"), "{err}");
        assert!(err.message.contains("canvas failed"), "{err}");
    }

    #[test]
    fn callback_provider_reentry_errors_cleanly() {
        let provider =
            CallbackTextMetricsProvider::new(valid_input(), |text, _font| Ok(text.len() as f64));

        provider.in_callback.set(true);
        assert_eq!(provider.measure_line_width("Gamma"), 0.0);
        provider.in_callback.set(false);

        let err = provider.finish().expect_err("re-entry should be recorded");
        assert!(err.message.contains("re-entered"), "{err}");
        assert!(err.message.contains("Gamma"), "{err}");
        assert!(err.message.contains("16px Inter"), "{err}");
    }

    #[test]
    fn callback_provider_caches_line_and_scalar_measurements() {
        let calls = Rc::new(Cell::new(0));
        let observed_calls = Rc::clone(&calls);
        let provider = CallbackTextMetricsProvider::new(valid_input(), move |text, _font| {
            calls.set(calls.get() + 1);
            Ok(text.len() as f64)
        });

        for line in ["Alpha", "Beta", "Alpha", "Beta"] {
            provider.measure_line_width(line);
        }
        for ch in "Alpha Beta".chars().chain("Alpha Beta".chars()) {
            provider.measure_scalar_width(ch);
        }

        provider.finish().expect("cached measurements should pass");
        let unique_lines = 2;
        let unique_scalars = "Alpha Beta"
            .chars()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert_eq!(observed_calls.get(), unique_lines + unique_scalars);
    }

    #[test]
    fn dynamic_svg_bridge_changes_label_background_width() {
        let input = "graph TD\nA -->|mmmm| B";
        let static_svg =
            render_diagram(input, OutputFormat::Svg, &RenderConfig::default()).unwrap();
        let dynamic_svg = render_graph_family_svg_with_dynamic_text_metrics(
            input,
            &RenderConfig::default(),
            valid_input(),
            |text, _css_font| Ok(if text.contains('m') { 100.0 } else { 8.0 }),
        )
        .unwrap();

        assert_ne!(dynamic_svg, static_svg);
        assert!(dynamic_svg.contains("<svg"), "{dynamic_svg}");
        assert!(dynamic_svg.contains("width=\"108.00\""), "{dynamic_svg}");
        assert!(!dynamic_svg.contains("metricsProfile"), "{dynamic_svg}");
    }

    #[test]
    fn dynamic_svg_bridge_rejects_unsupported_output_formats() {
        for format in [
            OutputFormat::Text,
            OutputFormat::Ascii,
            OutputFormat::Mmds,
            OutputFormat::Mermaid,
        ] {
            let err = render_graph_family_with_dynamic_text_metrics(
                "graph TD\nA-->B",
                format,
                &RenderConfig::default(),
                valid_input(),
                |_text, _css_font| Ok(8.0),
            )
            .expect_err("unsupported output format should fail");
            assert!(err.message.contains("only supports SVG output"), "{err}");
        }
    }

    #[test]
    fn dynamic_svg_bridge_rejects_mmds_input() {
        let mmds = render_diagram(
            "graph TD\nA-->B",
            OutputFormat::Mmds,
            &RenderConfig::default(),
        )
        .unwrap();

        let err = render_graph_family_svg_with_dynamic_text_metrics(
            &mmds,
            &RenderConfig::default(),
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("MMDS input should fail");

        assert!(err.message.contains("does not support MMDS input"), "{err}");
    }

    #[test]
    fn dynamic_svg_bridge_rejects_sequence_input() {
        let err = render_graph_family_svg_with_dynamic_text_metrics(
            "sequenceDiagram\nAlice->>Bob: Hi",
            &RenderConfig::default(),
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("sequence input should fail");

        assert!(
            err.message
                .contains("only supports graph-family Mermaid diagrams"),
            "{err}"
        );
    }

    #[test]
    fn dynamic_svg_bridge_rejects_layout_engine_override() {
        let err = render_graph_family_svg_with_dynamic_text_metrics(
            "graph TD\nA-->B",
            &RenderConfig {
                layout_engine: Some(EngineAlgorithmId::MERMAID_LAYERED),
                ..RenderConfig::default()
            },
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("layout engine override should fail");

        assert!(
            err.message.contains("does not accept layoutEngine"),
            "{err}"
        );
    }

    #[test]
    fn dynamic_svg_bridge_rejects_static_font_metrics_profile() {
        let err = render_graph_family_svg_with_dynamic_text_metrics(
            "graph TD\nA-->B",
            &RenderConfig {
                font_metrics_profile: Some("mmdflux-sans-v1".to_string()),
                ..RenderConfig::default()
            },
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("static font metrics profile should fail");

        assert!(
            err.message.contains("does not accept fontMetricsProfile"),
            "{err}"
        );
    }

    #[test]
    fn dynamic_svg_bridge_matches_static_svg_with_recorded_callback() {
        let input = "graph TD\nA[mmmm]\nB[iiii]";
        let static_svg =
            render_diagram(input, OutputFormat::Svg, &RenderConfig::default()).unwrap();
        let recorded = resolve_text_metrics_profile(TextMetricsProfileConfig::default())
            .expect("default recorded text metrics should resolve")
            .metrics;
        let dynamic_svg = render_graph_family_svg_with_dynamic_text_metrics(
            input,
            &RenderConfig::default(),
            static_equivalent_input(),
            move |text, _css_font| Ok(recorded.measure_line_width(text)),
        )
        .unwrap();

        assert_eq!(dynamic_svg, static_svg);
    }
}
