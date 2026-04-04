//! Shared SVG text emission helpers for graph rendering.

use crate::graph::geometry::FPoint;
use crate::graph::measure::ProportionalTextMetrics;
use crate::render::svg::{SvgWriter, escape_text, fmt_f64};

pub(super) struct TextRenderStyle<'a> {
    pub(super) color: &'a str,
    pub(super) extra_attrs: &'a str,
}

pub(super) fn render_text_centered(
    writer: &mut SvgWriter,
    center: FPoint,
    text: &str,
    metrics: &ProportionalTextMetrics,
    scale: f64,
    style: TextRenderStyle<'_>,
) {
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() == 1 {
        let line = format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{color}\"{extra_attrs}>{text}</text>",
            x = fmt_f64(center.x),
            y = fmt_f64(center.y),
            color = style.color,
            extra_attrs = style.extra_attrs,
            text = escape_text(text)
        );
        writer.push_line(&line);
        return;
    }

    let line_height = metrics.line_height * scale;
    let total_height = line_height * (lines.len().saturating_sub(1) as f64);
    let start_y = center.y - total_height / 2.0;

    for (idx, line_text) in lines.iter().enumerate() {
        let line_y = start_y + line_height * idx as f64;
        let line = format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{color}\"{extra_attrs}>{text}</text>",
            x = fmt_f64(center.x),
            y = fmt_f64(line_y),
            color = style.color,
            extra_attrs = style.extra_attrs,
            text = escape_text(line_text)
        );
        writer.push_line(&line);
    }
}
