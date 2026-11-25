//! A minimal JSON reader, hand written because this workspace
//! carries no external dependencies. It covers the full JSON data
//! model and nothing else. Numbers keep their source text so no
//! precision is lost on values the grader only passes through.

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Json {
    /// JSON `null`.
    Null,
    /// JSON boolean (`true` or `false`).
    Bool(bool),
    /// JSON number preserved in source text form.
    Number(String),
    /// JSON UTF-8 string with escapes decoded.
    Str(String),
    /// JSON array.
    List(Vec<Json>),
    /// JSON object preserving entry order.
    Map(Vec<(String, Json)>),
}

impl Json {
    /// Look up `key` when this value is a [`Json::Map`].
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Borrow the string slice when this value is [`Json::Str`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Borrow the item slice when this value is [`Json::List`].
    pub fn as_list(&self) -> Option<&[Json]> {
        match self {
            Json::List(items) => Some(items),
            _ => None,
        }
    }

    /// String items of a list, skipping anything that is not a
    /// string. Manifest reference lists are string lists by spec.
    pub fn as_str_list(&self) -> Vec<String> {
        match self {
            Json::List(items) => items
                .iter()
                .filter_map(|i| i.as_str().map(str::to_owned))
                .collect(),
            // A bare string is a one item list in the forms
            // the readers consume, synonyms above all. Dropping
            // it silently would lose a stated claim.
            Json::Str(s) => vec![s.clone()],
            _ => Vec::new(),
        }
    }
}

pub fn parse(text: &str) -> Result<Json, String> {
    let mut parser = Parser {
        chars: text.chars().collect(),
        pos: 0,
    };
    parser.skip_ws();
    let value = parser.value()?;
    parser.skip_ws();
    if parser.pos != parser.chars.len() {
        return Err(format!("trailing characters at offset {}", parser.pos));
    }
    Ok(value)
}

fn is_number_char(c: char) -> bool {
    c == '-' || c == '+' || c == '.' || c == 'e' || c == 'E' || c.is_ascii_digit()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, wanted: char) -> Result<(), String> {
        if self.bump() == Some(wanted) {
            Ok(())
        } else {
            Err(format!("expected {wanted} at offset {}", self.pos))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('t') => self.keyword("true", Json::Bool(true)),
            Some('f') => self.keyword("false", Json::Bool(false)),
            Some('n') => self.keyword("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!(
                "unexpected character {other:?} at offset {}",
                self.pos
            )),
        }
    }

    fn keyword(&mut self, word: &str, value: Json) -> Result<Json, String> {
        for wanted in word.chars() {
            if self.bump() != Some(wanted) {
                return Err(format!("bad literal at offset {}", self.pos));
            }
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if is_number_char(c)) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(format!("bad number at offset {start}"));
        }
        Ok(Json::Number(self.chars[start..self.pos].iter().collect()))
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let mut code = 0u32;
        for _ in 0..4 {
            let c = self.bump().ok_or("bad unicode escape")?;
            code = code * 16 + c.to_digit(16).ok_or("bad unicode escape")?;
        }
        Ok(code as u16)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated string".to_string()),
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000C}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let unit = self.hex4()?;
                        if (0xD800..=0xDBFF).contains(&unit) {
                            if self.bump() != Some('\\') || self.bump() != Some('u') {
                                return Err("unpaired high surrogate".to_string());
                            }
                            let low = self.hex4()?;
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err("invalid low surrogate".to_string());
                            }
                            let scalar =
                                0x10000 + (((unit as u32 - 0xD800) << 10) | (low as u32 - 0xDC00));
                            out.push(char::from_u32(scalar).ok_or("bad unicode escape")?);
                        } else if (0xDC00..=0xDFFF).contains(&unit) {
                            return Err("unpaired low surrogate".to_string());
                        } else {
                            out.push(char::from_u32(unit as u32).ok_or("bad unicode escape")?);
                        }
                    }
                    _ => return Err("bad escape".to_string()),
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect('[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.pos += 1;
            return Ok(Json::List(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.bump() {
                Some(']') => return Ok(Json::List(items)),
                Some(',') => continue,
                other => return Err(format!("expected , or ] in array, got {other:?}")),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect('{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(Json::Map(entries));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.bump() {
                Some('}') => return Ok(Json::Map(entries)),
                Some(',') => continue,
                other => return Err(format!("expected , or }} in object, got {other:?}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_manifest_shapes() {
        let value = parse(r#"{"a": [1, "two", true, null], "b": {"c": "x\"y"}, "n": -1.5e3}"#)
            .expect("parses");
        assert_eq!(
            value.get("a").map(|a| a.as_list().map(<[Json]>::len)),
            Some(Some(4))
        );
        assert_eq!(
            value
                .get("b")
                .and_then(|b| b.get("c"))
                .and_then(Json::as_str),
            Some("x\"y")
        );
        assert_eq!(value.get("n"), Some(&Json::Number("-1.5e3".to_string())));
        assert_eq!(value.get("missing"), None);
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("{} extra").is_err());
        assert!(parse("{\"a\": }").is_err());
    }

    #[test]
    fn decodes_utf16_surrogate_pairs_and_rejects_unpaired_surrogates() {
        let val = parse(r#"{"crab": "\uD83E\uDD80"}"#).expect("valid surrogate pair");
        assert_eq!(val.get("crab").and_then(Json::as_str), Some("🦀"));
        assert!(parse(r#"{"bad": "\uD83E"}"#).is_err());
        assert!(parse(r#"{"bad": "\uD83E\u0041"}"#).is_err());
        assert!(parse(r#"{"bad": "\uDD80"}"#).is_err());
    }
}
