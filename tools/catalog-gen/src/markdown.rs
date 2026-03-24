use roxmltree::Node;

/// Convert a `<definitionitem>` DOM subtree to Markdown.
///
/// Recursively walks the XML tree and produces formatted Markdown text.
pub fn definition_to_markdown(node: Node) -> String {
    let mut ctx = MarkdownContext::new();
    convert_children(node, &mut ctx);
    ctx.finish()
}

/// Convert any element subtree to Markdown (used for arbitrary nodes).
#[cfg(test)]
pub fn node_to_markdown(node: Node) -> String {
    let mut ctx = MarkdownContext::new();
    convert_node(node, &mut ctx);
    ctx.finish()
}

// ---------------------------------------------------------------------------
// Internal context
// ---------------------------------------------------------------------------

struct MarkdownContext {
    /// Accumulated output
    buf: String,
    /// Current list nesting depth (for indentation)
    list_depth: usize,
    /// Whether we are inside a paragraph (for inline element handling)
    in_para: bool,
}

impl MarkdownContext {
    fn new() -> Self {
        Self {
            buf: String::new(),
            list_depth: 0,
            in_para: false,
        }
    }

    fn finish(self) -> String {
        let mut s = self.buf.trim_end().to_string();
        s.push('\n');
        // Post-process: convert figure text references to image syntax
        s = convert_figure_references(&s);
        s
    }

    /// Ensure there is a blank line before new block-level content.
    fn ensure_blank_line(&mut self) {
        let trimmed = self.buf.trim_end_matches(' ');
        if trimmed.is_empty() {
            return;
        }
        if !trimmed.ends_with("\n\n") {
            if trimmed.ends_with('\n') {
                self.buf.truncate(trimmed.len());
                self.buf.push_str("\n\n");
            } else {
                self.buf.push_str("\n\n");
            }
        }
    }

    /// Ensure the current line ends with a newline.
    fn ensure_newline(&mut self) {
        if !self.buf.is_empty() && !self.buf.ends_with('\n') {
            self.buf.push('\n');
        }
    }

    /// Return indentation string for current list depth.
    fn indent(&self) -> String {
        "  ".repeat(self.list_depth)
    }

    fn push(&mut self, s: &str) {
        self.buf.push_str(s);
    }
}

// ---------------------------------------------------------------------------
// Core conversion
// ---------------------------------------------------------------------------

fn convert_children(node: Node, ctx: &mut MarkdownContext) {
    for child in node.children() {
        if child.is_text() {
            if ctx.in_para {
                let text = child.text().unwrap_or("");
                // Normalize whitespace within inline context
                let normalized = normalize_inline_whitespace(text);
                if !normalized.is_empty() {
                    ctx.push(&normalized);
                }
            }
        } else if child.is_element() {
            convert_node(child, ctx);
        }
    }
}

fn convert_node(node: Node, ctx: &mut MarkdownContext) {
    let tag = node.tag_name().name();

    match tag {
        "para" => convert_para(node, ctx),
        "code" => convert_inline_code(node, ctx),
        "var" => convert_italic(node, ctx),
        "emph" | "i" => convert_italic(node, ctx),
        "b" | "strong" => convert_bold(node, ctx),
        "example" => convert_example(node, ctx),
        "itemize" => convert_itemize(node, ctx),
        "enumerate" => convert_enumerate(node, ctx),
        "table" => convert_table(node, ctx),
        "multitable" => convert_multitable(node, ctx),
        "ref" | "xref" => convert_ref(node, ctx),
        "uref" | "url" => convert_url(node, ctx),
        "html" => convert_html(node, ctx),
        "tex" => { /* skip — duplicate of html math */ }
        "pre" => convert_pre(node, ctx),
        "group" => convert_children(node, ctx),
        "examplelanguage" => { /* skip */ }
        "definitionterm" | "indexterm" | "defcategory" => { /* skip metadata */ }
        "deffnx" | "defvrx" => { /* skip alt signatures — handled elsewhere */ }
        "anchor" => { /* skip */ }
        "quotation" => convert_blockquote(node, ctx),
        "w" => convert_children(node, ctx), // @w{} = "no-break" wrapper
        "dfn" => convert_italic(node, ctx),
        "math" => convert_inline_math(node, ctx),
        "sc" => convert_smallcaps(node, ctx),
        "key" => convert_kbd(node, ctx),
        "kbd" | "samp" | "command" | "env" | "file" | "option" => {
            convert_inline_code(node, ctx)
        }
        "footnote" => { /* skip footnotes in markdown output */ }
        "sup" | "sub" => convert_children(node, ctx),
        "itemformat" => convert_children(node, ctx), // @table @code wraps items in this
        "item" => convert_children(node, ctx),
        "formattingcommand" | "itemprepend" | "prepend" | "need" | "beforefirstitem" => { /* skip */ }
        "columnfractions" | "columnfraction" => { /* skip — multitable metadata */ }
        _ => {
            // For unknown elements, just recurse into children
            convert_children(node, ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// Block elements
// ---------------------------------------------------------------------------

fn convert_para(node: Node, ctx: &mut MarkdownContext) {
    ctx.ensure_blank_line();
    let indent = ctx.indent();
    ctx.push(&indent);
    ctx.in_para = true;
    convert_children(node, ctx);
    ctx.in_para = false;
    ctx.ensure_blank_line();
}

fn convert_example(node: Node, ctx: &mut MarkdownContext) {
    ctx.ensure_blank_line();
    ctx.push("```maxima\n");

    // Collect raw text from <pre> children (or direct text)
    let text = collect_raw_text(node);
    let trimmed = text.trim_end();
    ctx.push(trimmed);
    ctx.ensure_newline();
    ctx.push("```\n");
    ctx.ensure_blank_line();
}

fn convert_pre(node: Node, ctx: &mut MarkdownContext) {
    let text = collect_raw_text(node);
    ctx.push(&text);
}

fn convert_itemize(node: Node, ctx: &mut MarkdownContext) {
    ctx.ensure_blank_line();
    for child in node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "listitem" => {
                    convert_list_item(child, ctx, "- ");
                }
                "prepend" => { /* skip bullet marker definition */ }
                _ => {}
            }
        }
    }
}

fn convert_enumerate(node: Node, ctx: &mut MarkdownContext) {
    ctx.ensure_blank_line();
    let mut counter = 1;
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "listitem" {
            let marker = format!("{counter}. ");
            convert_list_item(child, ctx, &marker);
            counter += 1;
        }
    }
}

fn convert_list_item(node: Node, ctx: &mut MarkdownContext, marker: &str) {
    let indent = ctx.indent();
    ctx.ensure_newline();
    ctx.push(&indent);
    ctx.push(marker);

    ctx.list_depth += 1;

    // Process children — first para's content is inline with the marker
    let mut first_para = true;
    for child in node.children() {
        if child.is_element() {
            let tag = child.tag_name().name();
            if tag == "para" && first_para {
                // Render first paragraph inline with the bullet marker
                ctx.in_para = true;
                convert_children(child, ctx);
                ctx.in_para = false;
                ctx.ensure_newline();
                first_para = false;
            } else {
                // Subsequent blocks get full block treatment
                convert_node(child, ctx);
            }
        }
    }

    ctx.list_depth -= 1;
}

fn convert_table(node: Node, ctx: &mut MarkdownContext) {
    // Texinfo @table: definition-style list with <tableentry> > <tableterm> + <tableitem>
    ctx.ensure_blank_line();

    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "tableentry" {
            let mut term = String::new();
            let mut desc = String::new();

            for entry_child in child.children() {
                if entry_child.is_element() {
                    match entry_child.tag_name().name() {
                        "tableterm" => {
                            let mut term_ctx = MarkdownContext::new();
                            term_ctx.in_para = true;
                            convert_children(entry_child, &mut term_ctx);
                            term = term_ctx.finish().trim().to_string();
                        }
                        "tableitem" | "item" => {
                            let mut desc_ctx = MarkdownContext::new();
                            convert_children(entry_child, &mut desc_ctx);
                            desc = desc_ctx.finish().trim().to_string();
                        }
                        _ => {}
                    }
                }
            }

            let indent = ctx.indent();
            if !term.is_empty() {
                ctx.push(&indent);
                ctx.push("**");
                ctx.push(&term);
                ctx.push("**");
                if !desc.is_empty() {
                    ctx.push(" — ");
                    // Put first line inline, rest indented below
                    let lines: Vec<&str> = desc.lines().collect();
                    if let Some(first) = lines.first() {
                        ctx.push(first.trim());
                        ctx.ensure_newline();
                        for line in &lines[1..] {
                            if line.trim().is_empty() {
                                ctx.push("\n");
                            } else {
                                ctx.push(&indent);
                                ctx.push("  ");
                                ctx.push(line.trim());
                                ctx.ensure_newline();
                            }
                        }
                    }
                }
                ctx.ensure_newline();
            }
        }
    }
    ctx.ensure_blank_line();
}

fn convert_multitable(node: Node, ctx: &mut MarkdownContext) {
    // Texinfo @multitable: real table with columns
    ctx.ensure_blank_line();

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut has_header = false;

    for child in node.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "thead" => {
                    has_header = true;
                    for row_node in child.children() {
                        if row_node.is_element() && row_node.tag_name().name() == "row" {
                            rows.push(extract_row_cells(row_node));
                        }
                    }
                }
                "tbody" => {
                    for row_node in child.children() {
                        if row_node.is_element() && row_node.tag_name().name() == "row" {
                            rows.push(extract_row_cells(row_node));
                        }
                    }
                }
                "row" => {
                    rows.push(extract_row_cells(child));
                }
                _ => {}
            }
        }
    }

    if rows.is_empty() {
        return;
    }

    // Determine column count
    let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return;
    }

    // Normalize row lengths
    for row in &mut rows {
        while row.len() < num_cols {
            row.push(String::new());
        }
    }

    // Calculate column widths
    let col_widths: Vec<usize> = (0..num_cols)
        .map(|c| {
            rows.iter()
                .map(|r| r[c].len())
                .max()
                .unwrap_or(3)
                .max(3)
        })
        .collect();

    // Render header row (first row if has_header, or synthesized)
    let header_idx = if has_header { 1 } else { 0 };
    let start_idx;

    if has_header && !rows.is_empty() {
        render_table_row(&rows[0], &col_widths, ctx);
        render_separator_row(&col_widths, ctx);
        start_idx = 1;
    } else {
        // No explicit header — use first row as header
        if !rows.is_empty() {
            render_table_row(&rows[0], &col_widths, ctx);
            render_separator_row(&col_widths, ctx);
            start_idx = 1;
        } else {
            start_idx = 0;
        }
    }

    let _ = header_idx; // suppress warning

    for row in &rows[start_idx..] {
        render_table_row(row, &col_widths, ctx);
    }

    ctx.ensure_blank_line();
}

fn extract_row_cells(row_node: Node) -> Vec<String> {
    let mut cells = Vec::new();
    for child in row_node.children() {
        if child.is_element() && child.tag_name().name() == "entry" {
            let mut cell_ctx = MarkdownContext::new();
            cell_ctx.in_para = true;
            convert_children(child, &mut cell_ctx);
            cells.push(cell_ctx.finish().trim().to_string());
        }
    }
    cells
}

fn render_table_row(row: &[String], widths: &[usize], ctx: &mut MarkdownContext) {
    ctx.push("| ");
    for (i, cell) in row.iter().enumerate() {
        if i > 0 {
            ctx.push(" | ");
        }
        ctx.push(cell);
        let padding = widths[i].saturating_sub(cell.len());
        for _ in 0..padding {
            ctx.push(" ");
        }
    }
    ctx.push(" |\n");
}

fn render_separator_row(widths: &[usize], ctx: &mut MarkdownContext) {
    ctx.push("| ");
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            ctx.push(" | ");
        }
        for _ in 0..*w {
            ctx.push("-");
        }
    }
    ctx.push(" |\n");
}

fn convert_blockquote(node: Node, ctx: &mut MarkdownContext) {
    ctx.ensure_blank_line();
    let mut inner = MarkdownContext::new();
    convert_children(node, &mut inner);
    let text = inner.finish();
    for line in text.lines() {
        ctx.push("> ");
        ctx.push(line);
        ctx.push("\n");
    }
    ctx.ensure_blank_line();
}

// ---------------------------------------------------------------------------
// Inline elements
// ---------------------------------------------------------------------------

fn convert_inline_code(node: Node, ctx: &mut MarkdownContext) {
    let text = collect_inline_text(node);
    if !text.is_empty() {
        ctx.push("`");
        ctx.push(&text);
        ctx.push("`");
    }
}

fn convert_italic(node: Node, ctx: &mut MarkdownContext) {
    let was_in_para = ctx.in_para;
    ctx.in_para = true;
    ctx.push("*");
    convert_children(node, ctx);
    ctx.push("*");
    ctx.in_para = was_in_para;
}

fn convert_bold(node: Node, ctx: &mut MarkdownContext) {
    let was_in_para = ctx.in_para;
    ctx.in_para = true;
    ctx.push("**");
    convert_children(node, ctx);
    ctx.push("**");
    ctx.in_para = was_in_para;
}

fn convert_inline_math(node: Node, ctx: &mut MarkdownContext) {
    let text = collect_inline_text(node);
    if !text.is_empty() {
        ctx.push("$");
        ctx.push(&text);
        ctx.push("$");
    }
}

fn convert_smallcaps(node: Node, ctx: &mut MarkdownContext) {
    // No standard Markdown for small caps — just uppercase
    let text = collect_inline_text(node);
    ctx.push(&text.to_uppercase());
}

fn convert_kbd(node: Node, ctx: &mut MarkdownContext) {
    let text = collect_inline_text(node);
    if !text.is_empty() {
        ctx.push("`");
        ctx.push(&text);
        ctx.push("`");
    }
}

fn convert_ref(node: Node, ctx: &mut MarkdownContext) {
    if let Some(label) = node.attribute("label") {
        let label = label.trim();
        if !label.is_empty() {
            // Use custom fn: scheme for internal cross-references
            ctx.push("[");
            // Use xrefnodename child text if available, otherwise label
            let display = find_child_text(node, "xrefnodename")
                .unwrap_or_else(|| label.to_string());
            ctx.push(&display);
            ctx.push("](fn:");
            ctx.push(label);
            ctx.push(")");
        }
    }
}

fn convert_url(node: Node, ctx: &mut MarkdownContext) {
    // <uref><urefurl>URL</urefurl><urefdesc>text</urefdesc></uref>
    let url = find_child_text(node, "urefurl").unwrap_or_default();
    let desc = find_child_text(node, "urefdesc")
        .or_else(|| find_child_text(node, "urefreplacement"))
        .unwrap_or_else(|| url.clone());

    if !url.is_empty() {
        ctx.push("[");
        ctx.push(&desc);
        ctx.push("](");
        ctx.push(&url);
        ctx.push(")");
    }
}

fn convert_html(node: Node, ctx: &mut MarkdownContext) {
    // The <html> element contains raw Texinfo markup like @math{...}
    // or @displaymath ... @end displaymath
    let raw = collect_raw_text(node);

    // @math{...} → $...$
    if let Some(rest) = raw.trim().strip_prefix("@math{") {
        if let Some(content) = rest.strip_suffix('}') {
            ctx.push("$");
            ctx.push(content.trim());
            ctx.push("$");
            return;
        }
    }

    // @displaymath ... @end displaymath → $$...$$
    if let Some(rest) = raw.trim().strip_prefix("@displaymath") {
        if let Some(end_pos) = rest.find("@end displaymath") {
            let content = rest[..end_pos].trim();
            ctx.ensure_blank_line();
            ctx.push("$$");
            ctx.push(content);
            ctx.push("$$");
            ctx.ensure_blank_line();
            return;
        }
    }

    // Figure macro references: (Figure name) or (Figure name: desc)
    let trimmed = raw.trim();
    if trimmed.starts_with("(Figure ") && trimmed.ends_with(')') {
        let inner = &trimmed[8..trimmed.len() - 1]; // strip "(Figure " and ")"
        let (name, desc) = if let Some(colon) = inner.find(':') {
            (inner[..colon].trim(), inner[colon + 1..].trim())
        } else {
            (inner.trim(), inner.trim())
        };
        ctx.ensure_blank_line();
        ctx.push("![");
        ctx.push(desc);
        ctx.push("](figures/");
        ctx.push(name);
        ctx.push(".png)");
        ctx.ensure_blank_line();
        return;
    }

    // Fallback: strip @command{content} patterns and emit remaining text
    let cleaned = clean_texinfo_in_html(&raw);
    if !cleaned.trim().is_empty() {
        ctx.push(&cleaned);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect all text content from a node, preserving whitespace structure.
fn collect_raw_text(node: Node) -> String {
    let mut text = String::new();
    for child in node.children() {
        if child.is_text() {
            text.push_str(child.text().unwrap_or(""));
        } else if child.is_element() {
            text.push_str(&collect_raw_text(child));
        }
    }
    text
}

/// Collect inline text: recursively get text but strip leading/trailing whitespace.
fn collect_inline_text(node: Node) -> String {
    let raw = collect_raw_text(node);
    normalize_inline_whitespace(&raw).trim().to_string()
}

/// Find the text content of the first child element with the given tag name.
fn find_child_text(node: Node, tag: &str) -> Option<String> {
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == tag {
            let text = collect_raw_text(child).trim().to_string();
            if !text.is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// Convert figure text references like `(Figure name: desc)` to Markdown image syntax.
fn convert_figure_references(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut remaining = s;

    while let Some(start) = remaining.find("(Figure ") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 8..]; // skip "(Figure "
        if let Some(end) = after.find(')') {
            let inner = &after[..end];
            let (name, desc) = if let Some(colon) = inner.find(':') {
                (inner[..colon].trim(), inner[colon + 1..].trim())
            } else {
                (inner.trim(), inner.trim())
            };
            result.push_str("![");
            result.push_str(desc);
            result.push_str("](figures/");
            result.push_str(name);
            result.push_str(".png)");
            remaining = &after[end + 1..];
        } else {
            // No closing paren — emit as-is
            result.push_str("(Figure ");
            remaining = after;
        }
    }

    result.push_str(remaining);
    result
}

/// Normalize whitespace for inline text: collapse runs of whitespace to single spaces.
fn normalize_inline_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_ws && !result.is_empty() {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            result.push(ch);
            prev_ws = false;
        }
    }
    result
}

/// Clean Texinfo markup embedded in <html> elements.
fn clean_texinfo_in_html(s: &str) -> String {
    let mut result = s.to_string();

    // @math{...} → $...$
    while let Some(start) = result.find("@math{") {
        let content_start = start + 6;
        if let Some(end) = find_matching_brace(&result, content_start) {
            let latex = result[content_start..end].trim();
            result = format!("{}${latex}${}", &result[..start], &result[end + 1..]);
        } else {
            break;
        }
    }

    // @displaymath ... @end displaymath → $$...$$
    while let Some(start) = result.find("@displaymath") {
        let content_start = start + "@displaymath".len();
        if let Some(end_pos) = result[content_start..].find("@end displaymath") {
            let latex = result[content_start..content_start + end_pos].trim();
            let after = content_start + end_pos + "@end displaymath".len();
            result = format!("{}$${latex}$${}", &result[..start], &result[after..]);
        } else {
            break;
        }
    }

    // Strip remaining @command{content} patterns, keeping content
    loop {
        if let Some(at_pos) = result.find('@') {
            let rest = &result[at_pos + 1..];
            let cmd_len = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if cmd_len > 0 && rest[cmd_len..].starts_with('{') {
                let content_start = at_pos + 1 + cmd_len + 1;
                if let Some(end) = find_matching_brace(&result, content_start) {
                    let content = result[content_start..end].to_string();
                    result = format!("{}{content}{}", &result[..at_pos], &result[end + 1..]);
                    continue;
                }
            }
            // @@ → @
            if rest.starts_with('@') {
                result = format!("{}@{}", &result[..at_pos], &result[at_pos + 2..]);
                continue;
            }
            break;
        } else {
            break;
        }
    }

    result
}

/// Find the position of the closing brace matching an opening brace.
fn find_matching_brace(s: &str, start: usize) -> Option<usize> {
    let mut depth = 1;
    for (i, ch) in s[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + i);
                }
            }
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use roxmltree::{Document, ParsingOptions};

    fn parse_and_convert(xml: &str) -> String {
        let opts = ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        };
        let doc = Document::parse_with_options(xml, opts).expect("parse XML");
        let root = doc.root_element();
        node_to_markdown(root)
    }

    #[test]
    fn test_para() {
        let md = parse_and_convert("<root><para>Hello world.</para></root>");
        assert_eq!(md.trim(), "Hello world.");
    }

    #[test]
    fn test_multiple_paras() {
        let md = parse_and_convert(
            "<root><para>First paragraph.</para><para>Second paragraph.</para></root>",
        );
        assert!(md.contains("First paragraph.\n\nSecond paragraph."));
    }

    #[test]
    fn test_inline_code() {
        let md = parse_and_convert("<root><para>Use <code>diff(x)</code> to differentiate.</para></root>");
        assert!(md.contains("`diff(x)`"));
    }

    #[test]
    fn test_var_italic() {
        let md = parse_and_convert("<root><para>The variable <var>x</var> is free.</para></root>");
        assert!(md.contains("*x*"));
    }

    #[test]
    fn test_bold() {
        let md = parse_and_convert("<root><para><b>Important</b> note.</para></root>");
        assert!(md.contains("**Important**"));
    }

    #[test]
    fn test_example_code_block() {
        let md = parse_and_convert(
            r#"<root><example><pre xml:space="preserve">(%i1) diff(x^3, x);
(%o1)                           3 x^2
</pre></example></root>"#,
        );
        assert!(md.contains("```maxima"));
        assert!(md.contains("(%i1) diff(x^3, x);"));
        assert!(md.contains("```"));
    }

    #[test]
    fn test_itemize() {
        let md = parse_and_convert(
            "<root><itemize><listitem><para>Item one</para></listitem><listitem><para>Item two</para></listitem></itemize></root>",
        );
        assert!(md.contains("- Item one"));
        assert!(md.contains("- Item two"));
    }

    #[test]
    fn test_enumerate() {
        let md = parse_and_convert(
            "<root><enumerate><listitem><para>First</para></listitem><listitem><para>Second</para></listitem></enumerate></root>",
        );
        assert!(md.contains("1. First"));
        assert!(md.contains("2. Second"));
    }

    #[test]
    fn test_ref_link() {
        let md = parse_and_convert(
            r#"<root><para>See <ref label="diff"><xrefnodename>diff</xrefnodename></ref>.</para></root>"#,
        );
        assert!(md.contains("[diff](fn:diff)"));
    }

    #[test]
    fn test_url_link() {
        let md = parse_and_convert(
            "<root><para>Visit <uref><urefurl>https://example.com</urefurl><urefdesc>Example</urefdesc></uref>.</para></root>",
        );
        assert!(md.contains("[Example](https://example.com)"));
    }

    #[test]
    fn test_html_math_inline() {
        let md = parse_and_convert(
            "<root><para>The formula <html>@math{x^2 + y^2}</html> is important.</para></root>",
        );
        assert!(md.contains("$x^2 + y^2$"));
    }

    #[test]
    fn test_html_display_math() {
        let md = parse_and_convert(
            "<root><html>@displaymath x^2 + y^2 = r^2 @end displaymath</html></root>",
        );
        assert!(md.contains("$$x^2 + y^2 = r^2$$"));
    }

    #[test]
    fn test_figure() {
        let md = parse_and_convert(
            "<root><html>(Figure plotting1: A simple plot)</html></root>",
        );
        assert!(md.contains("![A simple plot](figures/plotting1.png)"));
    }

    #[test]
    fn test_table_definition_list() {
        let md = parse_and_convert(
            r#"<root><table><tableentry><tableterm><item><code>true</code></item></tableterm><tableitem><para>Always simplify.</para></tableitem></tableentry></table></root>"#,
        );
        assert!(md.contains("**`true`**"));
        assert!(md.contains("Always simplify."));
    }

    #[test]
    fn test_multitable() {
        let md = parse_and_convert(
            r#"<root><multitable><thead><row><entry>Name</entry><entry>Value</entry></row></thead><tbody><row><entry>x</entry><entry>1</entry></row></tbody></multitable></root>"#,
        );
        assert!(md.contains("| Name"));
        assert!(md.contains("| x"));
        assert!(md.contains("---"));
    }

    #[test]
    fn test_normalize_inline_whitespace() {
        assert_eq!(normalize_inline_whitespace("  hello   world  "), "hello world ");
    }

    #[test]
    fn test_nested_list() {
        let md = parse_and_convert(
            "<root><itemize><listitem><para>Outer item</para><itemize><listitem><para>Inner item</para></listitem></itemize></listitem></itemize></root>",
        );
        assert!(md.contains("- Outer item"));
        assert!(md.contains("  - Inner item"));
    }

    #[test]
    fn test_figure_in_para() {
        let md = parse_and_convert(
            "<root><para>(Figure plotting6: Plot of an explicit function)</para></root>",
        );
        assert!(md.contains("![Plot of an explicit function](figures/plotting6.png)"));
    }

    #[test]
    fn test_blockquote() {
        let md = parse_and_convert(
            "<root><quotation><para>This is quoted text.</para></quotation></root>",
        );
        assert!(md.contains("> "));
        assert!(md.contains("This is quoted text."));
    }
}
