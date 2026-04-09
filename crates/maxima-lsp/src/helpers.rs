/// Convert an LSP position (0-based line, UTF-16 character) to a byte offset in the content.
pub fn position_to_offset(content: &str, line: u32, character: u32) -> Option<usize> {
    let mut current_line = 0u32;
    let mut byte_offset = 0usize;

    for (i, ch) in content.char_indices() {
        if current_line == line {
            // Count UTF-16 code units from the start of this line
            let line_start = byte_offset;
            let mut utf16_offset = 0u32;
            for (j, c) in content[line_start..].char_indices() {
                if utf16_offset >= character {
                    return Some(line_start + j);
                }
                if c == '\n' {
                    // Cursor is at end of line
                    return Some(line_start + j);
                }
                utf16_offset += if (c as u32) > 0xFFFF { 2 } else { 1 };
            }
            // Past end of line/file — return end of content
            return Some(content.len());
        }
        if ch == '\n' {
            current_line += 1;
        }
        byte_offset = i + ch.len_utf8();
    }

    // If requested line is at/past end of content
    if current_line == line {
        Some(content.len())
    } else {
        None
    }
}

/// Check if a character can start a Maxima identifier.
fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '%' || c == '?'
}

/// Check if a character can continue a Maxima identifier.
fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '%'
}

/// Extract the Maxima identifier at the given LSP position.
pub fn word_at_position(content: &str, line: u32, character: u32) -> Option<String> {
    let offset = position_to_offset(content, line, character)?;
    // Scan backwards to find start of identifier
    let mut start = offset;
    while start > 0 {
        let prev = content[..start]
            .chars()
            .next_back()?;
        if is_ident_continue(prev) || (start == offset && is_ident_start(prev)) {
            start -= prev.len_utf8();
        } else {
            break;
        }
    }

    // Scan forwards to find end of identifier
    let mut end = offset;
    for ch in content[offset..].chars() {
        if is_ident_continue(ch) || (end == start && is_ident_start(ch)) {
            end += ch.len_utf8();
        } else {
            break;
        }
    }

    if start == end {
        return None;
    }

    Some(content[start..end].to_string())
}

/// Find the enclosing function call at the given position.
/// Returns `(function_name, active_param_index)` where active_param_index
/// is the 0-based index of the parameter the cursor is currently in.
pub fn find_enclosing_call(
    content: &str,
    line: u32,
    character: u32,
) -> Option<(String, usize)> {
    let offset = position_to_offset(content, line, character)?;
    let before = &content[..offset];

    let mut paren_depth: i32 = 0;
    let mut bracket_depth: i32 = 0;
    let mut comma_count: usize = 0;
    let mut in_string = false;
    let mut in_comment = false;

    // Walk backwards through the content
    let chars: Vec<char> = before.chars().collect();
    let mut i = chars.len();

    while i > 0 {
        i -= 1;
        let ch = chars[i];

        // Simple string tracking (backwards)
        if ch == '"' && !in_comment {
            // Check if escaped
            let mut backslashes = 0;
            let mut j = i;
            while j > 0 && chars[j - 1] == '\\' {
                backslashes += 1;
                j -= 1;
            }
            if backslashes % 2 == 0 {
                in_string = !in_string;
            }
            continue;
        }
        if in_string {
            continue;
        }

        // Simple comment tracking (backwards: detect */ then skip to /*)
        if ch == '/' && i > 0 && chars[i - 1] == '*' {
            in_comment = true;
            i -= 1; // skip the *
            continue;
        }
        if in_comment {
            if ch == '*' && i > 0 && chars[i - 1] == '/' {
                in_comment = false;
                i -= 1; // skip the /
            }
            continue;
        }

        match ch {
            ')' => paren_depth += 1,
            '(' => {
                if paren_depth == 0 {
                    // Found the unmatched opening paren — extract the function name
                    let before_paren = &chars[..i];
                    // Skip whitespace before the paren
                    let name_end = before_paren.len();
                    let mut name_start = name_end;
                    // First skip any trailing whitespace
                    while name_start > 0 && chars[name_start - 1].is_ascii_whitespace() {
                        name_start -= 1;
                    }
                    let adjusted_end = name_start;
                    // Now collect the identifier
                    while name_start > 0 && is_ident_continue(chars[name_start - 1]) {
                        name_start -= 1;
                    }
                    if name_start < adjusted_end {
                        let name: String = chars[name_start..adjusted_end].iter().collect();
                        if is_ident_start(name.chars().next().unwrap_or(' ')) {
                            return Some((name, comma_count));
                        }
                    }
                    return None;
                }
                paren_depth -= 1;
            }
            ']' => bracket_depth += 1,
            '[' => {
                if bracket_depth > 0 {
                    bracket_depth -= 1;
                }
            }
            ',' => {
                if paren_depth == 0 && bracket_depth == 0 {
                    comma_count += 1;
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
    fn position_to_offset_basic() {
        let content = "ab\ncd\nef";
        assert_eq!(position_to_offset(content, 0, 0), Some(0));
        assert_eq!(position_to_offset(content, 0, 1), Some(1));
        assert_eq!(position_to_offset(content, 1, 0), Some(3));
        assert_eq!(position_to_offset(content, 1, 1), Some(4));
        assert_eq!(position_to_offset(content, 2, 0), Some(6));
    }

    #[test]
    fn word_at_position_ident() {
        assert_eq!(
            word_at_position("integrate(x, y)", 0, 3),
            Some("integrate".to_string())
        );
    }

    #[test]
    fn word_at_position_percent() {
        assert_eq!(
            word_at_position("%pi + 1", 0, 1),
            Some("%pi".to_string())
        );
    }

    #[test]
    fn word_at_position_none_on_operator() {
        assert_eq!(word_at_position("a + b", 0, 2), None);
    }

    #[test]
    fn find_enclosing_call_simple() {
        // integrate(x, |)  — cursor after comma
        let content = "integrate(x, )";
        let result = find_enclosing_call(content, 0, 13);
        assert_eq!(result, Some(("integrate".to_string(), 1)));
    }

    #[test]
    fn find_enclosing_call_first_param() {
        // integrate(|)
        let content = "integrate()";
        let result = find_enclosing_call(content, 0, 10);
        assert_eq!(result, Some(("integrate".to_string(), 0)));
    }

    #[test]
    fn find_enclosing_call_nested() {
        // f(g(x), |)
        let content = "f(g(x), )";
        let result = find_enclosing_call(content, 0, 8);
        assert_eq!(result, Some(("f".to_string(), 1)));
    }

    #[test]
    fn find_enclosing_call_none_outside() {
        let content = "x + y";
        let result = find_enclosing_call(content, 0, 2);
        assert_eq!(result, None);
    }
}
