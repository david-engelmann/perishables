//! The native LookML reader, slices 1 and 2 of the project
//! the design record at docs/DESIGN-LOOKML.md scopes. A
//! scanner and a block tree parser for the stated grammar
//! subset, then the mapping onto the grading model. Files
//! the subset cannot scan come back as reader errors, never
//! as partial grades.
//!
//! The mapping follows the API reader where the two describe
//! the same things, so a model grades the same from its
//! files and from the API document. Views are models,
//! dimensions are items, synonyms are alias claims, explore
//! joins are relationships. A LookML measure is an
//! aggregation over fields, so it maps to a metric whose
//! measure references are the field references inside its
//! SQL, the dollar brace form with the TABLE pseudo
//! reference excluded. Slice 2 adds project scope. Files
//! merge, extends inherits fields and grades as a structural
//! reference, dotted cross view references resolve against
//! the project's qualified field set, and include patterns
//! match against the provided file paths.

use super::{Alias, Definition, Dimension, Metric, Relationship};

/// One parsed block or property in a LookML file. The tree
/// is the reader's public intermediate, as the design
/// record states, so a later slice can grade surfaces the
/// mapping does not consume, sets and parameters among
/// them, without reparsing.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Prop(String, Value),
    Block(String, Option<String>, Vec<Item>),
}

/// A property value in the tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Single(String),
    List(Vec<String>),
    Sql(String),
}

/// Parse one LookML file into its block tree, the public
/// intermediate the design record promises.
pub fn parse_tree(text: &str) -> Result<Vec<Item>, String> {
    parse_file(text)
}

/// Parse one LookML file's text into the grading model.
pub fn read_lookml_definition(text: &str) -> Result<Definition, String> {
    read_lookml_project(&[("inline.lookml".to_string(), text.to_string())])
}

/// Parse a whole LookML project, as path and text pairs,
/// into the grading model. Slice 2 of the design record.
/// Views and explores merge across files, extends inherit,
/// dotted cross view references resolve against the
/// project's qualified field set, and include patterns are
/// matched against the provided paths.
pub fn read_lookml_project(files: &[(String, String)]) -> Result<Definition, String> {
    let mut views: Vec<ViewData> = Vec::new();
    let mut explores: Vec<(String, Vec<Item>)> = Vec::new();
    let mut includes: Vec<String> = Vec::new();
    for (_path, text) in files {
        for item in parse_file(text)? {
            match item {
                Item::Block(kind, Some(name), children) if kind == "view" => {
                    views.push(collect_view(&children, &name));
                }
                Item::Block(kind, Some(name), children) if kind == "explore" => {
                    explores.push((name, children));
                }
                Item::Prop(key, Value::Single(pattern)) if key == "include" => {
                    includes.push(pattern);
                }
                _ => {}
            }
        }
    }
    let mut def = Definition::default();
    let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
    for pattern in &includes {
        if !paths.iter().any(|p| include_matches(pattern, p)) {
            def.unmatched_includes.push(pattern.clone());
        }
    }
    resolve_extends(&mut views, &mut def);
    for view in &views {
        def.model_names.push(view.name.clone());
        for dim in &view.dimensions {
            def.qualified_fields
                .push(format!("{}.{}", view.name, dim.name));
            def.dimensions.push(dim.clone());
        }
        for metric in &view.metrics {
            def.qualified_fields
                .push(format!("{}.{}", view.name, metric.name));
            def.metrics.push(metric.clone());
        }
        def.aliases.extend(view.aliases.iter().cloned());
    }
    for (name, children) in &explores {
        collect_explore(children, name, &mut def);
    }
    Ok(def)
}

/// One view's own fields, before extends inheritance.
struct ViewData {
    name: String,
    dimensions: Vec<Dimension>,
    metrics: Vec<Metric>,
    aliases: Vec<Alias>,
    extends: Vec<String>,
}

fn collect_view(children: &[Item], view: &str) -> ViewData {
    let mut data = ViewData {
        name: view.to_string(),
        dimensions: Vec::new(),
        metrics: Vec::new(),
        aliases: Vec::new(),
        extends: Vec::new(),
    };
    if let Some(Value::List(parents)) = prop(children, "extends") {
        data.extends = parents.clone();
    }
    for item in children {
        match item {
            Item::Block(kind, Some(name), fields)
                if kind == "dimension" || kind == "dimension_group" =>
            {
                push_aliases(
                    fields,
                    &format!("dimension {view}.{name}"),
                    &mut data.aliases,
                );
                data.dimensions.push(Dimension {
                    name: name.clone(),
                    label: prop_string(fields, "label"),
                    description: prop_string(fields, "description"),
                    model: view.to_string(),
                });
            }
            Item::Block(kind, Some(name), fields) if kind == "measure" => {
                push_aliases(fields, &format!("measure {view}.{name}"), &mut data.aliases);
                let mut refs = match prop(fields, "sql") {
                    Some(Value::Sql(sql)) => extract_refs(sql),
                    _ => Vec::new(),
                };
                refs.sort();
                refs.dedup();
                data.metrics.push(Metric {
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
    data
}

/// Extends is a declared structural reference between
/// views, so it enters as a relationship and the existing
/// family grades a missing parent. Inheritance then merges
/// each parent's fields into the child, the child's own
/// fields winning on a name clash, over as many passes as
/// the chain needs.
fn resolve_extends(views: &mut Vec<ViewData>, def: &mut Definition) {
    for view in views.iter() {
        for parent in &view.extends {
            def.relationships.push(Relationship {
                name: format!("{}_to_{parent}", view.name),
                left_table: view.name.clone(),
                right_table: parent.clone(),
            });
        }
    }
    for _ in 0..views.len() {
        let mut changed = false;
        for i in 0..views.len() {
            let parents = views[i].extends.clone();
            for parent in parents {
                let Some(parent_data) = views.iter().find(|v| v.name == parent) else {
                    continue;
                };
                // Collect only the fields the child lacks,
                // under shared borrows, then extend once. The
                // whole parent vectors are not cloned per pass.
                let missing_dims: Vec<Dimension> = parent_data
                    .dimensions
                    .iter()
                    .filter(|d| !views[i].dimensions.iter().any(|c| c.name == d.name))
                    .cloned()
                    .collect();
                let missing_metrics: Vec<Metric> = parent_data
                    .metrics
                    .iter()
                    .filter(|m| !views[i].metrics.iter().any(|c| c.name == m.name))
                    .cloned()
                    .collect();
                if !missing_dims.is_empty() || !missing_metrics.is_empty() {
                    // An inherited field keeps its home view as
                    // its model. It is the parent's field made
                    // addressable through the child, which the
                    // qualified field set records, not a second
                    // competing claim by the child.
                    views[i].dimensions.extend(missing_dims);
                    views[i].metrics.extend(missing_metrics);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

/// A minimal include matcher. Patterns are matched on the
/// path with any leading slash dropped, a star standing for
/// any run of characters. That covers the forms model files
/// use in practice, star dot extension and directory globs.
fn include_matches(pattern: &str, path: &str) -> bool {
    let pattern = pattern.strip_prefix('/').unwrap_or(pattern);
    let path = path.strip_prefix('/').unwrap_or(path);
    match pattern.split_once('*') {
        None => pattern == path,
        Some((prefix, suffix)) => {
            path.len() >= prefix.len() + suffix.len()
                && path.starts_with(prefix)
                && path.ends_with(suffix)
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

fn push_aliases(fields: &[Item], owner: &str, aliases: &mut Vec<Alias>) {
    if let Some(Value::List(terms)) = prop(fields, "synonyms") {
        for term in terms {
            aliases.push(Alias {
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
/// form. Unqualified names are fields of the carrying view.
/// Dotted names are cross view references, carried whole and
/// resolved against the project's qualified field set. The
/// TABLE pseudo reference is Looker's own and claims
/// nothing.
fn extract_refs(sql: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = sql;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else { break };
        let name = after[..end].trim();
        if !name.is_empty() && name != "TABLE" && !name.starts_with("TABLE.") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_chunks_end_at_the_double_semicolon_only() {
        // A single semicolon inside the chunk is content.
        // The chunk ends at the terminator, and parsing
        // continues after it.
        let items =
            parse_file("view: v { dimension: d { sql: SELECT 1; SELECT 2 ;; } }").expect("parses");
        let Item::Block(_, _, children) = &items[0] else {
            panic!("a view block");
        };
        let Item::Block(_, _, fields) = &children[0] else {
            panic!("a dimension block");
        };
        assert_eq!(
            prop(fields, "sql"),
            Some(&Value::Sql("SELECT 1; SELECT 2".to_string()))
        );
    }

    #[test]
    fn comments_inside_sql_chunks_survive_and_outside_do_not() {
        // Inside a SQL chunk the hash is SQL content. Outside,
        // it is a LookML comment and is skipped, so the file
        // below has exactly one property in the dimension.
        let text = "view: v {\n  # a comment\n  dimension: d {\n    sql: ${TABLE}.x -- keep # this\n    ;;\n  }\n}";
        let items = parse_file(text).expect("parses");
        let Item::Block(_, _, children) = &items[0] else {
            panic!("a view block");
        };
        let Item::Block(_, _, fields) = &children[0] else {
            panic!("a dimension block");
        };
        assert_eq!(fields.len(), 1);
        let Some(Value::Sql(sql)) = prop(fields, "sql") else {
            panic!("sql prop");
        };
        assert!(sql.contains("# this"), "got {sql}");
    }

    #[test]
    fn interpolation_is_captured_not_expanded() {
        assert_eq!(
            extract_refs("${amount} + ${customers.name} + ${TABLE}.id"),
            vec!["amount".to_string(), "customers.name".to_string()]
        );
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
