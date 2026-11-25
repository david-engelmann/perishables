//! A small reader for the YAML subset OKF frontmatter uses.
//!
//! The core crate is standard library only, so frontmatter is
//! parsed here rather than by an external YAML crate. The subset
//! covers what OKF bundles carry in practice, including plain and
//! quoted scalars, YAML `#` comments, flow maps such as
//! `generated: {by: x, at: y}`, flow lists, block maps, and block
//! lists of scalars or maps. Indentation is taken from the lines
//! themselves, so two-space and four-space styles both parse.
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
    /// counts as a one-item list, matching the way OKF allows a
    /// single `verified` or `sources` entry without list syntax.
    pub fn as_items(&self) -> Vec<&Value> {
        match self {
            Value::List(items) => items.iter().collect(),
            other => vec![other],
        }
    }
}

/// Strip an unquoted YAML `#` comment from a single line.
///
/// Per YAML 1.2, `#` starts a comment only outside single or double
/// quotes and only at the start of the line or after whitespace, so
/// URL fragments such as `https://example.com/doc#section` remain
/// intact.
fn strip_yaml_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut prev_ws = true;
    for (idx, ch) in line.char_indices() {
        match quote {
            Some('"') => {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quote = None;
                }
            }
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                }
            }
            Some(_) => {}
            None => match ch {
                '"' | '\'' => {
                    quote = Some(ch);
                    prev_ws = false;
                }
                '#' if prev_ws => {
                    return line[..idx].trim_end();
                }
                c => {
                    prev_ws = c == ' ' || c == '\t';
                }
            },
        }
    }
    line
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
        block.push(strip_yaml_comment(line));
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
            return if line == "---" || line == "..." {
                ""
            } else {
                text
            };
        };
        let line = rest[..end].trim();
        if line == "---" || line == "..." {
            return &rest[end + 1..];
        }
        rest = &rest[end + 1..];
    }
}

/// Detect a fenced code block opener (` ``` ` or `~~~`) and return
/// `(fence_char, fence_len, fence_indent, info_string)`.
fn fence_opener(line: &str) -> Option<(char, usize, usize, &str)> {
    let fence_indent = line.bytes().take_while(|&b| b == b' ').count();
    let trimmed = line[fence_indent..].trim_start();
    let first = trimmed.chars().next()?;
    if first != '`' && first != '~' {
        return None;
    }
    let count = trimmed.bytes().take_while(|b| *b == first as u8).count();
    if count < 3 {
        return None;
    }
    let info = &trimmed[count..];
    if first == '`' && info.contains('`') {
        return None;
    }
    Some((first, count, fence_indent, info.trim()))
}

/// Strip up to `max_spaces` leading `' '` bytes per CommonMark 4.5.
fn strip_up_to_spaces(line: &str, max_spaces: usize) -> &str {
    let n = line
        .bytes()
        .take(max_spaces)
        .take_while(|&b| b == b' ')
        .count();
    &line[n..]
}

/// Whether `line` closes an open fence of `(fence_char, min_len)`.
fn is_fence_closer(line: &str, fence_char: char, min_len: usize) -> bool {
    let trimmed = line.trim_start();
    let count = trimmed
        .bytes()
        .take_while(|b| *b == fence_char as u8)
        .count();
    count >= min_len && trimmed[count..].trim().is_empty()
}

/// Strip inline backtick code spans from a single prose line so code
/// spans such as `` `[^0-9]` `` are never mistaken for footnote
/// citations.
fn strip_inline_code(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let mut ticks = 0;
            while i + ticks < bytes.len() && bytes[i + ticks] == b'`' {
                ticks += 1;
            }
            let mut scan = i + ticks;
            let mut closed = None;
            while scan < bytes.len() {
                if bytes[scan] == b'`' {
                    let mut run = 0;
                    while scan + run < bytes.len() && bytes[scan + run] == b'`' {
                        run += 1;
                    }
                    if run == ticks {
                        closed = Some(scan + run);
                        break;
                    }
                    scan += run;
                } else {
                    scan += 1;
                }
            }
            if let Some(after) = closed {
                out.push(' ');
                i = after;
                continue;
            }
            for _ in 0..ticks {
                out.push('`');
            }
            i += ticks;
        } else {
            let ch = line[i..].chars().next().unwrap_or('\0');
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Whether a line is a Markdown footnote definition (`[^label]: ...`).
fn is_footnote_definition_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(after) = trimmed.strip_prefix("[^") else {
        return false;
    };
    let Some(end) = after.find(']') else {
        return false;
    };
    let label = &after[..end];
    !label.is_empty()
        && !label.chars().any(char::is_whitespace)
        && after[end + 1..].starts_with(':')
}

/// The footnote labels a body cites, as in `[^audit-memo]`, in
/// first-appearance order without duplicates. Fenced code blocks,
/// inline code spans, and footnote definition lines (`[^label]:`)
/// are skipped so regex character classes like `[^0-9]` in code
/// never trigger false citation findings.
pub fn citation_labels(body: &str) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    let mut active_fence: Option<(char, usize)> = None;
    for line in body.lines() {
        if let Some((fc, flen)) = active_fence {
            if is_fence_closer(line, fc, flen) {
                active_fence = None;
            }
            continue;
        }
        if let Some((fc, flen, _, _)) = fence_opener(line) {
            active_fence = Some((fc, flen));
            continue;
        }
        if is_footnote_definition_line(line) {
            continue;
        }
        let cleaned = strip_inline_code(line);
        let mut rest = cleaned.as_str();
        while let Some(start) = rest.find("[^") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(']') else {
                break;
            };
            let candidate = &after[..end];
            if !candidate.is_empty()
                && !candidate.chars().any(char::is_whitespace)
                && !candidate.contains('[')
                && !after[end + 1..].starts_with(':')
            {
                let owned = candidate.to_string();
                if !labels.contains(&owned) {
                    labels.push(owned);
                }
            }
            rest = &after[end + 1..];
        }
    }
    labels
}

/// The fenced code blocks of a body, as `(language, code)` pairs.
/// Info-string attributes after the language token (for example
/// ```` ```python title="x" ````) are stripped from the language id.
pub fn code_blocks(body: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut current: Option<(char, usize, usize, String, String)> = None;
    for line in body.lines() {
        match &mut current {
            None => {
                if let Some((fc, flen, findent, info)) = fence_opener(line) {
                    let lang = info
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    current = Some((fc, flen, findent, lang, String::new()));
                }
            }
            Some((fc, flen, findent, _, code)) => {
                if is_fence_closer(line, *fc, *flen) {
                    if let Some((_, _, _, lang, code)) = current.take() {
                        blocks.push((lang, code));
                    }
                } else {
                    code.push_str(strip_up_to_spaces(line, *findent));
                    code.push('\n');
                }
            }
        }
    }
    blocks
}

/// UTF-8-safe ASCII case-insensitive substring search.
///
/// Callers pass pure-ASCII needles, so any byte-window match lies on
/// single-byte ASCII code points and is guaranteed to be a valid
/// UTF-8 character boundary in `haystack`.
fn find_ascii_ci(haystack: &str, needle: &str) -> Option<usize> {
    debug_assert!(needle.is_ascii());
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// The attested result a concept body claims, under the v1 corpus
/// profile convention. The claim lives in a `Claimed result` section
/// and is the first value stated after `claimed ... is `, as in
/// `"The attested result claimed for this computation is 42."` A
/// body without such a statement claims nothing.
pub fn claimed_result(body: &str) -> Option<String> {
    let section = find_ascii_ci(body, "claimed result")?;
    let after = &body[section..];
    let claimed = find_ascii_ci(after, "claimed")?;
    let rest = &after[claimed + "claimed".len()..];
    let is = find_ascii_ci(rest, " is ")?;
    let mut chars = rest[is + 4..]
        .chars()
        .skip_while(|c| c.is_whitespace() || *c == '"' || *c == '`')
        .peekable();
    let mut token = String::new();
    while let Some(c) = chars.next() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            token.push(c);
        } else if c == '.' && chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            // A dot belongs to the value only as a decimal point,
            // never as the full stop that ends the sentence.
            token.push(c);
        } else {
            break;
        }
    }
    let token = token.trim_end_matches('.').to_string();
    if token.is_empty() { None } else { Some(token) }
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    if text.len() >= 2 {
        let bytes = text.as_bytes();
        if bytes[0] == b'"' && bytes[text.len() - 1] == b'"' {
            let inner = &text[1..text.len() - 1];
            let mut out = String::with_capacity(inner.len());
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => out.push('\n'),
                        Some('r') => out.push('\r'),
                        Some('t') => out.push('\t'),
                        Some('"') => out.push('"'),
                        Some('\\') => out.push('\\'),
                        Some(other) => {
                            out.push('\\');
                            out.push(other);
                        }
                        None => out.push('\\'),
                    }
                } else {
                    out.push(c);
                }
            }
            return out;
        }
        if bytes[0] == b'\'' && bytes[text.len() - 1] == b'\'' {
            return text[1..text.len() - 1].replace("''", "'");
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

/// Split a flow collection body on top-level commas, respecting
/// quotes, backslash escapes, and nested brackets.
fn split_flow(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut current = String::new();
    for ch in body.chars() {
        match quote {
            Some(q) => {
                current.push(ch);
                if q == '"' && escaped {
                    escaped = false;
                } else if q == '"' && ch == '\\' {
                    escaped = true;
                } else if ch == q {
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
            let mut peek = i + 1;
            while peek < lines.len() && lines[peek].trim().is_empty() {
                peek += 1;
            }
            let floor = if peek < lines.len()
                && (lines[peek].trim_start() == "-" || lines[peek].trim_start().starts_with("- "))
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
    fn yaml_comments_are_stripped_without_harming_urls_or_quotes() {
        let text = "---\n# leading comment\ntype: Metric # inline comment\nstatus: stable # another\nsources:\n  # indented comment\n  - id: \"hash#1\" # comment\n    resource: https://example.com/doc#section\n---\n";
        let fm = parse_frontmatter(text).expect("frontmatter");
        assert_eq!(fm.get("type").and_then(Value::as_str), Some("Metric"));
        assert_eq!(fm.get("status").and_then(Value::as_str), Some("stable"));
        let items = fm.get("sources").expect("sources").as_items();
        assert_eq!(items[0].get("id").and_then(Value::as_str), Some("hash#1"));
        assert_eq!(
            items[0].get("resource").and_then(Value::as_str),
            Some("https://example.com/doc#section")
        );
    }

    #[test]
    fn citations_skip_fenced_and_inline_code_and_definitions() {
        let body = "Prose cites [^real-ref] and inline `[^0-9]`.\n\n```python title=\"demo\"\nx = re.sub(r\"[^a-z]\", \"\", s)\n```\n\n~~~\n[^tilde-code]\n~~~\n\n[^real-ref]: https://example.com\n[^only-def]: Footnote text\n";
        assert_eq!(citation_labels(body), vec!["real-ref".to_string()]);
        let blocks = code_blocks(body);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].0, "python");
    }

    #[test]
    fn indented_fenced_code_blocks_strip_fence_indent() {
        let body = "- Step 1\n  ```python\n  x = 6\n  if x > 0:\n      result = x * 7\n  ```\n";
        let blocks = code_blocks(body);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, "python");
        assert_eq!(blocks[0].1, "x = 6\nif x > 0:\n    result = x * 7\n");
    }

    #[test]
    fn claimed_result_handles_multibyte_unicode_without_panic() {
        let body = "# İstanbul Report 🦀\n\n## Claimed result\n\nThe attested result claimed for this computation is 17.5.\n";
        assert_eq!(claimed_result(body), Some("17.5".to_string()));
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
