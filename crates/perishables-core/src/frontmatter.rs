//! A small reader for the YAML subset OKF frontmatter uses.
//!
//! The core crate is standard library only, so frontmatter is
//! parsed here rather than by a YAML crate. The subset covers what
//! real bundles carry: plain and quoted scalars, flow maps such as
//! `generated: {by: x, at: y}`, flow lists, block maps, and block
//! lists of scalars or maps. Indentation is taken from the lines
//! themselves, so two space and four space styles both parse.
//! Anything outside the subset parses as a scalar string, which
//! keeps odd values visible to the checks instead of dropping them.

/// A parsed frontmatter value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// A scalar, quotes stripped.
    Str(String),
    /// A mapping, in document order.
    Map(Vec<(String, Value)>),
    /// A list.
    List(Vec<Value>),
}

impl Value {
    /// Look up a key in a mapping value.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The scalar text of this value, when it is a scalar.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }

    /// The items of this value as a list. A lone map or scalar
    /// counts as a one item list, matching the way OKF allows a
    /// single `verified` or `sources` entry without list syntax.
    pub fn as_items(&self) -> Vec<&Value> {
        match self {
            Value::List(items) => items.iter().collect(),
            other => vec![other],
        }
    }
}

/// Extract and parse the frontmatter block of a Markdown document.
/// Returns `None` when the document has no frontmatter block.
pub fn parse_frontmatter(text: &str) -> Option<Value> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut block: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim() == "---" || line.trim() == "..." {
            let (value, _) = parse_block(&block, 0, 0);
            return Some(value);
        }
        block.push(line);
    }
    None
}

/// The body of a Markdown document, meaning everything after the
/// frontmatter block. Returns the whole text when there is no
/// frontmatter block.
pub fn body_of(text: &str) -> &str {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some(first_end) = text.find('\n') else {
        return text;
    };
    if text[..first_end].trim() != "---" {
        return text;
    }
    let mut rest = &text[first_end + 1..];
    loop {
        let Some(end) = rest.find('\n') else {
            let line = rest.trim();
            return if line == "---" || line == "..." { "" } else { text };
        };
        let line = rest[..end].trim();
        if line == "---" || line == "..." {
            return &rest[end + 1..];
        }
        rest = &rest[end + 1..];
    }
}

/// The footnote labels a body cites, as in `[^audit-memo]`, in
/// first appearance order without duplicates. A label counts when
/// it appears as a reference or as a footnote definition, because
/// both claim an attribution.
pub fn citation_labels(body: &str) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("[^") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(']') else {
            break;
        };
        let label = after[..end].trim().to_string();
        if !label.is_empty() && !labels.contains(&label) {
            labels.push(label);
        }
        rest = &after[end + 1..];
    }
    labels
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if (bytes[0] == b'"' && bytes[text.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[text.len() - 1] == b'\'')
        {
            return text[1..text.len() - 1].to_string();
        }
    }
    text.to_string()
}

/// Whether a list item body reads as a mapping entry. A mapping
/// entry is a simple key, then a colon, then a space or the end of
/// the line. A bare URL fails the test because its colon is
/// followed by slashes, not a space, so URLs stay scalars.
fn looks_like_mapping_entry(text: &str) -> bool {
    match text.split_once(':') {
        Some((key, rest)) => {
            !key.is_empty()
                && !key.contains(char::is_whitespace)
                && !key.contains('/')
                && (rest.is_empty() || rest.starts_with(' '))
        }
        None => false,
    }
}

/// Split a flow collection body on top level commas, respecting
/// quotes and nested brackets.
fn split_flow(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut current = String::new();
    for ch in body.chars() {
        match quote {
            Some(q) => {
                current.push(ch);
                if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                '{' | '[' => {
                    depth += 1;
                    current.push(ch);
                }
                '}' | ']' => {
                    depth -= 1;
                    current.push(ch);
                }
                ',' if depth == 0 => {
                    parts.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(ch),
            },
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_inline(text: &str) -> Value {
    let text = text.trim();
    if let Some(body) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')) {
        let mut entries = Vec::new();
        for part in split_flow(body) {
            if let Some((key, value)) = part.split_once(':') {
                entries.push((unquote(key), parse_inline(value)));
            }
        }
        return Value::Map(entries);
    }
    if let Some(body) = text.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
        return Value::List(split_flow(body).iter().map(|p| parse_inline(p)).collect());
    }
    Value::Str(unquote(text))
}

/// Parse a block starting at or after `start`, whose entries sit
/// deeper than `min_indent`. The block indent is adopted from its
/// first content line, so any consistent indent style parses.
/// Returns the value and the index of the first line not consumed.
fn parse_block(lines: &[&str], start: usize, min_indent: usize) -> (Value, usize) {
    let mut i = start;
    while i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    if i >= lines.len() || indent_of(lines[i]) < min_indent {
        return (Value::Map(Vec::new()), start);
    }
    let indent = indent_of(lines[i]);
    let first = lines[i].trim_start();
    if first == "-" || first.starts_with("- ") {
        let mut items = Vec::new();
        while i < lines.len() {
            let line = lines[i];
            if line.trim().is_empty() {
                i += 1;
                continue;
            }
            let trimmed = line.trim_start();
            if indent_of(line) != indent || !(trimmed == "-" || trimmed.starts_with("- ")) {
                break;
            }
            let after = trimmed.strip_prefix('-').unwrap_or("").trim_start();
            // The item body lives at the content column, which is
            // the dash indent plus two. Continuation lines are the
            // following lines indented past the dash.
            let content_col = indent + 2;
            let mut synthetic: Vec<String> = Vec::new();
            if !after.is_empty() {
                synthetic.push(format!("{}{}", " ".repeat(content_col), after));
            }
            let mut j = i + 1;
            while j < lines.len() {
                let next = lines[j];
                if next.trim().is_empty() {
                    j += 1;
                    continue;
                }
                if indent_of(next) > indent {
                    synthetic.push(next.to_string());
                    j += 1;
                } else {
                    break;
                }
            }
            if after.starts_with('{') || after.starts_with('[') {
                items.push(parse_inline(after));
            } else if after.is_empty() || looks_like_mapping_entry(after) {
                let refs: Vec<&str> = synthetic.iter().map(String::as_str).collect();
                let (value, _) = parse_block(&refs, 0, content_col);
                items.push(value);
            } else {
                items.push(Value::Str(unquote(after)));
            }
            i = j;
        }
        return (Value::List(items), i);
    }
    let mut entries: Vec<(String, Value)> = Vec::new();
    while i < lines.len() {
        let line = lines[i];
        if line.trim().is_empty() {
            i += 1;
            continue;
        }
        if indent_of(line) != indent {
            break;
        }
        let Some((key, rest)) = line.trim().split_once(':') else {
            break;
        };
        let key = unquote(key);
        let rest = rest.trim();
        if rest.is_empty() {
            // A nested block sits deeper than its key, except a
            // block list, whose dashes may sit at the key indent.
            // Peek at the next content line to pick the floor, so a
            // childless key never swallows its siblings.
            let mut peek = i + 1;
            while peek < lines.len() && lines[peek].trim().is_empty() {
                peek += 1;
            }
            let floor = if peek < lines.len()
                && lines[peek].trim_start().starts_with("- ")
                && indent_of(lines[peek]) >= indent
            {
                indent_of(lines[peek])
            } else {
                indent + 1
            };
            let (value, next) = parse_block(lines, i + 1, floor);
            entries.push((key, value));
            i = next;
        } else {
            entries.push((key, parse_inline(rest)));
            i += 1;
        }
    }
    (Value::Map(entries), i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_and_block_forms() {
        let text = "---\ntype: metric\nverified:\n  - {by: \"human:a\", at: \"2026-07-01\"}\ngenerated:\n  by: reference_agent/test\n  at: \"2026-06-30T15:30:49Z\"\n---\nbody\n";
        let fm = parse_frontmatter(text).expect("frontmatter");
        assert_eq!(fm.get("type").and_then(Value::as_str), Some("metric"));
        let verified = fm.get("verified").expect("verified");
        let items = verified.as_items();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("by").and_then(Value::as_str), Some("human:a"));
        let generated = fm.get("generated").expect("generated");
        assert_eq!(
            generated.get("by").and_then(Value::as_str),
            Some("reference_agent/test")
        );
    }

    #[test]
    fn block_list_of_maps() {
        let text = "---\ntype: metric\nsources:\n  - id: memos/launch\n    resource: \"/sources/memo.md\"\n  - id: other\n    resource: \"https://example.com/x\"\n---\n";
        let fm = parse_frontmatter(text).expect("frontmatter");
        let items = fm.get("sources").expect("sources").as_items();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get("resource").and_then(Value::as_str),
            Some("/sources/memo.md")
        );
        assert_eq!(items[1].get("id").and_then(Value::as_str), Some("other"));
    }

    #[test]
    fn block_list_of_urls_stays_scalar() {
        let text = "---\ntype: metric\nsee_also:\n  - https://example.com/a\n  - https://example.com/b\n---\n";
        let fm = parse_frontmatter(text).expect("frontmatter");
        let items = fm.get("see_also").expect("see_also").as_items();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_str(), Some("https://example.com/a"));
    }

    #[test]
    fn four_space_indent_parses() {
        let text = "---\ntype: metric\ngenerated:\n    by: agent/x\n    at: \"2026-06-30\"\n---\n";
        let fm = parse_frontmatter(text).expect("frontmatter");
        let generated = fm.get("generated").expect("generated");
        assert_eq!(generated.get("by").and_then(Value::as_str), Some("agent/x"));
    }

    #[test]
    fn no_frontmatter() {
        assert!(parse_frontmatter("# Title\nbody\n").is_none());
    }
}
