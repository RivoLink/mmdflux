//! Shared SVG writing utilities used by both graph and sequence renderers.

pub(crate) fn fmt_f64(value: f64) -> String {
    let mut v = value;
    if v.abs() < 0.005 {
        v = 0.0;
    }
    format!("{:.2}", v)
}

pub(crate) fn escape_text(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) struct SvgWriter {
    buf: String,
    indent: usize,
}

impl SvgWriter {
    pub(crate) fn new() -> Self {
        Self {
            buf: String::new(),
            indent: 0,
        }
    }

    pub(crate) fn start_svg(&mut self, width: f64, height: f64, font_family: &str, font_size: f64) {
        let view_width = fmt_f64(width);
        let view_height = fmt_f64(height);
        let view_box = format!("0 0 {view_width} {view_height}");
        let style = format!("max-width: {view_width}px; background-color: transparent;");
        let line = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100%\" viewBox=\"{view_box}\" style=\"{style}\" font-family=\"{font}\" font-size=\"{font_size}\">",
            view_box = view_box,
            style = style,
            font = escape_text(font_family),
            font_size = fmt_f64(font_size)
        );
        self.push_line(&line);
        self.indent += 1;
    }

    pub(crate) fn end_svg(&mut self) {
        self.indent = self.indent.saturating_sub(1);
        self.push_line("</svg>");
    }

    pub(crate) fn start_tag(&mut self, line: &str) {
        self.push_line(line);
        self.indent += 1;
    }

    pub(crate) fn end_tag(&mut self, line: &str) {
        self.indent = self.indent.saturating_sub(1);
        self.push_line(line);
    }

    pub(crate) fn start_group(&mut self, class_name: &str) {
        let line = format!("<g class=\"{class}\">", class = escape_text(class_name));
        self.start_tag(&line);
    }

    pub(crate) fn start_group_transform(&mut self, dx: f64, dy: f64) {
        let line = format!(
            "<g transform=\"translate({x},{y})\">",
            x = fmt_f64(dx),
            y = fmt_f64(dy)
        );
        self.start_tag(&line);
    }

    pub(crate) fn end_group(&mut self) {
        self.end_tag("</g>");
    }

    pub(crate) fn push_line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(line);
        self.buf.push('\n');
    }

    pub(crate) fn finish(self) -> String {
        self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_text, fmt_f64};

    #[test]
    fn fmt_f64_snaps_tiny_values_to_zero() {
        assert_eq!(fmt_f64(0.004), "0.00");
        assert_eq!(fmt_f64(-0.004), "0.00");
        assert_eq!(fmt_f64(12.345), "12.35");
    }

    #[test]
    fn escape_text_escapes_xml_significant_characters() {
        assert_eq!(
            escape_text("<tag attr=\"a&b\">it's</tag>"),
            "&lt;tag attr=&quot;a&amp;b&quot;&gt;it&apos;s&lt;/tag&gt;"
        );
    }
}
