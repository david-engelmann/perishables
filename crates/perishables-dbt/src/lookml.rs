//! The native LookML reader, slice 1 of the project the
//! design record at docs/DESIGN-LOOKML.md scopes. A scanner
//! and a block tree parser for the stated grammar subset,
//! then the mapping onto the grading model. Files the subset
//! cannot scan come back as reader errors, never as partial
//! grades.
//!
//! The mapping follows the API reader where the two describe
//! the same things, so a model grades the same from its
//! files and from the API document. Views are models,
//! dimensions are items, synonyms are alias claims, explore
//! joins are relationships. A LookML measure is an
//! aggregation over fields, so it maps to a metric whose
//! measure references are the field references inside its
//! SQL, the dollar brace form with the TABLE pseudo
//! reference excluded. Cross view references, the dotted
//! form, need project scope and wait for slice 2, as the
//! design record states.

use super::{Alias, Definition, Dimension, Metric, Relationship};

/// One parsed block or property in a LookML file.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Prop(String, Value),
    Block(String, Option<String>, Vec<Item>),
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Single(String),
    List(Vec<String>),
    Sql(String),
}

/// Parse one LookML file's text into the grading model.
pub fn read_lookml_definition(text: &str) -> Result<Definition, String> {
    let items = parse_file(text)?;
    let mut def = Definition::default();
    for item in &items {
        match item {
            Item::Block(kind, Some(name), children) if kind == "view" => {
                def.model_names.push(name.clone());
                collect_view(children, name, &mut def);
            }
            Item::Block(kind, Some(name), children) if kind == "explore" => {
                collect_explore(children, name, &mut def);
            }
            _ => {}
        }
    }
    Ok(def)
}

fn collect_view(children: &[Item], view: &str, def: &mut Definition) {
    for item in children {
        match item {
            Item::Block(kind, Some(name), fields)
                if kind == "dimension" || kind == "dimension_group" =>
            {
                push_aliases(fields, &format!("dimension {view}.{name}"), def);
                def.dimensions.push(Dimension {
                    name: name.clone(),
                    label: prop_string(fields, "label"),
                    description: prop_string(fields, "description"),
                    model: view.to_string(),
                });
            }
            Item::Block(kind, Some(name), fields) if kind == "measure" => {
                push_aliases(fields, &format!("measure {view}.{name}"), def);
                let mut refs = match prop(fields, "sql") {
                    Some(Value::Sql(sql)) => extract_refs(sql),
                    _ => Vec::new(),
                };
                refs.sort();
                refs.dedup();
                def.metrics.push(Metric {
                    name: name.clone(),
                    label: prop_string(fields, "label"),
                    description: prop_string(fields, "description"),
                    kind: "lookml_measure".to_string(),
                    measure_refs: refs,
                    metric_refs: Vec::new(),
                });
            }
            _ => {}
        }
    }
}

fn collect_explore(children: &[Item], explore: &str, def: &mut Definition) {
    let base = children
        .iter()
        .find_map(|item| match item {
            Item::Prop(key, Value::Single(v)) if key == "view_name" => Some(v.clone()),
            _ => None,
        })
        .unwrap_or_else(|| explore.to_string());
    for item in children {
        if let Item::Block(kind, Some(name), _) = item
            && kind == "join"
        {
            def.relationships.push(Relationship {
                name: format!("{base}_to_{name}"),
                left_table: base.clone(),
                right_table: name.clone(),
            });
        }
    }
}

fn push_aliases(fields: &[Item], owner: &str, def: &mut Definition) {
    if let Some(Value::List(terms)) = prop(fields, "synonyms") {
        for term in terms {
            def.aliases.push(Alias {
                term: term.clone(),
                owner: owner.to_string(),
            });
        }
    }
}

fn prop<'a>(fields: &'a [Item], key: &str) -> Option<&'a Value> {
    fields.iter().find_map(|item| match item {
        Item::Prop(k, v) if k == key => Some(v),
        _ => None,
    })
}

fn prop_string(fields: &[Item], key: &str) -> Option<String> {
    match prop(fields, key) {
        Some(Value::Single(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Field references inside a SQL chunk, the dollar brace
/// form. Unqualified names only in this slice. The TABLE
/// pseudo reference is Looker's own and claims nothing.
fn extract_refs(sql: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = sql;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else { break };
        let name = after[..end].trim();
        if !name.is_empty() && name != "TABLE" && !name.contains('.') {
            refs.push(name.to_string());
        }
        rest = &after[end + 1..];
    }
    refs
}

// The scanner and tree parser. The scanner is character
// based and the parser drives it, because SQL chunks are
// raw text up to their double semicolon terminator and only
// the parser knows when one starts, by property key.

struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Scanner {
            chars: text.chars().peekable(),
        }
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            while matches!(self.chars.peek(), Some(c) if c.is_whitespace()) {
                self.chars.next();
            }
            if self.chars.peek() == Some(&'#') {
                for c in self.chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn peek_non_space(&mut self) -> Option<char> {
        self.skip_space_and_comments();
        self.chars.peek().copied()
    }

    fn take(&mut self) -> Option<char> {
        self.chars.next()
    }

    /// A bareword runs over identifier characters, dots
    /// included, so qualified names stay whole.
    fn bareword(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.chars.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '.' | '-' | '/') {
                out.push(*c);
                self.chars.next();
            } else {
                break;
            }
        }
        out
    }

    fn quoted(&mut self) -> Result<String, String> {
        // The opening quote is current.
        self.chars.next();
        let mut out = String::new();
        for c in self.chars.by_ref() {
            match c {
                '"' => return Ok(out),
                '\\' => {
                    if let Some(next) = self.chars.next() {
                        out.push(next);
                    }
                }
                _ => out.push(c),
            }
        }
        Err("unterminated string".to_string())
    }

    /// Raw text up to the double semicolon terminator, which
    /// is consumed. Comments inside a SQL chunk are SQL, not
    /// LookML comments, and survive.
    fn sql_chunk(&mut self) -> Result<String, String> {
        let mut out = String::new();
        let mut prev = '\0';
        for c in self.chars.by_ref() {
            if c == ';' && prev == ';' {
                out.pop();
                return Ok(out.trim().to_string());
            }
            prev = c;
            out.push(c);
        }
        Err("unterminated sql chunk, no ;; terminator".to_string())
    }
}

/// SQL carrying property keys. A key starting with sql, plus
/// html, carries a raw chunk. Everything else is a token
/// value.
fn is_sql_key(key: &str) -> bool {
    key.starts_with("sql") || key == "html"
}

fn parse_file(text: &str) -> Result<Vec<Item>, String> {
    let mut scanner = Scanner::new(text);
    parse_items(&mut scanner, true)
}

fn parse_items(scanner: &mut Scanner, top: bool) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    loop {
        match scanner.peek_non_space() {
            None => {
                if top {
                    return Ok(items);
                }
                return Err("unexpected end of file inside a block".to_string());
            }
            Some('}') => {
                if top {
                    return Err("unmatched closing brace".to_string());
                }
                scanner.take();
                return Ok(items);
            }
            Some(_) => {}
        }
        let key = scanner.bareword();
        if key.is_empty() {
            return Err("expected a name".to_string());
        }
        if scanner.peek_non_space() != Some(':') {
            return Err(format!("expected : after {key}"));
        }
        scanner.take();
        match scanner.peek_non_space() {
            Some('{') => {
                scanner.take();
                let children = parse_items(scanner, false)?;
                items.push(Item::Block(key, None, children));
            }
            Some('"') => {
                let value = scanner.quoted()?;
                if scanner.peek_non_space() == Some('{') {
                    scanner.take();
                    let children = parse_items(scanner, false)?;
                    items.push(Item::Block(key, Some(value), children));
                } else {
                    items.push(Item::Prop(key, Value::Single(value)));
                }
            }
            Some('[') => {
                scanner.take();
                let list = parse_list(scanner)?;
                items.push(Item::Prop(key, Value::List(list)));
            }
            Some(_) => {
                if is_sql_key(&key) {
                    let sql = scanner.sql_chunk()?;
                    items.push(Item::Prop(key, Value::Sql(sql)));
                    continue;
                }
                let word = scanner.bareword();
                if word.is_empty() {
                    return Err(format!("expected a value after {key}:"));
                }
                if scanner.peek_non_space() == Some('{') {
                    scanner.take();
                    let children = parse_items(scanner, false)?;
                    items.push(Item::Block(key, Some(word), children));
                } else {
                    items.push(Item::Prop(key, Value::Single(word)));
                }
            }
            None => return Err(format!("unexpected end of file after {key}:")),
        }
    }
}

fn parse_list(scanner: &mut Scanner) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    loop {
        match scanner.peek_non_space() {
            Some(']') => {
                scanner.take();
                return Ok(out);
            }
            Some('"') => {
                out.push(scanner.quoted()?);
            }
            Some(',') => {
                scanner.take();
            }
            Some(_) => {
                let word = scanner.bareword();
                if word.is_empty() {
                    return Err("bad list entry".to_string());
                }
                out.push(word);
            }
            None => return Err("unterminated list".to_string()),
        }
    }
}
