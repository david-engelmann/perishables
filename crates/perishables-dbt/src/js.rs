//! A JavaScript subset reader for Cube model files. Cube
//! models are JavaScript, calls to cube and view functions
//! whose second argument is an object literal. This module
//! scans and parses that subset into the crate's Json
//! values, so the Cube mapping can consume it. The subset
//! is stated plainly. Object literals with bare or quoted
//! keys, string values in single, double, or backtick
//! quotes, numbers, booleans, null, arrays, and nested
//! objects. Line and block comments. Template interpolation
//! is carried as raw text inside the string, because the
//! reference is a stated fact and expansion is Cube's job.
//! A bare or dotted identifier is a reference by name and
//! parses as its name text. Anything else, function values,
//! spreads, computed keys, arbitrary expressions, is a
//! reader error, never a partial parse.

use super::Json;
use std::collections::BTreeMap;

/// One top level call, a function name and its arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub name: String,
    pub args: Vec<Json>,
}

/// Parse a Cube model file into its top level calls.
pub fn parse_calls(text: &str) -> Result<Vec<Call>, String> {
    let mut scanner = Scanner::new(text);
    let mut calls = Vec::new();
    loop {
        scanner.skip_space_and_comments()?;
        match scanner.peek() {
            None => return Ok(calls),
            Some(_) => {
                let name = scanner.ident()?;
                scanner.expect('(')?;
                let mut args = Vec::new();
                loop {
                    args.push(parse_value(&mut scanner)?);
                    scanner.skip_space_and_comments()?;
                    match scanner.take() {
                        Some(',') => continue,
                        Some(')') => break,
                        other => {
                            return Err(format!("expected , or ) in call, found {other:?}"));
                        }
                    }
                }
                scanner.skip_space_and_comments()?;
                if scanner.peek() == Some(';') {
                    scanner.take();
                }
                calls.push(Call { name, args });
            }
        }
    }
}

fn parse_value(scanner: &mut Scanner) -> Result<Json, String> {
    scanner.skip_space_and_comments()?;
    match scanner.peek() {
        Some('{') => {
            scanner.take();
            let mut map = BTreeMap::new();
            loop {
                scanner.skip_space_and_comments()?;
                if scanner.peek() == Some('}') {
                    scanner.take();
                    return Ok(Json::Map(map));
                }
                let key = match scanner.peek() {
                    Some('"') | Some('\'') => scanner.string()?,
                    _ => scanner.ident()?,
                };
                scanner.expect(':')?;
                let value = parse_value(scanner)?;
                map.insert(key, value);
                scanner.skip_space_and_comments()?;
                match scanner.take() {
                    Some(',') => continue,
                    Some('}') => return Ok(Json::Map(map)),
                    other => return Err(format!("expected , or }} in object, found {other:?}")),
                }
            }
        }
        Some('[') => {
            scanner.take();
            let mut items = Vec::new();
            loop {
                scanner.skip_space_and_comments()?;
                if scanner.peek() == Some(']') {
                    scanner.take();
                    return Ok(Json::List(items));
                }
                items.push(parse_value(scanner)?);
                scanner.skip_space_and_comments()?;
                match scanner.take() {
                    Some(',') => continue,
                    Some(']') => return Ok(Json::List(items)),
                    other => return Err(format!("expected , or ] in array, found {other:?}")),
                }
            }
        }
        Some('"') | Some('\'') | Some('`') => Ok(Json::Str(scanner.string()?)),
        Some(c) if c.is_ascii_digit() || c == '-' => Ok(Json::Number(scanner.number()?)),
        Some(_) => {
            let word = scanner.dotted_ident()?;
            match word.as_str() {
                "true" => Ok(Json::Bool(true)),
                "false" => Ok(Json::Bool(false)),
                "null" => Ok(Json::Null),
                // A bare identifier, or a dotted chain of
                // them, is a reference to a definition by
                // name, join_path in Cube files. It parses as
                // its name text, because the name is the
                // gradeable fact.
                _ => Ok(Json::Str(word)),
            }
        }
        None => Err("unexpected end of file".to_string()),
    }
}

struct Scanner<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Scanner {
            chars: text.chars().peekable(),
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn take(&mut self) -> Option<char> {
        self.chars.next()
    }

    fn skip_space_and_comments(&mut self) -> Result<(), String> {
        loop {
            while matches!(self.chars.peek(), Some(c) if c.is_whitespace()) {
                self.chars.next();
            }
            if self.chars.peek() == Some(&'/') {
                let mut lookahead = self.chars.clone();
                lookahead.next();
                match lookahead.peek() {
                    Some('/') => {
                        for c in self.chars.by_ref() {
                            if c == '\n' {
                                break;
                            }
                        }
                        continue;
                    }
                    Some('*') => {
                        self.chars.next();
                        self.chars.next();
                        let mut prev = '\0';
                        let mut closed = false;
                        for c in self.chars.by_ref() {
                            if prev == '*' && c == '/' {
                                closed = true;
                                break;
                            }
                            prev = c;
                        }
                        if !closed {
                            return Err("unterminated block comment".to_string());
                        }
                        continue;
                    }
                    _ => return Err("a bare / is outside the stated subset".to_string()),
                }
            }
            break;
        }
        Ok(())
    }

    fn expect(&mut self, want: char) -> Result<(), String> {
        self.skip_space_and_comments()?;
        if self.chars.next() == Some(want) {
            Ok(())
        } else {
            Err(format!("expected {want}"))
        }
    }

    /// An identifier or a dotted chain of identifiers, the
    /// form a reference takes in Cube files.
    fn dotted_ident(&mut self) -> Result<String, String> {
        let mut out = self.ident()?;
        while self.peek() == Some('.') {
            self.take();
            out.push('.');
            out.push_str(&self.ident()?);
        }
        Ok(out)
    }

    fn ident(&mut self) -> Result<String, String> {
        self.skip_space_and_comments()?;
        let mut out = String::new();
        while let Some(c) = self.chars.peek() {
            if c.is_alphanumeric() || matches!(c, '_' | '$') {
                out.push(*c);
                self.chars.next();
            } else {
                break;
            }
        }
        if out.is_empty() {
            Err("expected an identifier".to_string())
        } else {
            Ok(out)
        }
    }

    /// A string in any of the three quote forms. Backtick
    /// interpolation is carried raw, dollar brace and all.
    fn string(&mut self) -> Result<String, String> {
        let quote = self.chars.next().ok_or("expected a string")?;
        let mut out = String::new();
        for c in self.chars.by_ref() {
            match c {
                '\\' => {
                    if let Some(next) = self.chars.next() {
                        out.push(next);
                    }
                }
                c if c == quote => return Ok(out),
                _ => out.push(c),
            }
        }
        Err("unterminated string".to_string())
    }

    fn number(&mut self) -> Result<String, String> {
        let mut out = String::new();
        while let Some(c) = self.chars.peek() {
            if c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E') {
                out.push(*c);
                self.chars.next();
            } else {
                break;
            }
        }
        if out.is_empty() {
            Err("expected a number".to_string())
        } else {
            Ok(out)
        }
    }
}
