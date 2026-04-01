//! SVG rendering for sequence diagrams.
//!
//! Consumes an `SvgSequenceLayout` and emits SVG markup using the shared
//! `SvgWriter` utilities.

use super::svg_layout::{
    SvgActivation, SvgBlock, SvgBlockDivider, SvgDestroyMarker, SvgLifeline, SvgMessage, SvgNote,
    SvgParticipant, SvgParticipantBox, SvgRow, SvgSelfMessage, SvgSequenceLayout, SvgTitle,
};
use crate::render::svg::{SvgWriter, escape_text, fmt_f64};
use crate::timeline::sequence::model::{ArrowHead, LineStyle, ParticipantKind};

const STROKE_COLOR: &str = "#333";
const FILL_COLOR: &str = "white";
const TEXT_COLOR: &str = "#333";
const NOTE_FILL: &str = "#ffffcc";
const ACTIVATION_FILL: &str = "#ddd";
const ACTOR_STROKE: &str = "#333";
const BLOCK_STROKE: &str = "#666";
const BLOCK_LABEL_BG: &str = "white";
const BLOCK_STROKE_DASH: &str = "3,3";
const BLOCK_TAB_HEIGHT: f64 = 20.0;
const BLOCK_TAB_MIN_WIDTH: f64 = 42.0;
const BLOCK_TAB_PADDING_X: f64 = 8.0;
const BLOCK_TAB_CUT: f64 = 7.0;
const BLOCK_TAB_CHAR_WIDTH: f64 = 8.0;
const BLOCK_HEADER_TEXT_Y: f64 = 15.0;
const BLOCK_SECTION_TEXT_Y: f64 = 15.0;
const PARTICIPANT_BOX_STROKE: &str = "#8a8a8a";
const PARTICIPANT_BOX_LABEL_BG: &str = "white";

/// Render an SVG sequence layout to an SVG string.
pub fn render(layout: &SvgSequenceLayout) -> String {
    let mut writer = SvgWriter::new();

    writer.start_svg(
        layout.width,
        layout.height,
        &layout.font_family,
        layout.font_size,
    );

    render_defs(&mut writer);

    if let Some(title) = &layout.title {
        writer.start_group("title");
        render_title(&mut writer, layout, title);
        writer.end_group();
    }

    if !layout.participant_boxes.is_empty() {
        writer.start_group("participant-boxes");
        for participant_box in &layout.participant_boxes {
            render_participant_group_box(&mut writer, participant_box);
        }
        writer.end_group();
    }

    // Lifelines (behind everything else)
    writer.start_group("lifelines");
    for lifeline in &layout.lifelines {
        render_lifeline(&mut writer, lifeline);
    }
    for marker in &layout.destroy_markers {
        render_destroy_marker(&mut writer, marker);
    }
    writer.end_group();

    if !layout.blocks.is_empty() {
        writer.start_group("blocks");
        for block in &layout.blocks {
            render_block(&mut writer, block);
        }
        writer.end_group();
    }

    // Activation bars (behind messages, on top of lifelines)
    if !layout.activations.is_empty() {
        writer.start_group("activations");
        for activation in &layout.activations {
            render_activation(&mut writer, activation);
        }
        writer.end_group();
    }

    // Participant boxes (on top of lifelines)
    writer.start_group("participants");
    for participant in &layout.participants {
        render_participant(&mut writer, participant);
    }
    writer.end_group();

    // Messages and notes
    writer.start_group("events");
    for row in &layout.rows {
        match row {
            SvgRow::Message(msg) => render_message(&mut writer, msg),
            SvgRow::SelfMessage(sm) => render_self_message(&mut writer, sm),
            SvgRow::Note(note) => render_note(&mut writer, note),
        }
    }
    writer.end_group();

    writer.end_svg();
    writer.finish()
}

fn render_title(writer: &mut SvgWriter, layout: &SvgSequenceLayout, title: &SvgTitle) {
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{TEXT_COLOR}\">{title}</text>",
        x = fmt_f64(layout.width / 2.0),
        y = fmt_f64(title.y),
        title = escape_text(&title.text),
    ));
}

fn render_participant_group_box(writer: &mut SvgWriter, participant_box: &SvgParticipantBox) {
    let rect = &participant_box.rect;
    let fill = participant_box
        .color
        .as_deref()
        .filter(|color| !color.eq_ignore_ascii_case("transparent"));

    match fill {
        Some(color) => writer.push_line(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{fill}\" fill-opacity=\"0.14\" stroke=\"{stroke}\" stroke-width=\"1\" />",
            x = fmt_f64(rect.x),
            y = fmt_f64(rect.y),
            w = fmt_f64(rect.width),
            h = fmt_f64(rect.height),
            fill = escape_text(color),
            stroke = PARTICIPANT_BOX_STROKE,
        )),
        None => writer.push_line(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1\" />",
            x = fmt_f64(rect.x),
            y = fmt_f64(rect.y),
            w = fmt_f64(rect.width),
            h = fmt_f64(rect.height),
            stroke = PARTICIPANT_BOX_STROKE,
        )),
    }

    if let Some(label) = participant_box.label.as_deref() {
        let label_x = rect.x + rect.width / 2.0;
        let label_y = rect.y + 14.0;
        let label_bg_width = ((label.chars().count() as f64) * 8.0 + 24.0)
            .max(48.0)
            .min(rect.width - 8.0);
        writer.push_line(&format!(
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"20\" fill=\"{bg}\" stroke=\"none\" />",
            x = fmt_f64(label_x - label_bg_width / 2.0),
            y = fmt_f64(rect.y + 2.0),
            w = fmt_f64(label_bg_width),
            bg = PARTICIPANT_BOX_LABEL_BG,
        ));
        writer.push_line(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{TEXT_COLOR}\">{label}</text>",
            x = fmt_f64(label_x),
            y = fmt_f64(label_y),
            label = escape_text(label),
        ));
    }
}

// ---------------------------------------------------------------------------
// Marker definitions
// ---------------------------------------------------------------------------

fn render_defs(writer: &mut SvgWriter) {
    writer.start_tag("<defs>");

    // Filled arrowhead (solid triangle)
    writer.start_tag(
        "<marker id=\"seq-arrowhead\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\" \
         markerUnits=\"userSpaceOnUse\">",
    );
    writer.push_line(&format!(
        "<path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"{STROKE_COLOR}\" />"
    ));
    writer.end_tag("</marker>");

    // Open arrowhead (hollow triangle)
    writer.start_tag(
        "<marker id=\"seq-open-arrowhead\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\" \
         markerUnits=\"userSpaceOnUse\">",
    );
    writer.push_line(&format!(
        "<polygon points=\"0,0 10,5 0,10\" fill=\"white\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" />"
    ));
    writer.end_tag("</marker>");

    // Cross marker (X shape)
    writer.start_tag(
        "<marker id=\"seq-crosshead\" viewBox=\"0 0 11 11\" refX=\"11\" refY=\"5.5\" \
         markerWidth=\"11\" markerHeight=\"11\" orient=\"auto-start-reverse\" \
         markerUnits=\"userSpaceOnUse\">",
    );
    writer.push_line(&format!(
        "<path d=\"M 1,1 l 9,9 M 10,1 l -9,9\" stroke=\"{STROKE_COLOR}\" stroke-width=\"2\" />"
    ));
    writer.end_tag("</marker>");

    // Async arrowhead (open arrow — just two lines forming a chevron)
    writer.start_tag(
        "<marker id=\"seq-async-arrowhead\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" \
         markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\" \
         markerUnits=\"userSpaceOnUse\">",
    );
    writer.push_line(&format!(
        "<path d=\"M 0 0 L 10 5 L 0 10\" fill=\"none\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1.5\" />"
    ));
    writer.end_tag("</marker>");

    writer.end_tag("</defs>");
}

// ---------------------------------------------------------------------------
// Element renderers
// ---------------------------------------------------------------------------

fn render_participant(writer: &mut SvgWriter, p: &SvgParticipant) {
    match p.kind {
        ParticipantKind::Participant => render_participant_box(writer, p),
        ParticipantKind::Actor => render_actor(writer, p),
    }
}

fn render_participant_box(writer: &mut SvgWriter, p: &SvgParticipant) {
    let r = &p.rect;
    writer.push_line(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" \
         fill=\"{FILL_COLOR}\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" />",
        x = fmt_f64(r.x),
        y = fmt_f64(r.y),
        w = fmt_f64(r.width),
        h = fmt_f64(r.height),
    ));

    let text_x = p.center_x;
    let text_y = r.y + r.height / 2.0;
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" \
         fill=\"{TEXT_COLOR}\">{label}</text>",
        x = fmt_f64(text_x),
        y = fmt_f64(text_y),
        label = escape_text(&p.label),
    ));
}

fn render_actor(writer: &mut SvgWriter, p: &SvgParticipant) {
    // Stick figure centered at (center_x, rect center_y)
    let cx = p.center_x;
    let r = &p.rect;
    let top = r.y + 4.0;

    // Head (circle)
    let head_r = 8.0;
    let head_cy = top + head_r;
    writer.push_line(&format!(
        "<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" \
         fill=\"none\" stroke=\"{ACTOR_STROKE}\" stroke-width=\"1.5\" />",
        cx = fmt_f64(cx),
        cy = fmt_f64(head_cy),
        r = fmt_f64(head_r),
    ));

    // Body
    let body_top = head_cy + head_r;
    let body_bottom = body_top + 14.0;
    writer.push_line(&format!(
        "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" \
         stroke=\"{ACTOR_STROKE}\" stroke-width=\"1.5\" />",
        x = fmt_f64(cx),
        y1 = fmt_f64(body_top),
        y2 = fmt_f64(body_bottom),
    ));

    // Arms
    let arm_y = body_top + 6.0;
    let arm_span = 14.0;
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" \
         stroke=\"{ACTOR_STROKE}\" stroke-width=\"1.5\" />",
        x1 = fmt_f64(cx - arm_span),
        x2 = fmt_f64(cx + arm_span),
        y = fmt_f64(arm_y),
    ));

    // Legs
    let leg_span = 10.0;
    let leg_bottom = body_bottom + 12.0;
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" \
         stroke=\"{ACTOR_STROKE}\" stroke-width=\"1.5\" />",
        x1 = fmt_f64(cx),
        y1 = fmt_f64(body_bottom),
        x2 = fmt_f64(cx - leg_span),
        y2 = fmt_f64(leg_bottom),
    ));
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" \
         stroke=\"{ACTOR_STROKE}\" stroke-width=\"1.5\" />",
        x1 = fmt_f64(cx),
        y1 = fmt_f64(body_bottom),
        x2 = fmt_f64(cx + leg_span),
        y2 = fmt_f64(leg_bottom),
    ));

    // Label below the figure
    let label_y = r.y + r.height - 2.0;
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"auto\" \
         fill=\"{TEXT_COLOR}\">{label}</text>",
        x = fmt_f64(cx),
        y = fmt_f64(label_y),
        label = escape_text(&p.label),
    ));
}

fn render_lifeline(writer: &mut SvgWriter, ll: &SvgLifeline) {
    writer.push_line(&format!(
        "<line x1=\"{x}\" y1=\"{y1}\" x2=\"{x}\" y2=\"{y2}\" \
         stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" stroke-dasharray=\"5,5\" />",
        x = fmt_f64(ll.x),
        y1 = fmt_f64(ll.y_start),
        y2 = fmt_f64(ll.y_end),
    ));
}

fn render_destroy_marker(writer: &mut SvgWriter, marker: &SvgDestroyMarker) {
    let size = 6.0;
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1.5\" />",
        x1 = fmt_f64(marker.x - size),
        y1 = fmt_f64(marker.y - size),
        x2 = fmt_f64(marker.x + size),
        y2 = fmt_f64(marker.y + size),
    ));
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1.5\" />",
        x1 = fmt_f64(marker.x - size),
        y1 = fmt_f64(marker.y + size),
        x2 = fmt_f64(marker.x + size),
        y2 = fmt_f64(marker.y - size),
    ));
}

fn render_block(writer: &mut SvgWriter, block: &SvgBlock) {
    let rect = &block.rect;
    writer.push_line(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"{BLOCK_STROKE}\" stroke-width=\"1\" stroke-dasharray=\"{BLOCK_STROKE_DASH}\" />",
        x = fmt_f64(rect.x),
        y = fmt_f64(rect.y),
        w = fmt_f64(rect.width),
        h = fmt_f64(rect.height),
    ));

    let operator = block.kind.keyword();
    let tab_width = block_tab_width(operator);
    render_block_operator_tab(
        writer,
        operator,
        rect.x,
        rect.y,
        tab_width,
        BLOCK_TAB_HEIGHT,
    );

    if let Some(title) = format_fragment_guard(&block.label) {
        let title_x = rect.x + tab_width + (rect.width - tab_width) / 2.0;
        render_centered_block_text(writer, &title, title_x, rect.y + BLOCK_HEADER_TEXT_Y);
    }

    for divider in &block.dividers {
        render_block_divider(writer, rect, divider);
    }
}

fn render_block_divider(
    writer: &mut SvgWriter,
    rect: &super::svg_layout::SvgRect,
    divider: &SvgBlockDivider,
) {
    let _divider_keyword = divider.kind.keyword();
    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" stroke=\"{BLOCK_STROKE}\" stroke-width=\"1\" stroke-dasharray=\"{BLOCK_STROKE_DASH}\" />",
        x1 = fmt_f64(rect.x),
        x2 = fmt_f64(rect.x + rect.width),
        y = fmt_f64(divider.y),
    ));

    if let Some(label) = format_fragment_guard(&divider.label) {
        render_centered_block_text(
            writer,
            &label,
            rect.x + rect.width / 2.0,
            divider.y + BLOCK_SECTION_TEXT_Y,
        );
    }
}

fn render_block_operator_tab(
    writer: &mut SvgWriter,
    keyword: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let cut = BLOCK_TAB_CUT;
    writer.push_line(&format!(
        "<polygon points=\"{x0},{y0} {x1},{y0} {x1},{y1} {x2},{y2} {x0},{y2}\" fill=\"{BLOCK_LABEL_BG}\" stroke=\"{BLOCK_STROKE}\" stroke-width=\"1\" />",
        x0 = fmt_f64(x),
        y0 = fmt_f64(y),
        x1 = fmt_f64(x + width),
        y1 = fmt_f64(y + height - cut),
        x2 = fmt_f64(x + width - cut * 1.2),
        y2 = fmt_f64(y + height),
    ));
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{TEXT_COLOR}\">{label}</text>",
        x = fmt_f64(x + width / 2.0),
        y = fmt_f64(y + height / 2.0),
        label = escape_text(keyword),
    ));
}

fn render_centered_block_text(writer: &mut SvgWriter, label: &str, x: f64, y: f64) {
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{TEXT_COLOR}\">{label}</text>",
        x = fmt_f64(x),
        y = fmt_f64(y),
        label = escape_text(label),
    ));
}

fn render_message(writer: &mut SvgWriter, msg: &SvgMessage) {
    let marker = marker_attr(&msg.arrow_head);
    let dash = dash_attr(&msg.line_style);

    writer.push_line(&format!(
        "<line x1=\"{x1}\" y1=\"{y}\" x2=\"{x2}\" y2=\"{y}\" \
         stroke=\"{STROKE_COLOR}\" stroke-width=\"1\"{dash} {marker} />",
        x1 = fmt_f64(msg.from_x),
        x2 = fmt_f64(msg.to_x),
        y = fmt_f64(msg.y),
    ));

    if !msg.label.is_empty() {
        writer.push_line(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"auto\" \
             fill=\"{TEXT_COLOR}\">{label}</text>",
            x = fmt_f64(msg.label_x),
            y = fmt_f64(msg.label_y),
            label = escape_text(&msg.label),
        ));
    }
}

fn render_self_message(writer: &mut SvgWriter, sm: &SvgSelfMessage) {
    let x = sm.x;
    let y = sm.y;
    let arm = sm.arm_width;
    let h = sm.height;
    let dash = dash_attr(&sm.line_style);
    let marker = marker_attr(&sm.arrow_head);

    // Right-angle loop: right → down → left with arrow
    writer.push_line(&format!(
        "<path d=\"M {x0} {y0} L {x1} {y0} L {x1} {y1} L {x0} {y1}\" \
         fill=\"none\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\"{dash} {marker} />",
        x0 = fmt_f64(x),
        y0 = fmt_f64(y),
        x1 = fmt_f64(x + arm),
        y1 = fmt_f64(y + h),
    ));

    if !sm.label.is_empty() {
        writer.push_line(&format!(
            "<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\" dominant-baseline=\"auto\" \
             fill=\"{TEXT_COLOR}\">{label}</text>",
            x = fmt_f64(sm.label_x),
            y = fmt_f64(sm.label_y),
            label = escape_text(&sm.label),
        ));
    }
}

fn render_note(writer: &mut SvgWriter, note: &SvgNote) {
    let r = &note.rect;

    // Folded-corner note box
    let fold = 7.0;
    let x = r.x;
    let y = r.y;
    let w = r.width;
    let h = r.height;

    // Main body with folded corner
    writer.push_line(&format!(
        "<path d=\"M {x0} {y0} L {x1} {y0} L {x2} {y1} L {x2} {y2} L {x0} {y2} Z\" \
         fill=\"{NOTE_FILL}\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" />",
        x0 = fmt_f64(x),
        y0 = fmt_f64(y),
        x1 = fmt_f64(x + w - fold),
        x2 = fmt_f64(x + w),
        y1 = fmt_f64(y + fold),
        y2 = fmt_f64(y + h),
    ));

    // Fold line
    writer.push_line(&format!(
        "<path d=\"M {x1} {y0} L {x1} {y1} L {x2} {y1}\" \
         fill=\"none\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" />",
        x1 = fmt_f64(x + w - fold),
        y0 = fmt_f64(y),
        x2 = fmt_f64(x + w),
        y1 = fmt_f64(y + fold),
    ));

    // Text centered in the note
    let text_x = x + w / 2.0;
    let text_y = y + h / 2.0;
    writer.push_line(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\" dominant-baseline=\"middle\" \
         fill=\"{TEXT_COLOR}\">{text}</text>",
        x = fmt_f64(text_x),
        y = fmt_f64(text_y),
        text = escape_text(&note.text),
    ));
}

fn render_activation(writer: &mut SvgWriter, act: &SvgActivation) {
    writer.push_line(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" \
         fill=\"{ACTIVATION_FILL}\" stroke=\"{STROKE_COLOR}\" stroke-width=\"1\" />",
        x = fmt_f64(act.x),
        y = fmt_f64(act.y_start),
        w = fmt_f64(act.width),
        h = fmt_f64(act.y_end - act.y_start),
    ));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn marker_attr(arrow_head: &ArrowHead) -> String {
    let id = match arrow_head {
        ArrowHead::Filled => "seq-arrowhead",
        ArrowHead::Open => "seq-open-arrowhead",
        ArrowHead::Cross => "seq-crosshead",
        ArrowHead::Async => "seq-async-arrowhead",
    };
    format!("marker-end=\"url(#{id})\"")
}

fn dash_attr(line_style: &LineStyle) -> String {
    match line_style {
        LineStyle::Solid => String::new(),
        LineStyle::Dashed => " stroke-dasharray=\"6,4\"".to_string(),
    }
}

fn block_tab_width(keyword: &str) -> f64 {
    ((keyword.len() as f64) * BLOCK_TAB_CHAR_WIDTH + BLOCK_TAB_PADDING_X * 2.0)
        .max(BLOCK_TAB_MIN_WIDTH)
}

fn format_fragment_guard(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("[{trimmed}]"))
    }
}
