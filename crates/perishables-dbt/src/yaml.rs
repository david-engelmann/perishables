//! A YAML block structure reader producing the same Json
//! values the JSON reader produces, so both feed one grading
//! model. It covers the subset semantic view YAML uses in
//! practice. Block mappings and sequences by indentation,
//! plain and quoted scalars, flow sequences of scalars, and
//! literal block scalars, which is how verified query SQL is
//! carried. It is not a general YAML implementation, and the
//! boundary is stated here rather than smoothed over. Anchors,
//! tags, and folded scalars are outside it.

use super::Json;

struct Line {
    indent: usize,
    text: String,
}

pub fn parse(text: &str) -> Result<Json, String> {
    let lines = lex(text);
    if lines.is_empty() {
        return Ok(Json::Map(Vec::new()));
    }
    let mut pos = 0;
    let value = parse_block(&lines, &mut pos, lines[0].indent)?;
    Ok(value)
}

/// Split the source into content lines, dropping blank lines
/// and full line comments, and keeping each line's indent.
fn lex(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        out.push(Line {
            indent,
            text: trimmed.to_string(),
        });
    }
    out
}

fn parse_block(lines: &[Line], pos: &mut usize, indent: usize) -> Result<Json, String> {
    if *pos >= lines.len() {
        return Ok(Json::Null);
    }
    if lines[*pos].text.starts_with("- ") || lines[*pos].text == "-" {
        parse_list(lines, pos, indent)
    } else {
        parse_map(lines, pos, indent)
    }
}

fn parse_list(lines: &[Line], pos: &mut usize, indent: usize) -> Result<Json, String> {
    let mut items = Vec::new();
    while *pos < lines.len() && lines[*pos].indent == indent && is_list_item(&lines[*pos].text) {
        let rest = item_rest(&lines[*pos].text).to_string();
        if rest.is_empty() {
            *pos += 1;
            if *pos < lines.len() && lines[*pos].indent > indent {
                let child_indent = lines[*pos].indent;
                items.push(parse_block(lines, pos, child_indent)?);
            } else {
                items.push(Json::Null);
            }
        } else if is_map_entry(&rest) {
            // A map whose first entry rides on the dash line.
            // Rewrite the line as a map entry at a virtual
            // indent and parse the map from there.
            let virtual_indent = indent + 2;
            let mut synthetic: Vec<Line> = vec![Line {
                indent: virtual_indent,
                text: rest,
            }];
            let mut scan = *pos + 1;
            while scan < lines.len() && lines[scan].indent > indent {
                synthetic.push(Line {
                    indent: lines[scan].indent.max(virtual_indent),
                    text: lines[scan].text.clone(),
                });
                scan += 1;
            }
            let mut sub_pos = 0;
            let value = parse_map(&synthetic, &mut sub_pos, virtual_indent)?;
            items.push(value);
            *pos = scan;
        } else {
            items.push(parse_scalar(&rest)?);
            *pos += 1;
        }
    }
    Ok(Json::List(items))
}

fn parse_map(lines: &[Line], pos: &mut usize, indent: usize) -> Result<Json, String> {
    let mut entries: Vec<(String, Json)> = Vec::new();
    while *pos < lines.len() && lines[*pos].indent == indent && !is_list_item(&lines[*pos].text) {
        let text = lines[*pos].text.clone();
        let Some((key, rest)) = split_entry(&text) else {
            return Err(format!("expected a mapping entry, got {text}"));
        };
        let key = unquote(key.trim());
        if rest.is_empty() {
            *pos += 1;
            if *pos < lines.len() && lines[*pos].indent > indent {
                let child_indent = lines[*pos].indent;
                let value = parse_block(lines, pos, child_indent)?;
                entries.push((key, value));
            } else {
                entries.push((key, Json::Null));
            }
        } else if rest == "|" || rest == "|-" || rest == ">" || rest == ">-" {
            let value = block_scalar(lines, pos, indent, rest.starts_with('>'));
            entries.push((key, Json::Str(value)));
        } else {
            let value = parse_scalar(&rest)?;
            entries.push((key, value));
            *pos += 1;
        }
    }
    Ok(Json::Map(entries))
}

/// Consume a literal or folded block scalar. The lines of the
/// scalar are the following lines more indented than the key.
/// Folding joins lines with spaces, literal keeps newlines.
/// Lexing has already trimmed each line and dropped blank
/// lines, so a block scalar survives in shape rather than
/// byte for byte. This slice carries SQL as a stated fact and
/// never executes it, so the loss is accepted and stated.
fn block_scalar(lines: &[Line], pos: &mut usize, indent: usize, folded: bool) -> String {
    *pos += 1;
    let mut parts = Vec::new();
    while *pos < lines.len() && lines[*pos].indent > indent {
        parts.push(lines[*pos].text.clone());
        *pos += 1;
    }
    if folded {
        parts.join(" ")
    } else {
        parts.join("\n")
    }
}

fn parse_scalar(text: &str) -> Result<Json, String> {
    let text = text.trim();
    if text.starts_with('[') && text.ends_with(']') {
        let inner = &text[1..text.len() - 1];
        if inner.trim().is_empty() {
            return Ok(Json::List(Vec::new()));
        }
        let items = inner
            .split(',')
            .map(|part| Json::Str(unquote(part.trim())))
            .collect();
        return Ok(Json::List(items));
    }
    if text.starts_with('{') && text.ends_with('}') {
        // A one line flow mapping, split on top level commas.
        let inner = &text[1..text.len() - 1];
        let mut entries = Vec::new();
        if !inner.trim().is_empty() {
            for part in inner.split(',') {
                let Some((k, v)) = split_entry(part.trim()) else {
                    return Err(format!("bad flow mapping entry {part}"));
                };
                entries.push((unquote(k.trim()), Json::Str(unquote(v.trim()))));
            }
        }
        return Ok(Json::Map(entries));
    }
    if is_integer(text) {
        return Ok(Json::Number(text.to_string()));
    }
    Ok(Json::Str(unquote(text)))
}

fn is_list_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

fn item_rest(text: &str) -> &str {
    if text == "-" {
        ""
    } else {
        text[2..].trim_start()
    }
}

/// A mapping entry is a key, a colon, then a space or the end
/// of the line. A colon inside a plain scalar, as in a URL or
/// a time, is not followed by a space and does not split.
fn is_map_entry(text: &str) -> bool {
    split_entry(text).is_some()
}

fn split_entry(text: &str) -> Option<(&str, &str)> {
    if let Some(idx) = text.find(": ") {
        Some((&text[..idx], text[idx + 2..].trim_start()))
    } else if let Some(stripped) = text.strip_suffix(':') {
        Some((stripped, ""))
    } else {
        None
    }
}

fn is_integer(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

fn unquote(text: &str) -> String {
    let text = text.trim();
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        text[1..text.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else if text.len() >= 2 && text.starts_with('\'') && text.ends_with('\'') {
        text[1..text.len() - 1].replace("''", "'")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_semantic_view_shape() {
        let text = "name: sales\ntables:\n  - name: orders\n    synonyms: [orders_data, order_facts]\n    facts:\n      - name: revenue\n        expr: amount\nverified_queries:\n  - name: q1\n    verified_at: 1760000000\n    sql: |\n      SELECT 1\n";
        let value = parse(text).expect("parses");
        assert_eq!(value.get("name").and_then(Json::as_str), Some("sales"));
        let tables = value.get("tables").and_then(Json::as_list).expect("tables");
        assert_eq!(tables.len(), 1);
        assert_eq!(
            tables[0].get("synonyms").map(|s| s.as_str_list()),
            Some(vec!["orders_data".to_string(), "order_facts".to_string()])
        );
        let queries = value
            .get("verified_queries")
            .and_then(Json::as_list)
            .expect("queries");
        assert_eq!(
            queries[0].get("verified_at"),
            Some(&Json::Number("1760000000".to_string()))
        );
        assert_eq!(
            queries[0].get("sql").and_then(Json::as_str),
            Some("SELECT 1")
        );
    }

    #[test]
    fn plain_scalars_with_colons_survive() {
        let value = parse("expr: \"a: b\"\nurl: https://example.com/x\n").expect("parses");
        assert_eq!(value.get("expr").and_then(Json::as_str), Some("a: b"));
        assert_eq!(
            value.get("url").and_then(Json::as_str),
            Some("https://example.com/x")
        );
    }
}
