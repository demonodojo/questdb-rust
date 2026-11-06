use qdb_types::{parse_date_millis, parse_timestamp_micros, Error, Scalar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub columns: SelectList,
    pub table: String,
    pub filter: Option<Filter>,
    pub order_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectList {
    Star,
    Columns(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub column: String,
    pub op: CmpOp,
    pub literal: Scalar,
}

pub fn parse_query(sql: &str) -> Result<Query, Error> {
    let mut parser = Parser {
        src: sql.as_bytes(),
        pos: 0,
    };
    parser.skip();
    parser.expect_keyword("select")?;
    let columns = parser.parse_select_list()?;
    parser.expect_keyword("from")?;
    let table = parser.ident()?;
    parser.skip();
    let mut filter = None;
    let mut order_by = None;
    if parser.eat_keyword("where") {
        filter = Some(parser.parse_filter()?);
        parser.skip();
    }
    if parser.eat_keyword("order") {
        parser.expect_keyword("by")?;
        let column = parser.ident()?;
        parser.skip();
        if parser.eat_keyword("asc") {
            parser.skip();
        } else if parser.eat_keyword("desc") {
            return Err(Error::new("unsupported: ORDER BY DESC"));
        }
        order_by = Some(column);
    }
    if parser.eat(b';') {
        parser.skip();
    }
    if parser.pos != parser.src.len() {
        return Err(Error::new(format!(
            "unsupported: trailing input near `{}`",
            parser.rest()
        )));
    }
    Ok(Query {
        columns,
        table,
        filter,
        order_by,
    })
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &str {
        std::str::from_utf8(&self.src[self.pos..]).unwrap_or("")
    }

    fn skip(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn eat(&mut self, byte: u8) -> bool {
        self.skip();
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_keyword(&mut self, word: &str) -> bool {
        self.skip();
        let start = self.pos;
        if self.ident_raw().map(|got| got.eq_ignore_ascii_case(word)) == Some(true) {
            true
        } else {
            self.pos = start;
            false
        }
    }

    fn expect_keyword(&mut self, word: &str) -> Result<(), Error> {
        if self.eat_keyword(word) {
            Ok(())
        } else {
            Err(Error::new(format!(
                "unsupported: expected {word}, found `{}`",
                self.rest()
            )))
        }
    }

    fn ident_raw(&mut self) -> Option<String> {
        self.skip();
        let start = self.pos;
        let first = self.peek()?;
        if !first.is_ascii_alphabetic() && first != b'_' {
            return None;
        }
        self.pos += 1;
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'_') {
            self.pos += 1;
        }
        Some(
            std::str::from_utf8(&self.src[start..self.pos])
                .unwrap()
                .to_ascii_lowercase(),
        )
    }

    fn ident(&mut self) -> Result<String, Error> {
        self.ident_raw()
            .ok_or_else(|| Error::new(format!("expected identifier near `{}`", self.rest())))
    }

    fn parse_select_list(&mut self) -> Result<SelectList, Error> {
        self.skip();
        if self.eat(b'*') {
            return Ok(SelectList::Star);
        }
        if !matches!(self.peek(), Some(b) if b.is_ascii_alphabetic() || b == b'_') {
            return Err(Error::new(
                "unsupported: SELECT list must name columns or `*`",
            ));
        }
        let mut columns = Vec::new();
        loop {
            columns.push(self.ident()?);
            self.skip();
            if !self.eat(b',') {
                break;
            }
        }
        if columns.is_empty() {
            return Err(Error::new("unsupported: empty select list"));
        }
        Ok(SelectList::Columns(columns))
    }

    fn parse_filter(&mut self) -> Result<Filter, Error> {
        let column = self.ident()?;
        let op = self.parse_op()?;
        let literal = self.parse_literal()?;
        Ok(Filter {
            column,
            op,
            literal,
        })
    }

    fn parse_op(&mut self) -> Result<CmpOp, Error> {
        self.skip();
        let two = self.src.get(self.pos..self.pos + 2);
        let op = match two {
            Some(b"!=") | Some(b"<>") => CmpOp::NotEq,
            Some(b"<=") => CmpOp::LtEq,
            Some(b">=") => CmpOp::GtEq,
            _ => match self.peek() {
                Some(b'=') => CmpOp::Eq,
                Some(b'<') => CmpOp::Lt,
                Some(b'>') => CmpOp::Gt,
                _ => {
                    return Err(Error::new(format!(
                        "unsupported: expected comparison operator near `{}`",
                        self.rest()
                    )))
                }
            },
        };
        self.pos += match op {
            CmpOp::NotEq | CmpOp::LtEq | CmpOp::GtEq => 2,
            _ => 1,
        };
        Ok(op)
    }

    fn parse_literal(&mut self) -> Result<Scalar, Error> {
        self.skip();
        if self.peek() == Some(b'\'') {
            return Ok(Scalar::Text(self.parse_string()?));
        }
        if let Some(ident) = self.ident_raw() {
            return match ident.as_str() {
                "null" => Ok(Scalar::Null),
                "true" => Ok(Scalar::Boolean(true)),
                "false" => Ok(Scalar::Boolean(false)),
                other => Err(Error::new(format!("unsupported literal `{other}`"))),
            };
        }
        self.parse_number()
    }

    fn parse_string(&mut self) -> Result<String, Error> {
        self.skip();
        if self.peek() != Some(b'\'') {
            return Err(Error::new("expected string literal"));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.src.len() {
            let byte = self.src[self.pos];
            self.pos += 1;
            if byte == b'\'' {
                if self.peek() == Some(b'\'') {
                    self.pos += 1;
                    out.push('\'');
                    continue;
                }
                return Ok(out);
            }
            if byte.is_ascii() {
                out.push(byte as char);
            } else {
                // Restart on the non-ascii byte and decode a UTF-8 char.
                self.pos -= 1;
                let rest = std::str::from_utf8(&self.src[self.pos..])
                    .map_err(|_| Error::new("string literal is not valid UTF-8"))?;
                let ch = rest.chars().next().unwrap();
                self.pos += ch.len_utf8();
                out.push(ch);
            }
        }
        Err(Error::new("unterminated string literal"))
    }

    fn parse_number(&mut self) -> Result<Scalar, Error> {
        self.skip();
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        if !matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            return Err(Error::new(format!(
                "unsupported literal near `{}`",
                self.rest()
            )));
        }
        while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            float = true;
            self.pos += 1;
            if !matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                return Err(Error::new("unsupported: trailing decimal point"));
            }
            while matches!(self.peek(), Some(b) if b.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        if float {
            let value: f64 = text
                .parse()
                .map_err(|_| Error::new(format!("invalid number `{text}`")))?;
            Ok(Scalar::Double(value))
        } else {
            let value: i64 = text
                .parse()
                .map_err(|_| Error::new(format!("invalid integer `{text}`")))?;
            Ok(Scalar::Long(value))
        }
    }
}

pub fn timestamp_from_literal(value: &Scalar) -> Result<Option<i64>, Error> {
    match value {
        Scalar::Null => Ok(None),
        Scalar::Text(text) => parse_timestamp_micros(text)
            .map(Some)
            .ok_or_else(|| Error::new(format!("invalid timestamp literal `{text}`"))),
        Scalar::Timestamp(v) => Ok(Some(*v)),
        other => Err(Error::new(format!(
            "unsupported timestamp comparison with {other}"
        ))),
    }
}

pub fn date_from_literal(value: &Scalar) -> Result<Option<i64>, Error> {
    match value {
        Scalar::Null => Ok(None),
        Scalar::Text(text) => parse_date_millis(text)
            .map(Some)
            .ok_or_else(|| Error::new(format!("invalid date literal `{text}`"))),
        Scalar::Date(v) => Ok(Some(*v)),
        other => Err(Error::new(format!(
            "unsupported date comparison with {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filter_and_rejects_select_constant() {
        let query = parse_query("SELECT id, note FROM sensors WHERE id > 1 ORDER BY ts").unwrap();
        assert_eq!(query.table, "sensors");
        assert!(query.order_by.as_deref() == Some("ts"));
        let err = parse_query("SELECT 1").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
    }
}
