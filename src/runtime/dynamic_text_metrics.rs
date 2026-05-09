//! Experimental dynamic text metrics contract for callback-backed adapters.
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
    DEFAULT_PROPORTIONAL_NODE_PADDING_Y, TextMetricsLayoutDescriptor, TextMetricsProfileDescriptor,
    TextMetricsProvider, TextMetricsStyleDescriptor,
};
use crate::payload::Diagram;
use crate::registry::DiagramFamily;
use crate::render::graph::SvgRenderOptions;
use crate::runtime::config::RenderConfig;
use crate::runtime::config_input::apply_svg_surface_defaults;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DynamicMetricsInput {
    pub css_font: String,
    pub font_family: String,
    pub font_size_px: f64,
    pub line_height_px: f64,
    pub profile_id: Option<String>,
    pub profile_version: Option<u32>,
    pub font_style: Option<String>,
    pub font_weight: Option<String>,
}

impl DynamicMetricsInput {
    pub fn validate(&self) -> Result<(), RenderError> {
        validate_non_empty("cssFont", &self.css_font)?;
        validate_non_empty("fontFamily", &self.font_family)?;
        validate_positive_finite("fontSizePx", self.font_size_px)?;
        validate_positive_finite("lineHeightPx", self.line_height_px)?;
        if let Some(profile_id) = &self.profile_id {
            validate_non_empty("profileId", profile_id)?;
        }
        if matches!(self.profile_version, Some(0)) {
            return Err(RenderError {
                message: "dynamic text metrics field `profileVersion` must be a positive integer"
                    .to_string(),
            });
        }
        if let Some(font_style) = &self.font_style {
            validate_non_empty("fontStyle", font_style)?;
        }
        if let Some(font_weight) = &self.font_weight {
            validate_non_empty("fontWeight", font_weight)?;
        }
        Ok(())
    }

    pub(crate) fn profile_version_or_default(&self) -> u32 {
        self.profile_version.unwrap_or(1)
    }

    pub(crate) fn font_style_or_default(&self) -> &str {
        self.font_style.as_deref().unwrap_or("normal")
    }

    pub(crate) fn font_weight_or_default(&self) -> &str {
        self.font_weight.as_deref().unwrap_or("400")
    }

    pub(crate) fn require_profile_id(&self, operation: &str) -> Result<&str, RenderError> {
        match self.profile_id.as_deref() {
            Some(profile_id) if !profile_id.trim().is_empty() => Ok(profile_id),
            _ => Err(RenderError {
                message: format!(
                    "dynamic text metrics field `profileId` is required for {operation}"
                ),
            }),
        }
    }

    pub(crate) fn text_metrics_descriptor_for_layout(
        &self,
        node_padding_x: f64,
        node_padding_y: f64,
        edge_label_max_width: Option<f64>,
    ) -> Result<TextMetricsProfileDescriptor, RenderError> {
        self.validate()?;
        let profile_id = self.require_profile_id("dynamic MMDS descriptor")?;
        Ok(TextMetricsProfileDescriptor {
            profile_id: profile_id.to_string(),
            source: "dynamic".to_string(),
            version: self.profile_version_or_default(),
            default_text_style: TextMetricsStyleDescriptor {
                font_family: self.font_family.clone(),
                font_size: self.font_size_px,
                font_style: self.font_style_or_default().to_string(),
                font_weight: self.font_weight_or_default().to_string(),
                line_height: self.line_height_px,
            },
            layout_text: TextMetricsLayoutDescriptor {
                node_padding_x,
                node_padding_y,
                label_padding_x: DEFAULT_LABEL_PADDING_X,
                label_padding_y: DEFAULT_LABEL_PADDING_Y,
                edge_label_max_width,
            },
        })
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
    if !matches!(format, OutputFormat::Svg | OutputFormat::Mmds) {
        return Err(RenderError {
            message: format!(
                "dynamic text metrics supports SVG and provider-bound MMDS output (requested {format})"
            ),
        });
    }
    if config.layout_engine.is_some() {
        return Err(RenderError {
            message:
                "dynamic text metrics does not accept layoutEngine; it always uses flux-layered SVG"
                    .to_string(),
        });
    }
    if config.font_metrics_profile.is_some() {
        return Err(RenderError {
            message: "dynamic text metrics does not accept fontMetricsProfile; provider measurement is selected by this API"
                .to_string(),
        });
    }
    if config.graph_text_style.is_some() {
        return Err(RenderError {
            message: "dynamic text metrics uses DynamicMetricsInput for font identity; do not pass RenderConfig.graph_text_style"
                .to_string(),
        });
    }

    dynamic_input.validate()?;
    super::validate_render_config(config)?;

    match detect_input_frontend(input) {
        Some(InputFrontend::Mmds) => {
            let mut effective_config = super::effective_render_config(input, format, config);
            if matches!(format, OutputFormat::Svg) {
                apply_svg_surface_defaults(OutputFormat::Svg, &mut effective_config, true);
            }
            let font_family = dynamic_input.font_family.clone();
            let font_size = dynamic_input.font_size_px;
            let node_padding_x = effective_config
                .svg_node_padding_x
                .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_X);
            let node_padding_y = effective_config
                .svg_node_padding_y
                .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_Y);
            let text_metrics_descriptor = dynamic_input.text_metrics_descriptor_for_layout(
                node_padding_x,
                node_padding_y,
                effective_config.layout.edge_label_max_width,
            )?;
            let provider = CallbackTextMetricsProvider::with_node_padding(
                dynamic_input,
                node_padding_x,
                node_padding_y,
                callback,
            );
            let mut options = effective_config.svg_render_options();
            options.font_family = font_family;
            options.font_size = font_size;
            let svg_theme = super::resolve_configured_svg_theme(&effective_config)?;
            let rendered = crate::runtime::mmds::render_input_with_dynamic_text_metrics(
                input,
                format,
                &options,
                svg_theme.as_ref(),
                &text_metrics_descriptor,
                &provider,
            )?;
            provider.finish()?;
            return Ok(rendered);
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
                "dynamic text metrics only supports graph-family Mermaid diagrams (detected {})",
                resolved.diagram_id()
            ),
        });
    }
    if !resolved.supported_formats().contains(&format) {
        return Err(RenderError {
            message: format!(
                "{} diagrams do not support {format} output",
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

    let mut effective_config = super::effective_render_config(input, format, config);
    if matches!(format, OutputFormat::Svg) {
        // Match the existing Wasm SVG surface while keeping dynamic-measured
        // layout on this separate export instead of accepting a caller
        // layoutEngine knob.
        apply_svg_surface_defaults(OutputFormat::Svg, &mut effective_config, true);
    }
    let font_family = dynamic_input.font_family.clone();
    let font_size = dynamic_input.font_size_px;

    let node_padding_x = effective_config
        .svg_node_padding_x
        .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_X);
    let node_padding_y = effective_config
        .svg_node_padding_y
        .unwrap_or(DEFAULT_PROPORTIONAL_NODE_PADDING_Y);
    let text_metrics_descriptor = if matches!(format, OutputFormat::Mmds) {
        Some(dynamic_input.text_metrics_descriptor_for_layout(
            node_padding_x,
            node_padding_y,
            effective_config.layout.edge_label_max_width,
        )?)
    } else {
        None
    };
    let provider = CallbackTextMetricsProvider::with_node_padding(
        dynamic_input,
        node_padding_x,
        node_padding_y,
        callback,
    );
    let mut options = effective_config.svg_render_options();
    options.font_family = font_family;
    options.font_size = font_size;
    let render_context = DynamicGraphFamilyRenderContext {
        format,
        config: &effective_config,
        svg_options: &options,
        text_metrics_descriptor: text_metrics_descriptor.as_ref(),
        provider: &provider,
    };

    let payload =
        super::payload::prepare_payload_for_render(parsed.into_payload()?, &effective_config);
    let rendered = match payload {
        Diagram::Flowchart(mut graph) => render_graph_family_payload_with_dynamic_metrics(
            "flowchart",
            &mut graph,
            &render_context,
        ),
        Diagram::Class(mut graph) => {
            render_graph_family_payload_with_dynamic_metrics("class", &mut graph, &render_context)
        }
        Diagram::State(mut graph) => {
            render_graph_family_payload_with_dynamic_metrics("state", &mut graph, &render_context)
        }
        Diagram::Sequence(_) => Err(RenderError {
            message: "dynamic text metrics only supports graph-family Mermaid diagrams".to_string(),
        }),
    }?;

    provider.finish()?;
    Ok(rendered)
}

struct DynamicGraphFamilyRenderContext<'a> {
    format: OutputFormat,
    config: &'a RenderConfig,
    svg_options: &'a SvgRenderOptions,
    text_metrics_descriptor: Option<&'a TextMetricsProfileDescriptor>,
    provider: &'a dyn TextMetricsProvider,
}

fn render_graph_family_payload_with_dynamic_metrics(
    diagram_id: &str,
    graph: &mut crate::graph::Graph,
    context: &DynamicGraphFamilyRenderContext<'_>,
) -> Result<String, RenderError> {
    match context.format {
        OutputFormat::Svg => crate::runtime::graph_family::render_graph_family_svg_with_provider(
            diagram_id,
            graph,
            context.config,
            context.svg_options,
            context.provider,
        ),
        OutputFormat::Mmds => {
            let descriptor = context.text_metrics_descriptor.ok_or_else(|| RenderError {
                message:
                    "dynamic text metrics field `profileId` is required for dynamic MMDS descriptor"
                        .to_string(),
            })?;
            crate::runtime::graph_family::render_graph_family_mmds_with_provider(
                diagram_id,
                graph,
                context.config,
                descriptor,
                context.provider,
            )
        }
        _ => unreachable!("format validation restricts dynamic text metrics output"),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::engines::graph::EngineAlgorithmId;
    use crate::format::OutputFormat;
    use crate::graph::GeometryLevel;
    use crate::graph::measure::{
        DEFAULT_GRAPH_FONT_FAMILY, TextMetricsProfileConfig, TextMetricsProvider,
        resolve_text_metrics_profile,
    };
    use crate::runtime::config::{GraphTextStyleConfig, RenderConfig};
    use crate::runtime::render_diagram;

    fn valid_input() -> DynamicMetricsInput {
        serde_json::from_str(
            r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24}"#,
        )
        .unwrap()
    }

    fn profiled_input(profile_id: &str) -> DynamicMetricsInput {
        serde_json::from_str(&format!(
            r#"{{
              "cssFont":"16px Inter",
              "fontFamily":"Inter",
              "fontSizePx":16,
              "lineHeightPx":24,
              "profileId":"{profile_id}"
            }}"#
        ))
        .unwrap()
    }

    fn deterministic_width(text: &str, _css_font: &str) -> Result<f64, DynamicTextMetricsError> {
        Ok(text.len() as f64 * 8.0)
    }

    fn routed_config() -> RenderConfig {
        RenderConfig {
            geometry_level: GeometryLevel::Routed,
            ..RenderConfig::default()
        }
    }

    fn dynamic_mmds_fixture() -> String {
        render_graph_family_with_dynamic_text_metrics(
            "graph TD\nA[Alpha] -->|a labeled edge| B[Beta]",
            OutputFormat::Mmds,
            &routed_config(),
            profiled_input("browser-test-v1"),
            deterministic_width,
        )
        .expect("dynamic MMDS fixture should render")
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
            profile_id: None,
            profile_version: None,
            font_style: None,
            font_weight: None,
        }
    }

    #[test]
    fn dynamic_metrics_input_accepts_optional_provider_identity_for_svg() {
        let input: DynamicMetricsInput = serde_json::from_str(
            r#"{
              "cssFont":"16px Inter",
              "fontFamily":"Inter",
              "fontSizePx":16,
              "lineHeightPx":24,
              "profileId":"browser-inter-v1",
              "profileVersion":2,
              "fontStyle":"italic",
              "fontWeight":"700"
            }"#,
        )
        .unwrap();

        input.validate().unwrap();
        assert_eq!(
            input.require_profile_id("dynamic MMDS").unwrap(),
            "browser-inter-v1"
        );
        assert_eq!(input.profile_version_or_default(), 2);
        assert_eq!(input.font_style_or_default(), "italic");
        assert_eq!(input.font_weight_or_default(), "700");
    }

    #[test]
    fn dynamic_metrics_input_keeps_existing_svg_metrics_json_valid() {
        let input = valid_input();

        input.validate().unwrap();
        assert_eq!(input.profile_version_or_default(), 1);
        assert_eq!(input.font_style_or_default(), "normal");
        assert_eq!(input.font_weight_or_default(), "400");
        let err = input
            .require_profile_id("dynamic MMDS")
            .expect_err("profile id should only be required for descriptor operations");
        assert!(err.message.contains("profileId"), "{err}");
        assert!(err.message.contains("dynamic MMDS"), "{err}");
        assert!(!err.message.contains("browser"), "{err}");
    }

    #[test]
    fn dynamic_metrics_input_rejects_zero_profile_version() {
        let input: DynamicMetricsInput = serde_json::from_str(
            r#"{
              "cssFont":"16px Inter",
              "fontFamily":"Inter",
              "fontSizePx":16,
              "lineHeightPx":24,
              "profileVersion":0
            }"#,
        )
        .unwrap();

        let err = input
            .validate()
            .expect_err("zero profileVersion should fail");
        assert!(err.message.contains("profileVersion"), "{err}");
        assert!(err.message.contains("positive"), "{err}");
    }

    #[test]
    fn dynamic_metrics_input_rejects_empty_optional_identity_fields() {
        for (field, json) in [
            (
                "profileId",
                r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24,"profileId":" "}"#,
            ),
            (
                "fontStyle",
                r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24,"fontStyle":" "}"#,
            ),
            (
                "fontWeight",
                r#"{"cssFont":"16px Inter","fontFamily":"Inter","fontSizePx":16,"lineHeightPx":24,"fontWeight":" "}"#,
            ),
        ] {
            let input: DynamicMetricsInput = serde_json::from_str(json).unwrap();
            let err = input
                .validate()
                .expect_err("empty optional field should fail");
            assert!(err.message.contains(field), "{err}");
        }
    }

    #[test]
    fn dynamic_metrics_input_builds_provider_bound_descriptor() {
        let input: DynamicMetricsInput = serde_json::from_str(
            r#"{
              "cssFont":"16px Inter",
              "fontFamily":"Inter",
              "fontSizePx":16,
              "lineHeightPx":24,
              "profileId":"browser-inter-v1"
            }"#,
        )
        .unwrap();

        let descriptor = input
            .text_metrics_descriptor_for_layout(15.0, 16.0, Some(200.0))
            .unwrap();

        assert_eq!(descriptor.profile_id, "browser-inter-v1");
        assert_eq!(descriptor.source, "dynamic");
        assert_eq!(descriptor.version, 1);
        assert_eq!(descriptor.default_text_style.font_family, "Inter");
        assert_eq!(descriptor.default_text_style.font_size, 16.0);
        assert_eq!(descriptor.default_text_style.font_style, "normal");
        assert_eq!(descriptor.default_text_style.font_weight, "400");
        assert_eq!(descriptor.default_text_style.line_height, 24.0);
        assert_eq!(descriptor.layout_text.node_padding_x, 15.0);
        assert_eq!(descriptor.layout_text.node_padding_y, 16.0);
        assert_eq!(
            descriptor.layout_text.label_padding_x,
            DEFAULT_LABEL_PADDING_X
        );
        assert_eq!(
            descriptor.layout_text.label_padding_y,
            DEFAULT_LABEL_PADDING_Y
        );
        assert_eq!(descriptor.layout_text.edge_label_max_width, Some(200.0));
    }

    #[test]
    fn dynamic_metrics_descriptor_mirrors_adapter_owned_style_and_version() {
        let input: DynamicMetricsInput = serde_json::from_str(
            r#"{
              "cssFont":"italic 700 16px Inter",
              "fontFamily":"Inter",
              "fontSizePx":16,
              "lineHeightPx":24,
              "profileId":"browser-inter-v2",
              "profileVersion":2,
              "fontStyle":"italic",
              "fontWeight":"700"
            }"#,
        )
        .unwrap();

        let descriptor = input
            .text_metrics_descriptor_for_layout(12.0, 13.0, None)
            .unwrap();

        assert_eq!(descriptor.profile_id, "browser-inter-v2");
        assert_eq!(descriptor.version, 2);
        assert_eq!(descriptor.default_text_style.font_style, "italic");
        assert_eq!(descriptor.default_text_style.font_weight, "700");
        assert_eq!(descriptor.layout_text.edge_label_max_width, None);
    }

    #[test]
    fn dynamic_metrics_descriptor_requires_profile_id() {
        let err = valid_input()
            .text_metrics_descriptor_for_layout(15.0, 15.0, Some(200.0))
            .expect_err("descriptor construction should require profileId");

        assert!(err.message.contains("profileId"), "{err}");
        assert!(err.message.contains("dynamic MMDS descriptor"), "{err}");
        assert!(!err.message.contains("browser"), "{err}");
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
            OutputFormat::Mermaid,
        ] {
            let err = render_graph_family_with_dynamic_text_metrics(
                "graph TD\nA-->B",
                format,
                &RenderConfig::default(),
                profiled_input("browser-test-v1"),
                |_text, _css_font| Ok(8.0),
            )
            .expect_err("unsupported output format should fail");
            assert!(
                err.message
                    .contains("supports SVG and provider-bound MMDS output"),
                "{err}"
            );
        }
    }

    #[test]
    fn dynamic_mmds_output_emits_provider_bound_text_metrics_extension() {
        let output = render_graph_family_with_dynamic_text_metrics(
            "graph TD\nA[Alpha] --> B[Beta]",
            OutputFormat::Mmds,
            &RenderConfig::default(),
            profiled_input("browser-test-v1"),
            |text, _css_font| Ok(text.len() as f64 * 8.0),
        )
        .expect("dynamic MMDS output should render");

        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        let extension = &value["extensions"]["org.mmdflux.text-metrics.v1"];
        assert_eq!(extension["metricsProfile"]["source"], "dynamic");
        assert_eq!(extension["metricsProfile"]["id"], "browser-test-v1");
        assert_eq!(extension["metricsProfile"]["version"], 1);
        assert_eq!(extension["defaultTextStyle"]["font-family"], "Inter");
        assert_eq!(extension["defaultTextStyle"]["font-size"], 16.0);
        assert_eq!(extension["defaultTextStyle"]["line-height"], 24.0);
        assert!(
            value["extensions"]
                .get("org.mmdflux.text-measurements.v1")
                .is_none()
        );
    }

    #[test]
    fn dynamic_mmds_output_requires_profile_id() {
        let err = render_graph_family_with_dynamic_text_metrics(
            "graph TD\nA-->B",
            OutputFormat::Mmds,
            &RenderConfig::default(),
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("dynamic MMDS output should require provider identity");

        assert!(err.message.contains("profileId"), "{err}");
        assert!(err.message.contains("dynamic MMDS descriptor"), "{err}");
    }

    #[test]
    fn dynamic_mmds_output_rejects_sequence_input() {
        let err = render_graph_family_with_dynamic_text_metrics(
            "sequenceDiagram\nAlice->>Bob: Hi",
            OutputFormat::Mmds,
            &RenderConfig::default(),
            profiled_input("browser-test-v1"),
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
    fn dynamic_mmds_output_rejects_layout_engine_override() {
        let err = render_graph_family_with_dynamic_text_metrics(
            "graph TD\nA-->B",
            OutputFormat::Mmds,
            &RenderConfig {
                layout_engine: Some(EngineAlgorithmId::MERMAID_LAYERED),
                ..RenderConfig::default()
            },
            profiled_input("browser-test-v1"),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("layout engine override should fail");

        assert!(
            err.message.contains("does not accept layoutEngine"),
            "{err}"
        );
    }

    #[test]
    fn dynamic_mmds_replay_with_matching_provider_matches_direct_dynamic_svg() {
        let input = "graph TD\nA[Alpha] -->|a labeled edge| B[Beta]";
        let config = routed_config();
        let metrics = profiled_input("browser-test-v1");
        let direct_svg = render_graph_family_with_dynamic_text_metrics(
            input,
            OutputFormat::Svg,
            &config,
            metrics.clone(),
            deterministic_width,
        )
        .expect("direct dynamic SVG should render");
        let mmds = render_graph_family_with_dynamic_text_metrics(
            input,
            OutputFormat::Mmds,
            &config,
            metrics.clone(),
            deterministic_width,
        )
        .expect("dynamic MMDS should render");

        let replay_svg = render_graph_family_with_dynamic_text_metrics(
            &mmds,
            OutputFormat::Svg,
            &config,
            metrics,
            deterministic_width,
        )
        .expect("dynamic MMDS replay should render with matching provider");

        assert_eq!(replay_svg, direct_svg);
    }

    #[test]
    fn dynamic_mmds_replay_rejects_provider_descriptor_mismatches() {
        let mmds = dynamic_mmds_fixture();

        let mut version = profiled_input("browser-test-v1");
        version.profile_version = Some(2);
        let mut font_family = profiled_input("browser-test-v1");
        font_family.font_family = "Arial".to_string();
        let mut font_size = profiled_input("browser-test-v1");
        font_size.font_size_px = 18.0;
        let mut font_style = profiled_input("browser-test-v1");
        font_style.font_style = Some("italic".to_string());
        let mut font_weight = profiled_input("browser-test-v1");
        font_weight.font_weight = Some("700".to_string());
        let mut line_height = profiled_input("browser-test-v1");
        line_height.line_height_px = 30.0;

        for (name, metrics, expected_field) in [
            (
                "profile id",
                profiled_input("browser-other-v1"),
                "metricsProfile.id",
            ),
            ("version", version, "metricsProfile.version"),
            ("font family", font_family, "defaultTextStyle.font-family"),
            ("font size", font_size, "defaultTextStyle.font-size"),
            ("font style", font_style, "defaultTextStyle.font-style"),
            ("font weight", font_weight, "defaultTextStyle.font-weight"),
            ("line height", line_height, "defaultTextStyle.line-height"),
        ] {
            let err = render_graph_family_with_dynamic_text_metrics(
                &mmds,
                OutputFormat::Svg,
                &routed_config(),
                metrics,
                deterministic_width,
            )
            .unwrap_err();
            assert!(err.message.contains(expected_field), "{name}: {err}");
        }

        let mut node_padding_config = routed_config();
        node_padding_config.svg_node_padding_x = Some(20.0);
        let err = render_graph_family_with_dynamic_text_metrics(
            &mmds,
            OutputFormat::Svg,
            &node_padding_config,
            profiled_input("browser-test-v1"),
            deterministic_width,
        )
        .expect_err("node padding mismatch should fail");
        assert!(err.message.contains("layoutText.node-padding-x"), "{err}");

        let mut edge_width_config = routed_config();
        edge_width_config.layout.edge_label_max_width = Some(120.0);
        let err = render_graph_family_with_dynamic_text_metrics(
            &mmds,
            OutputFormat::Svg,
            &edge_width_config,
            profiled_input("browser-test-v1"),
            deterministic_width,
        )
        .expect_err("edge label max width mismatch should fail");
        assert!(
            err.message.contains("layoutText.edge-label-max-width"),
            "{err}"
        );
    }

    #[test]
    fn dynamic_mmds_replay_rejects_persisted_label_padding_mismatch() {
        let mut value: serde_json::Value = serde_json::from_str(&dynamic_mmds_fixture()).unwrap();
        value["extensions"]["org.mmdflux.text-metrics.v1"]["layoutText"]["label-padding-x"] =
            serde_json::json!(8.0);
        let mmds = serde_json::to_string(&value).unwrap();

        let err = render_graph_family_with_dynamic_text_metrics(
            &mmds,
            OutputFormat::Svg,
            &routed_config(),
            profiled_input("browser-test-v1"),
            deterministic_width,
        )
        .expect_err("label padding mismatch should fail");

        assert!(err.message.contains("layoutText.label-padding-x"), "{err}");
    }

    #[test]
    fn dynamic_mmds_replay_rejects_static_mmds_input() {
        let mmds = render_diagram(
            "graph TD\nA-->B",
            OutputFormat::Mmds,
            &RenderConfig::default(),
        )
        .unwrap();

        let err = render_graph_family_svg_with_dynamic_text_metrics(
            &mmds,
            &RenderConfig::default(),
            profiled_input("browser-test-v1"),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("static MMDS input should fail on the dynamic replay path");

        assert!(err.message.contains("metricsProfile.source"), "{err}");
        assert!(err.message.contains("not dynamic"), "{err}");
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
    fn dynamic_svg_rejects_config_graph_text_style() {
        let config = RenderConfig {
            graph_text_style: Some(GraphTextStyleConfig::new("Inter", 16.0)),
            ..RenderConfig::default()
        };

        let err = render_graph_family_svg_with_dynamic_text_metrics(
            "graph TD\nA-->B",
            &config,
            valid_input(),
            |_text, _css_font| Ok(8.0),
        )
        .expect_err("config graph text style should fail");

        assert!(err.message.contains("DynamicMetricsInput"), "{err}");
        assert!(
            err.message.contains("RenderConfig.graph_text_style"),
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
