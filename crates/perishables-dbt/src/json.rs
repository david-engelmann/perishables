//! A minimal JSON reader, hand written because this workspace
//! carries no external dependencies. It covers the full JSON data
//! model and nothing else. Numbers keep their source text so no
//! precision is lost on values the grader only passes through.

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Number(String),
    Str(String),
    List(Vec<Json>),
    Map(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

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
                        let mut code = 0u32;
                        for _ in 0..4 {
                            let c = self.bump().ok_or("bad unicode escape")?;
                            code = code * 16 + c.to_digit(16).ok_or("bad unicode escape")?;
                        }
                        out.push(char::from_u32(code).ok_or("bad unicode escape")?);
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
}
