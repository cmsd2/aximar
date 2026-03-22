use aximar_lib::catalog::types::{FunctionCategory, FunctionExample, MaximaFunction};
use roxmltree::{Document, Node, ParsingOptions};

use crate::mapping::map_category;

/// Parse a Maxima Texinfo XML document and extract function/variable definitions.
pub fn parse_xml(xml: &str, log_unmapped: bool, min_description: usize) -> Vec<MaximaFunction> {
    let xml = replace_texinfo_entities(xml);
    let opts = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(&xml, opts).expect("failed to parse XML");
    let root = doc.root_element();

    let mut functions = Vec::new();
    let current_chapter = String::new();

    collect_definitions(root, &mut functions, &current_chapter, log_unmapped, min_description);

    // Deduplicate by name (keep first occurrence which tends to be the primary definition)
    let mut seen = std::collections::HashSet::new();
    functions.retain(|f| seen.insert(f.name.to_lowercase()));

    // Sort by name for stable output
    functions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    functions
}

fn collect_definitions(
    node: Node,
    functions: &mut Vec<MaximaFunction>,
    current_chapter: &str,
    log_unmapped: bool,
    min_description: usize,
) {
    for child in node.children() {
        if child.is_element() {
            let tag = child.tag_name().name();
            match tag {
                "chapter" => {
                    // Extract chapter title and use it as context for category mapping
                    let chapter_title = extract_section_title(child);
                    collect_definitions(child, functions, &chapter_title, log_unmapped, min_description);
                }
                "deffn" | "defvr" => {
                    if let Some(func) = parse_definition(child, tag, current_chapter, log_unmapped) {
                        if func.description.len() >= min_description {
                            functions.push(func);
                        }
                    }
                }
                _ => {
                    collect_definitions(child, functions, current_chapter, log_unmapped, min_description);
                }
            }
        }
    }
}

/// Extract the title text from a <sectiontitle> child element.
fn extract_section_title(node: Node) -> String {
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "sectiontitle" {
            return collect_text(child).trim().to_string();
        }
    }
    String::new()
}

/// Parse a single `<deffn>` or `<defvr>` element into a MaximaFunction.
fn parse_definition(node: Node, tag: &str, chapter: &str, log_unmapped: bool) -> Option<MaximaFunction> {
    let name = extract_name(node, tag)?;

    // Skip internal/private names (starting with %)
    if name.starts_with('%') && name.len() > 2 {
        // Allow single-char % names like %pi, %e, etc.
    }

    let signatures = extract_signatures(node, tag, &name);
    let description = extract_description(node);
    let examples = extract_examples(node);
    let category = extract_category(node, tag, chapter, log_unmapped);
    let see_also = extract_see_also(node);

    Some(MaximaFunction {
        name,
        signatures,
        description,
        category,
        examples,
        see_also,
    })
}

/// Extract the function/variable name from the definition term.
fn extract_name(node: Node, tag: &str) -> Option<String> {
    // Try to find name from <definitionterm> > <deffunction> or <defvariable>
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "definitionterm" {
            let inner_tag = if tag == "deffn" {
                "deffunction"
            } else {
                "defvariable"
            };
            if let Some(name_node) = find_element(child, inner_tag) {
                let name = collect_text(name_node).trim().to_string();
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }

    // Fallback: look for indexterm
    if let Some(idx) = find_element(node, "indexterm") {
        let index_attr = idx.attribute("index");
        if index_attr == Some("fn") || index_attr == Some("vr") {
            let name = collect_text(idx).trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}

/// Extract all signatures (primary + alternatives from deffnx/defvrx).
fn extract_signatures(node: Node, tag: &str, name: &str) -> Vec<String> {
    let mut sigs = Vec::new();

    // Primary signatures from <definitionterm> (may contain multiple, separated by linebreaks)
    sigs.extend(extract_signatures_from_term(node, tag));

    // Alternative signatures from <deffnx> or <defvrx> siblings
    let alt_tag = format!("{tag}x");
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == alt_tag {
            sigs.extend(extract_signatures_from_term(child, tag));
        }
    }

    // Remove bare name if there are more specific signatures with arguments
    if sigs.len() > 1 {
        sigs.retain(|s| s != name);
    }

    // If no signatures found, synthesize one from the name
    if sigs.is_empty() {
        if tag == "deffn" {
            sigs.push(format!("{name}(...)"));
        } else {
            sigs.push(name.to_string());
        }
    }

    sigs
}

/// Reconstruct signature strings from a <definitionterm> element.
///
/// A single `<definitionterm>` may contain multiple signatures separated by
/// linebreak markers (`<defparam>\n</defparam>`). This function splits on those
/// markers and returns each signature separately.
fn extract_signatures_from_term(node: Node, tag: &str) -> Vec<String> {
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "definitionterm" {
            let func_tag = if tag == "deffn" || tag == "deffnx" {
                "deffunction"
            } else {
                "defvariable"
            };

            // Collect all parts, using None as a separator for linebreaks
            let mut parts: Vec<Option<String>> = Vec::new();
            let mut found_name = false;

            for term_child in child.children() {
                if term_child.is_element() {
                    let term_tag = term_child.tag_name().name();
                    match term_tag {
                        t if t == func_tag => {
                            parts.push(Some(collect_text(term_child).trim().to_string()));
                            found_name = true;
                        }
                        "defdelimiter" => {
                            let delim = collect_text(term_child).trim().to_string();
                            // Add space after comma delimiters for readability
                            if delim == "," {
                                parts.push(Some(", ".to_string()));
                            } else {
                                parts.push(Some(delim));
                            }
                        }
                        "defparam" | "var" => {
                            let text = collect_text(term_child);
                            let trimmed = text.trim();
                            // Linebreak separators between signatures
                            if trimmed.is_empty() || trimmed == "\n" {
                                parts.push(None); // separator
                            } else {
                                parts.push(Some(trimmed.to_string()));
                            }
                        }
                        "indexterm" | "defcategory" => {
                            // Skip metadata elements
                        }
                        _ => {}
                    }
                }
            }

            if !found_name && parts.iter().all(|p| p.is_none()) {
                return Vec::new();
            }

            // Split parts on None separators into individual signatures
            let mut sigs = Vec::new();
            let mut current = Vec::new();

            for part in parts {
                match part {
                    Some(s) => current.push(s),
                    None => {
                        if !current.is_empty() {
                            sigs.push(current.join(""));
                            current = Vec::new();
                        }
                    }
                }
            }
            if !current.is_empty() {
                sigs.push(current.join(""));
            }

            // Filter out empty signatures
            sigs.retain(|s| !s.trim().is_empty());

            return sigs;
        }
    }

    Vec::new()
}

/// Extract description text from <definitionitem> paragraphs.
fn extract_description(node: Node) -> String {
    let mut paragraphs = Vec::new();

    if let Some(item) = find_element(node, "definitionitem") {
        for child in item.children() {
            if child.is_element() {
                match child.tag_name().name() {
                    "para" => {
                        let text = collect_text(child).trim().to_string();
                        if !text.is_empty() {
                            paragraphs.push(text);
                        }
                    }
                    // Stop at examples — description comes before them
                    "example" => break,
                    _ => {}
                }
            }

            // Limit to first 3 paragraphs
            if paragraphs.len() >= 3 {
                break;
            }
        }
    }

    let desc = paragraphs.join(" ");
    let desc = normalize_whitespace(&desc);
    clean_texinfo_markup(&desc)
}

/// Extract examples from <example> blocks inside the definition.
fn extract_examples(node: Node) -> Vec<FunctionExample> {
    let mut examples = Vec::new();

    for example_node in find_all_elements(node, "example") {
        let pre_text = collect_text(example_node);
        let parsed = parse_maxima_examples(&pre_text);
        examples.extend(parsed);

        // Limit total examples
        if examples.len() >= 3 {
            examples.truncate(3);
            break;
        }
    }

    examples
}

/// Parse Maxima example text into structured examples.
///
/// Maxima examples follow the pattern:
/// ```
/// (%i1) diff(x^3, x);
/// (%o1)                           3 x^2
/// ```
fn parse_maxima_examples(text: &str) -> Vec<FunctionExample> {
    let mut examples = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for input lines: (%iN)
        if let Some(rest) = strip_input_marker(line) {
            let input = rest.trim().trim_end_matches(';').trim().to_string();

            if !input.is_empty() {
                // Look ahead for output line: (%oN)
                let mut description = None;
                if i + 1 < lines.len() {
                    let next = lines[i + 1].trim();
                    if let Some(output) = strip_output_marker(next) {
                        let out = output.trim().to_string();
                        if !out.is_empty() {
                            description = Some(out);
                        }
                        i += 1;
                    }
                }

                examples.push(FunctionExample { input, description });
            }
        }

        i += 1;
    }

    examples
}

/// Strip `(%iN) ` prefix from a line, returning the rest.
fn strip_input_marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with("(%i") {
        if let Some(end) = line.find(')') {
            return Some(&line[end + 1..]);
        }
    }
    None
}

/// Strip `(%oN) ` prefix from a line, returning the rest.
fn strip_output_marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if line.starts_with("(%o") {
        if let Some(end) = line.find(')') {
            return Some(&line[end + 1..]);
        }
    }
    None
}

/// Extract the function category from the definition.
///
/// Strategy:
/// 1. Try `<defcategory>` — but this usually contains a definition type ("Function"),
///    not a topical category. If it maps to a real topic, use it.
/// 2. Try embedded HTML-style category spans in the body.
/// 3. Fall back to the chapter title (e.g., "Differentiation" → Calculus).
fn extract_category(node: Node, tag: &str, chapter: &str, log_unmapped: bool) -> FunctionCategory {
    // Try to find <defcategory> element
    for child in node.children() {
        if child.is_element() && child.tag_name().name() == "definitionterm" {
            if let Some(cat_node) = find_element(child, "defcategory") {
                let cat_text = collect_text(cat_node).trim().to_string();
                if !cat_text.is_empty() {
                    let mapped = map_category(&cat_text, false);
                    if mapped != FunctionCategory::Other {
                        return mapped;
                    }
                    // Definition type (e.g., "Function") — fall through to chapter
                }
            }
        }
    }

    // Try embedded HTML-style category spans in the body
    if let Some(item) = find_element(node, "definitionitem") {
        let body_text = collect_text(item);
        if let Some(cat) = extract_category_from_html_span(&body_text) {
            return map_category(&cat, log_unmapped);
        }
    }

    // Fall back to chapter title
    if !chapter.is_empty() {
        let mapped = map_category(chapter, log_unmapped);
        if mapped != FunctionCategory::Other {
            return mapped;
        }
    }

    let _ = tag;
    FunctionCategory::Other
}

/// Try to extract category from HTML span embedded in text (Maxima sometimes does this).
fn extract_category_from_html_span(text: &str) -> Option<String> {
    // Look for patterns like <span class="category">Name</span>
    let marker = "class=\"category\">";
    if let Some(start) = text.find(marker) {
        let rest = &text[start + marker.len()..];
        if let Some(end) = rest.find("</span>") {
            let cat = rest[..end].trim().to_string();
            if !cat.is_empty() {
                return Some(cat);
            }
        }
    }
    None
}

/// Extract see-also references from the definition body.
fn extract_see_also(node: Node) -> Vec<String> {
    let mut refs = Vec::new();

    if let Some(item) = find_element(node, "definitionitem") {
        collect_refs(item, &mut refs);
    }

    // Deduplicate
    refs.sort();
    refs.dedup();

    refs
}

fn collect_refs(node: Node, refs: &mut Vec<String>) {
    for child in node.children() {
        if child.is_element() {
            let tag = child.tag_name().name();
            match tag {
                // Cross-references from @mref{}, @ref{}, @xref{}
                "ref" | "xref" => {
                    if let Some(label) = child.attribute("label") {
                        let name = label.trim().to_string();
                        if !name.is_empty() && is_likely_function_name(&name) {
                            refs.push(name);
                        }
                    }
                }
                _ => {
                    collect_refs(child, refs);
                }
            }
        }
    }
}

/// Heuristic: function names are typically alphanumeric, may start with % or _.
fn is_likely_function_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 50 {
        return false;
    }
    let first = name.chars().next().unwrap();
    (first.is_alphabetic() || first == '%' || first == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '%')
}

// --- XML pre-processing ---

/// Replace Texinfo-specific XML entity references that roxmltree doesn't know about.
/// Standard entities (&amp; &lt; &gt; &quot;) are kept as-is.
fn replace_texinfo_entities(xml: &str) -> String {
    xml.replace("&arobase;", "@")
        .replace("&bullet;", "*")
        .replace("&comma;", ",")
        .replace("&dots;", "...")
        .replace("&euro;", "EUR")
        .replace("&hyphenbreak;", "-")
        .replace("&lbrace;", "{")
        .replace("&linebreak;", "\n")
        .replace("&pound;", "GBP")
        .replace("&rarr;", "->")
        .replace("&rbrace;", "}")
        .replace("&szlig;", "ss")
        .replace("&tex;", "TeX")
        .replace("&textldquo;", "\"")
        .replace("&textlsquo;", "'")
        .replace("&textmdash;", "---")
        .replace("&textndash;", "--")
        .replace("&textrdquo;", "\"")
        .replace("&textrsquo;", "'")
}

// --- XML utility helpers ---

/// Recursively collect all text content from a node, stripping markup.
fn collect_text(node: Node) -> String {
    let mut text = String::new();
    for child in node.children() {
        if child.is_text() {
            text.push_str(child.text().unwrap_or(""));
        } else if child.is_element() {
            text.push_str(&collect_text(child));
        }
    }
    text
}

/// Find the first descendant element with the given tag name.
fn find_element<'a>(node: Node<'a, 'a>, tag: &str) -> Option<Node<'a, 'a>> {
    for child in node.children() {
        if child.is_element() {
            if child.tag_name().name() == tag {
                return Some(child);
            }
            if let Some(found) = find_element(child, tag) {
                return Some(found);
            }
        }
    }
    None
}

/// Find all descendant elements with the given tag name.
fn find_all_elements<'a>(node: Node<'a, 'a>, tag: &str) -> Vec<Node<'a, 'a>> {
    let mut results = Vec::new();
    collect_elements(node, tag, &mut results);
    results
}

fn collect_elements<'a>(node: Node<'a, 'a>, tag: &str, results: &mut Vec<Node<'a, 'a>>) {
    for child in node.children() {
        if child.is_element() {
            if child.tag_name().name() == tag {
                results.push(child);
            }
            collect_elements(child, tag, results);
        }
    }
}

/// Normalize whitespace: collapse multiple spaces/newlines into single spaces.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Clean up residual Texinfo markup in description text.
///
/// The XML sometimes embeds `<html>` elements containing raw Texinfo like
/// `@math{\sin\left(x\right)^2}` or `@displaymath ... @end displaymath`.
/// This preserves the LaTeX content wrapped in `$...$` (inline) or `$$...$$`
/// (display) delimiters for rendering with KaTeX in the frontend.
fn clean_texinfo_markup(s: &str) -> String {
    let mut result = s.to_string();

    // @math{...} → $...$  (inline math)
    while let Some(start) = result.find("@math{") {
        let content_start = start + 6;
        if let Some(end) = find_matching_brace(&result, content_start) {
            let latex = result[content_start..end].trim();
            result = format!("{}${}${}", &result[..start], latex, &result[end + 1..]);
        } else {
            break;
        }
    }

    // @displaymath ... @end displaymath → $$...$$  (display math)
    while let Some(start) = result.find("@displaymath") {
        let content_start = start + "@displaymath".len();
        if let Some(end_pos) = result[content_start..].find("@end displaymath") {
            let latex = result[content_start..content_start + end_pos].trim();
            let after = content_start + end_pos + "@end displaymath".len();
            result = format!("{}$${}$${}", &result[..start], latex, &result[after..]);
        } else {
            break;
        }
    }

    // Strip any remaining @command{content} patterns — keep content
    loop {
        if let Some(at_pos) = result.find('@') {
            let rest = &result[at_pos + 1..];
            // Check if it's @word{...}
            let cmd_len = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
            if cmd_len > 0 && rest[cmd_len..].starts_with('{') {
                let content_start = at_pos + 1 + cmd_len + 1;
                if let Some(end) = find_matching_brace(&result, content_start) {
                    let content = result[content_start..end].to_string();
                    result = format!("{}{}{}", &result[..at_pos], content, &result[end + 1..]);
                    continue;
                }
            }
            // Not a @cmd{} pattern — skip past this @
            if at_pos + 1 < result.len() {
                // Skip @@ (escaped @)
                if result[at_pos + 1..].starts_with('@') {
                    result = format!("{}@{}", &result[..at_pos], &result[at_pos + 2..]);
                    continue;
                }
            }
            break;
        } else {
            break;
        }
    }

    // Clean up extra whitespace introduced by removals
    normalize_whitespace(&result)
}

/// Find the position of the closing brace matching an opening brace.
/// `start` should point to the character after the opening `{`.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_input_marker() {
        assert_eq!(
            strip_input_marker("(%i1) diff(x^3, x);"),
            Some(" diff(x^3, x);")
        );
        assert_eq!(strip_input_marker("(%i12) foo();"), Some(" foo();"));
        assert_eq!(strip_input_marker("no marker"), None);
    }

    #[test]
    fn test_strip_output_marker() {
        assert_eq!(
            strip_output_marker("(%o1)                           3 x^2"),
            Some("                           3 x^2")
        );
        assert_eq!(strip_output_marker("no marker"), None);
    }

    #[test]
    fn test_parse_maxima_examples() {
        let text = "(%i1) diff(x^3, x);\n(%o1)                           3 x^2\n(%i2) integrate(x, x);\n(%o2)                           x^2/2\n";
        let examples = parse_maxima_examples(text);
        assert_eq!(examples.len(), 2);
        assert_eq!(examples[0].input, "diff(x^3, x)");
        assert_eq!(examples[0].description.as_deref(), Some("3 x^2"));
        assert_eq!(examples[1].input, "integrate(x, x)");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(
            normalize_whitespace("  hello   world\n  foo  "),
            "hello world foo"
        );
    }

    #[test]
    fn test_clean_texinfo_markup_preserves_latex() {
        assert_eq!(
            clean_texinfo_markup(
                r"uses @math{\sin\left(x\right)^2 + \cos\left(x\right)^2 = 1} to simplify"
            ),
            r"uses $\sin\left(x\right)^2 + \cos\left(x\right)^2 = 1$ to simplify"
        );
        assert_eq!(
            clean_texinfo_markup(r"computes @math{\frac{a}{b}}"),
            r"computes $\frac{a}{b}$"
        );
    }

    #[test]
    fn test_clean_texinfo_markup_display_math() {
        assert_eq!(
            clean_texinfo_markup(r"defined by @displaymath x^2 + y^2 @end displaymath for all x"),
            r"defined by $$x^2 + y^2$$ for all x"
        );
    }

    #[test]
    fn test_is_likely_function_name() {
        assert!(is_likely_function_name("diff"));
        assert!(is_likely_function_name("integrate"));
        assert!(is_likely_function_name("%pi"));
        assert!(is_likely_function_name("_internal"));
        assert!(!is_likely_function_name(""));
        assert!(!is_likely_function_name("has spaces"));
        assert!(!is_likely_function_name("123"));
    }
}
