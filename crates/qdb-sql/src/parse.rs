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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderBy {
    pub column: String,
    pub descending: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub columns: SelectList,
    pub table: String,
    pub filter: Option<Filter>,
    /// One `GROUP BY` column. Several columns are rejected.
    pub group_by: Option<String>,
    pub order_by: Vec<OrderBy>,
    /// `LIMIT` bounds after the single-argument form is rewritten.
    /// `LIMIT n` with `n >= 0` is `[0, n)`. A negative `n` is `[n, 0)`.
    pub limit: Option<Limit>,
}

/// Inclusive lower bound and exclusive upper bound, as QuestDB stores them
/// before applying them to the row count. Negative values count from the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limit {
    pub lo: i64,
    pub hi: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFunc {
    Count,
    Min,
    Max,
    Sum,
    Avg,
    First,
    Last,
    CountDistinct,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateExpr {
    pub func: AggFunc,
    /// Result column name, such as `count()`, `min(id)` or `MAX(note)`.
    pub label: String,
    /// `None` for `count()` and `count(*)`. Every other aggregate names a column.
    pub column: Option<String>,
}

/// One grouping column plus one aggregate, in select-list order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupedSelect {
    /// Column spelling from the select list, used as the result header.
    pub key: String,
    pub aggregate: AggregateExpr,
    /// Position of the key in the output row.
    pub key_at: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectList {
    Star,
    Columns(Vec<String>),
    Aggregate(AggregateExpr),
    Grouped(GroupedSelect),
    /// A select list that contains `concat(...)`. Plain columns may sit beside it.
    Projection(Vec<Proj>),
}

/// One select-list value. A column name is stored lowercased; `concat` keeps its label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proj {
    Column(String),
    Concat(ConcatExpr),
    Length(LengthExpr),
    Lower(LowerExpr),
    Upper(UpperExpr),
    Side(SideExpr),
    Substr(SubstrExpr),
    Trim(TrimExpr),
    Replace(ReplaceExpr),
    StartsWith(StartsWithExpr),
    StrPos(StrPosExpr),
    SplitPart(SplitPartExpr),
    Pad(PadExpr),
    QuoteIdent(QuoteIdentExpr),
    SizePretty(SizePrettyExpr),
    Abs(AbsExpr),
    Round(RoundExpr),
    RoundDown(RoundDownExpr),
    RoundUp(RoundUpExpr),
    RoundHalfEven(RoundHalfEvenExpr),
    Coalesce(CoalesceExpr),
    Floor(FloorExpr),
    Ceil(CeilExpr),
    Sign(SignExpr),
    Sqrt(SqrtExpr),
    Power(PowerExpr),
    Ln(LnExpr),
    Log(LogExpr),
    Exp(ExpExpr),
    Pi(PiExpr),
    Greatest(GreatestExpr),
    Least(LeastExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub bytes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpperExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `left` or `right`: a text and a character count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub from_right: bool,
}

/// `substring(text, start, length)`. `start` is 1-based.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubstrExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// Which ends `trim`, `ltrim` and `rtrim` clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimEnd {
    Both,
    Left,
    Right,
}

/// `trim`, `ltrim` or `rtrim`: one text argument, ASCII spaces removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub end: TrimEnd,
}

/// `replace(value, old, new)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `starts_with(text, prefix)` returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartsWithExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `strpos` or `position`: 1-based index of the first match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrPosExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `split_part(text, delimiter, index)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPartExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `lpad` or `rpad`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub from_right: bool,
}

/// `quote_ident(text)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteIdentExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `size_pretty(bytes)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizePrettyExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `abs(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `round(number[, scale])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `round_down(number, scale)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundDownExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `round_up(number, scale)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundUpExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `round_half_even(number, scale)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundHalfEvenExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `coalesce(value, value, ...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoalesceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `floor(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FloorExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `ceil(number)` and its alias `ceiling(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CeilExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `sign(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `sqrt(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqrtExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `power(base, exponent)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `ln(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LnExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `log(number)`, base 10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `exp(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `pi()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `greatest(number, ...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreatestExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `least(number, ...)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeastExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcatExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcatArg {
    Column(String),
    /// `NULL` contributes nothing. A string literal contributes its text.
    Literal(Scalar),
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
    let mut order_by = Vec::new();
    if parser.eat_keyword("where") {
        filter = Some(parser.parse_filter()?);
        parser.skip();
    }
    let mut group_by = None;
    if parser.eat_keyword("group") {
        parser.expect_keyword("by")?;
        let column = parser.ident()?;
        parser.skip();
        if parser.peek() == Some(b',') {
            return Err(Error::new("unsupported: multiple GROUP BY columns"));
        }
        group_by = Some(column);
    }
    if parser.eat_keyword("order") {
        parser.expect_keyword("by")?;
        loop {
            parser.skip();
            let column = if parser.starts_concat_call() {
                parser.parse_concat()?.label
            } else if parser.starts_length_call() {
                parser.parse_length()?.label
            } else if parser.starts_lower_call() {
                parser.parse_lower()?.label
            } else if parser.starts_upper_call() {
                parser.parse_upper()?.label
            } else if parser.starts_side_call() {
                parser.parse_side()?.label
            } else if parser.starts_substr_call() {
                parser.parse_substr()?.label
            } else if parser.starts_trim_call() {
                parser.parse_trim()?.label
            } else if parser.starts_replace_call() {
                parser.parse_replace()?.label
            } else if parser.starts_starts_with_call() {
                parser.parse_starts_with()?.label
            } else if parser.starts_strpos_call() {
                parser.parse_strpos()?.label
            } else if parser.starts_split_part_call() {
                parser.parse_split_part()?.label
            } else if parser.starts_pad_call() {
                parser.parse_pad()?.label
            } else if parser.starts_quote_ident_call() {
                parser.parse_quote_ident()?.label
            } else if parser.starts_size_pretty_call() {
                parser.parse_size_pretty()?.label
            } else if parser.starts_abs_call() {
                parser.parse_abs()?.label
            } else if parser.starts_round_down_call() {
                parser.parse_round_down()?.label
            } else if parser.starts_round_up_call() {
                parser.parse_round_up()?.label
            } else if parser.starts_round_half_even_call() {
                parser.parse_round_half_even()?.label
            } else if parser.starts_coalesce_call() {
                parser.parse_coalesce()?.label
            } else if parser.starts_round_call() {
                parser.parse_round()?.label
            } else if parser.starts_floor_call() {
                parser.parse_floor()?.label
            } else if parser.starts_ceil_call() {
                parser.parse_ceil()?.label
            } else if parser.starts_sign_call() {
                parser.parse_sign()?.label
            } else if parser.starts_sqrt_call() {
                parser.parse_sqrt()?.label
            } else if parser.starts_power_call() {
                parser.parse_power()?.label
            } else if parser.starts_ln_call() {
                parser.parse_ln()?.label
            } else if parser.starts_log_call() {
                parser.parse_log()?.label
            } else if parser.starts_exp_call() {
                parser.parse_exp()?.label
            } else if parser.starts_pi_call() {
                parser.parse_pi()?.label
            } else if parser.starts_greatest_call() {
                parser.parse_greatest()?.label
            } else if parser.starts_least_call() {
                parser.parse_least()?.label
            } else {
                let mut column = parser.ident()?;
                parser.skip();
                if parser.peek() == Some(b'(') {
                    column = parser.parse_order_call(column)?;
                    parser.skip();
                }
                column
            };
            let descending = if parser.eat_keyword("desc") {
                parser.skip();
                true
            } else if parser.eat_keyword("asc") {
                parser.skip();
                false
            } else {
                false
            };
            order_by.push(OrderBy { column, descending });
            parser.skip();
            if !parser.eat(b',') {
                break;
            }
        }
    }
    let mut limit = None;
    if parser.eat_keyword("limit") {
        limit = Some(parser.parse_limit()?);
        parser.skip();
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
        group_by,
        order_by,
        limit,
    })
}

enum SelectPiece {
    Column(String),
    Aggregate(AggregateExpr),
    Concat(ConcatExpr),
    Length(LengthExpr),
    Lower(LowerExpr),
    Upper(UpperExpr),
    Side(SideExpr),
    Substr(SubstrExpr),
    Trim(TrimExpr),
    Replace(ReplaceExpr),
    StartsWith(StartsWithExpr),
    StrPos(StrPosExpr),
    SplitPart(SplitPartExpr),
    Pad(PadExpr),
    QuoteIdent(QuoteIdentExpr),
    SizePretty(SizePrettyExpr),
    Abs(AbsExpr),
    Round(RoundExpr),
    RoundDown(RoundDownExpr),
    RoundUp(RoundUpExpr),
    RoundHalfEven(RoundHalfEvenExpr),
    Coalesce(CoalesceExpr),
    Floor(FloorExpr),
    Ceil(CeilExpr),
    Sign(SignExpr),
    Sqrt(SqrtExpr),
    Power(PowerExpr),
    Ln(LnExpr),
    Log(LogExpr),
    Exp(ExpExpr),
    Pi(PiExpr),
    Greatest(GreatestExpr),
    Least(LeastExpr),
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
        if !self.starts_value() {
            return Err(Error::new(
                "unsupported: SELECT list must name columns or `*`",
            ));
        }
        let mut pieces = Vec::new();
        loop {
            self.skip();
            if self.starts_aggregate_call() {
                pieces.push(SelectPiece::Aggregate(self.parse_aggregate()?));
            } else if self.starts_concat_call() {
                pieces.push(SelectPiece::Concat(self.parse_concat()?));
            } else if self.starts_length_call() {
                pieces.push(SelectPiece::Length(self.parse_length()?));
            } else if self.starts_lower_call() {
                pieces.push(SelectPiece::Lower(self.parse_lower()?));
            } else if self.starts_upper_call() {
                pieces.push(SelectPiece::Upper(self.parse_upper()?));
            } else if self.starts_side_call() {
                pieces.push(SelectPiece::Side(self.parse_side()?));
            } else if self.starts_substr_call() {
                pieces.push(SelectPiece::Substr(self.parse_substr()?));
            } else if self.starts_trim_call() {
                pieces.push(SelectPiece::Trim(self.parse_trim()?));
            } else if self.starts_replace_call() {
                pieces.push(SelectPiece::Replace(self.parse_replace()?));
            } else if self.starts_starts_with_call() {
                pieces.push(SelectPiece::StartsWith(self.parse_starts_with()?));
            } else if self.starts_strpos_call() {
                pieces.push(SelectPiece::StrPos(self.parse_strpos()?));
            } else if self.starts_split_part_call() {
                pieces.push(SelectPiece::SplitPart(self.parse_split_part()?));
            } else if self.starts_pad_call() {
                pieces.push(SelectPiece::Pad(self.parse_pad()?));
            } else if self.starts_quote_ident_call() {
                pieces.push(SelectPiece::QuoteIdent(self.parse_quote_ident()?));
            } else if self.starts_size_pretty_call() {
                pieces.push(SelectPiece::SizePretty(self.parse_size_pretty()?));
            } else if self.starts_abs_call() {
                pieces.push(SelectPiece::Abs(self.parse_abs()?));
            } else if self.starts_round_down_call() {
                pieces.push(SelectPiece::RoundDown(self.parse_round_down()?));
            } else if self.starts_round_up_call() {
                pieces.push(SelectPiece::RoundUp(self.parse_round_up()?));
            } else if self.starts_round_half_even_call() {
                pieces.push(SelectPiece::RoundHalfEven(self.parse_round_half_even()?));
            } else if self.starts_coalesce_call() {
                pieces.push(SelectPiece::Coalesce(self.parse_coalesce()?));
            } else if self.starts_round_call() {
                pieces.push(SelectPiece::Round(self.parse_round()?));
            } else if self.starts_floor_call() {
                pieces.push(SelectPiece::Floor(self.parse_floor()?));
            } else if self.starts_ceil_call() {
                pieces.push(SelectPiece::Ceil(self.parse_ceil()?));
            } else if self.starts_sign_call() {
                pieces.push(SelectPiece::Sign(self.parse_sign()?));
            } else if self.starts_sqrt_call() {
                pieces.push(SelectPiece::Sqrt(self.parse_sqrt()?));
            } else if self.starts_power_call() {
                pieces.push(SelectPiece::Power(self.parse_power()?));
            } else if self.starts_ln_call() {
                pieces.push(SelectPiece::Ln(self.parse_ln()?));
            } else if self.starts_log_call() {
                pieces.push(SelectPiece::Log(self.parse_log()?));
            } else if self.starts_exp_call() {
                pieces.push(SelectPiece::Exp(self.parse_exp()?));
            } else if self.starts_pi_call() {
                pieces.push(SelectPiece::Pi(self.parse_pi()?));
            } else if self.starts_greatest_call() {
                pieces.push(SelectPiece::Greatest(self.parse_greatest()?));
            } else if self.starts_least_call() {
                pieces.push(SelectPiece::Least(self.parse_least()?));
            } else {
                pieces.push(SelectPiece::Column(self.ident_spelling()?));
            }
            self.skip();
            if !self.eat(b',') {
                break;
            }
        }
        let counts = pieces
            .iter()
            .filter(|piece| matches!(piece, SelectPiece::Aggregate(_)))
            .count();
        let projected = pieces.iter().any(|piece| {
            matches!(
                piece,
                SelectPiece::Concat(_)
                    | SelectPiece::Length(_)
                    | SelectPiece::Lower(_)
                    | SelectPiece::Upper(_)
                    | SelectPiece::Side(_)
                    | SelectPiece::Substr(_)
                    | SelectPiece::Trim(_)
                    | SelectPiece::Replace(_)
                    | SelectPiece::StartsWith(_)
                    | SelectPiece::StrPos(_)
                    | SelectPiece::SplitPart(_)
                    | SelectPiece::Pad(_)
                    | SelectPiece::QuoteIdent(_)
                    | SelectPiece::SizePretty(_)
                    | SelectPiece::Abs(_)
                    | SelectPiece::Round(_)
                    | SelectPiece::RoundDown(_)
                    | SelectPiece::RoundUp(_)
                    | SelectPiece::RoundHalfEven(_)
                    | SelectPiece::Coalesce(_)
                    | SelectPiece::Floor(_)
                    | SelectPiece::Ceil(_)
                    | SelectPiece::Sign(_)
                    | SelectPiece::Sqrt(_)
                    | SelectPiece::Power(_)
                    | SelectPiece::Ln(_)
                    | SelectPiece::Log(_)
                    | SelectPiece::Exp(_)
                    | SelectPiece::Pi(_)
                    | SelectPiece::Greatest(_)
                    | SelectPiece::Least(_)
            )
        });
        if counts > 0 && projected {
            return Err(Error::new(
                "unsupported: aggregate mixed with other select expressions",
            ));
        }
        if counts == 0 && projected {
            let mut items = Vec::with_capacity(pieces.len());
            for piece in pieces {
                match piece {
                    SelectPiece::Column(name) => {
                        items.push(Proj::Column(name.to_ascii_lowercase()))
                    }
                    SelectPiece::Concat(expr) => items.push(Proj::Concat(expr)),
                    SelectPiece::Length(expr) => items.push(Proj::Length(expr)),
                    SelectPiece::Lower(expr) => items.push(Proj::Lower(expr)),
                    SelectPiece::Upper(expr) => items.push(Proj::Upper(expr)),
                    SelectPiece::Side(expr) => items.push(Proj::Side(expr)),
                    SelectPiece::Substr(expr) => items.push(Proj::Substr(expr)),
                    SelectPiece::Trim(expr) => items.push(Proj::Trim(expr)),
                    SelectPiece::Replace(expr) => items.push(Proj::Replace(expr)),
                    SelectPiece::StartsWith(expr) => items.push(Proj::StartsWith(expr)),
                    SelectPiece::StrPos(expr) => items.push(Proj::StrPos(expr)),
                    SelectPiece::SplitPart(expr) => items.push(Proj::SplitPart(expr)),
                    SelectPiece::Pad(expr) => items.push(Proj::Pad(expr)),
                    SelectPiece::QuoteIdent(expr) => items.push(Proj::QuoteIdent(expr)),
                    SelectPiece::SizePretty(expr) => items.push(Proj::SizePretty(expr)),
                    SelectPiece::Abs(expr) => items.push(Proj::Abs(expr)),
                    SelectPiece::Round(expr) => items.push(Proj::Round(expr)),
                    SelectPiece::RoundDown(expr) => items.push(Proj::RoundDown(expr)),
                    SelectPiece::RoundUp(expr) => items.push(Proj::RoundUp(expr)),
                    SelectPiece::RoundHalfEven(expr) => items.push(Proj::RoundHalfEven(expr)),
                    SelectPiece::Coalesce(expr) => items.push(Proj::Coalesce(expr)),
                    SelectPiece::Floor(expr) => items.push(Proj::Floor(expr)),
                    SelectPiece::Ceil(expr) => items.push(Proj::Ceil(expr)),
                    SelectPiece::Sign(expr) => items.push(Proj::Sign(expr)),
                    SelectPiece::Sqrt(expr) => items.push(Proj::Sqrt(expr)),
                    SelectPiece::Power(expr) => items.push(Proj::Power(expr)),
                    SelectPiece::Ln(expr) => items.push(Proj::Ln(expr)),
                    SelectPiece::Log(expr) => items.push(Proj::Log(expr)),
                    SelectPiece::Exp(expr) => items.push(Proj::Exp(expr)),
                    SelectPiece::Pi(expr) => items.push(Proj::Pi(expr)),
                    SelectPiece::Greatest(expr) => items.push(Proj::Greatest(expr)),
                    SelectPiece::Least(expr) => items.push(Proj::Least(expr)),
                    SelectPiece::Aggregate(_) => {
                        return Err(Error::new("internal: aggregate piece"));
                    }
                }
            }
            return Ok(SelectList::Projection(items));
        }
        if counts == 0 {
            let mut columns = Vec::with_capacity(pieces.len());
            for piece in pieces {
                let SelectPiece::Column(name) = piece else {
                    return Err(Error::new("internal: column piece"));
                };
                columns.push(name.to_ascii_lowercase());
            }
            return Ok(SelectList::Columns(columns));
        }
        if counts != 1 || pieces.len() > 2 {
            return Err(Error::new(
                "unsupported: aggregate mixed with other select expressions",
            ));
        }
        if pieces.len() == 1 {
            let Some(SelectPiece::Aggregate(aggregate)) = pieces.pop() else {
                return Err(Error::new("internal: aggregate piece"));
            };
            return Ok(SelectList::Aggregate(aggregate));
        }
        let mut key = None;
        let mut key_at = 0;
        let mut aggregate = None;
        for (index, piece) in pieces.into_iter().enumerate() {
            match piece {
                SelectPiece::Column(name) => {
                    key_at = index;
                    key = Some(name);
                }
                SelectPiece::Aggregate(expr) => aggregate = Some(expr),
                SelectPiece::Concat(_)
                | SelectPiece::Length(_)
                | SelectPiece::Lower(_)
                | SelectPiece::Upper(_)
                | SelectPiece::Side(_)
                | SelectPiece::Substr(_)
                | SelectPiece::Trim(_)
                | SelectPiece::Replace(_)
                | SelectPiece::StartsWith(_)
                | SelectPiece::StrPos(_)
                | SelectPiece::SplitPart(_)
                | SelectPiece::Pad(_)
                | SelectPiece::QuoteIdent(_)
                | SelectPiece::SizePretty(_)
                | SelectPiece::Abs(_)
                | SelectPiece::Round(_)
                | SelectPiece::RoundDown(_)
                | SelectPiece::RoundUp(_)
                | SelectPiece::RoundHalfEven(_)
                | SelectPiece::Coalesce(_)
                | SelectPiece::Floor(_)
                | SelectPiece::Ceil(_)
                | SelectPiece::Sign(_)
                | SelectPiece::Sqrt(_)
                | SelectPiece::Power(_)
                | SelectPiece::Ln(_)
                | SelectPiece::Log(_)
                | SelectPiece::Exp(_)
                | SelectPiece::Pi(_)
                | SelectPiece::Greatest(_)
                | SelectPiece::Least(_) => {
                    return Err(Error::new("internal: projection piece"));
                }
            }
        }
        let (Some(key), Some(aggregate)) = (key, aggregate) else {
            return Err(Error::new(
                "unsupported: aggregate mixed with other select expressions",
            ));
        };
        Ok(SelectList::Grouped(GroupedSelect {
            key,
            aggregate,
            key_at,
        }))
    }

    fn starts_value(&self) -> bool {
        matches!(self.peek(), Some(byte) if byte.is_ascii_alphabetic() || byte == b'_')
    }

    fn starts_concat_call(&self) -> bool {
        self.starts_named_call("concat")
    }

    fn starts_length_call(&self) -> bool {
        self.starts_named_call("length") || self.starts_named_call("length_bytes")
    }

    fn starts_lower_call(&self) -> bool {
        self.starts_named_call("lower") || self.starts_named_call("to_lowercase")
    }

    fn starts_upper_call(&self) -> bool {
        self.starts_named_call("upper") || self.starts_named_call("to_uppercase")
    }

    fn starts_side_call(&self) -> bool {
        self.starts_named_call("left") || self.starts_named_call("right")
    }

    fn starts_substr_call(&self) -> bool {
        self.starts_named_call("substring")
    }

    fn starts_least_call(&self) -> bool {
        self.starts_named_call("least")
    }

    fn parse_least(&mut self) -> Result<LeastExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "at least one argument is required by LEAST(V)".to_string(),
            ));
        }
        Ok(LeastExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_greatest_call(&self) -> bool {
        self.starts_named_call("greatest")
    }

    fn parse_greatest(&mut self) -> Result<GreatestExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "at least one argument is required by GREATEST(V)".to_string(),
            ));
        }
        Ok(GreatestExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_pi_call(&self) -> bool {
        self.starts_named_call("pi")
    }

    fn parse_pi(&mut self) -> Result<PiExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(PiExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_exp_call(&self) -> bool {
        self.starts_named_call("exp")
    }

    fn parse_exp(&mut self) -> Result<ExpExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `exp` requires arguments: exp(DOUBLE)".to_string(),
            ));
        }
        Ok(ExpExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_log_call(&self) -> bool {
        self.starts_named_call("log")
    }

    fn parse_log(&mut self) -> Result<LogExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `log` requires arguments: log(DOUBLE)".to_string(),
            ));
        }
        Ok(LogExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_ln_call(&self) -> bool {
        self.starts_named_call("ln")
    }

    fn parse_ln(&mut self) -> Result<LnExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `ln` requires arguments: ln(DOUBLE)".to_string(),
            ));
        }
        Ok(LnExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_power_call(&self) -> bool {
        self.starts_named_call("power")
    }

    fn parse_power(&mut self) -> Result<PowerExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `power` requires arguments: power(DOUBLE, DOUBLE)".to_string(),
            ));
        }
        Ok(PowerExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_sqrt_call(&self) -> bool {
        self.starts_named_call("sqrt")
    }

    fn parse_sqrt(&mut self) -> Result<SqrtExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `sqrt` requires arguments: sqrt(DOUBLE)".to_string(),
            ));
        }
        Ok(SqrtExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_sign_call(&self) -> bool {
        self.starts_named_call("sign")
    }

    fn parse_sign(&mut self) -> Result<SignExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(SignExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_ceil_call(&self) -> bool {
        self.starts_named_call("ceil") || self.starts_named_call("ceiling")
    }

    fn parse_ceil(&mut self) -> Result<CeilExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(CeilExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_floor_call(&self) -> bool {
        self.starts_named_call("floor")
    }

    fn parse_floor(&mut self) -> Result<FloorExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(FloorExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_coalesce_call(&self) -> bool {
        self.starts_named_call("coalesce")
    }

    fn parse_coalesce(&mut self) -> Result<CoalesceExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.len() < 2 {
            return Err(Error::new(
                "coalesce can be used with 2 or more arguments".to_string(),
            ));
        }
        Ok(CoalesceExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_round_half_even_call(&self) -> bool {
        self.starts_named_call("round_half_even")
    }

    fn parse_round_half_even(&mut self) -> Result<RoundHalfEvenExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `round_half_even` requires arguments".to_string(),
            ));
        }
        Ok(RoundHalfEvenExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_round_up_call(&self) -> bool {
        self.starts_named_call("round_up")
    }

    fn parse_round_up(&mut self) -> Result<RoundUpExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `round_up` requires arguments".to_string(),
            ));
        }
        Ok(RoundUpExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_round_down_call(&self) -> bool {
        self.starts_named_call("round_down")
    }

    fn parse_round_down(&mut self) -> Result<RoundDownExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `round_down` requires arguments".to_string(),
            ));
        }
        Ok(RoundDownExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_round_call(&self) -> bool {
        self.starts_named_call("round")
    }

    fn parse_round(&mut self) -> Result<RoundExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(RoundExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_abs_call(&self) -> bool {
        self.starts_named_call("abs")
    }

    fn parse_abs(&mut self) -> Result<AbsExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(AbsExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_size_pretty_call(&self) -> bool {
        self.starts_named_call("size_pretty")
    }

    fn parse_size_pretty(&mut self) -> Result<SizePrettyExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `size_pretty` requires arguments: size_pretty(LONG)".to_string(),
            ));
        }
        Ok(SizePrettyExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_quote_ident_call(&self) -> bool {
        self.starts_named_call("quote_ident")
    }

    fn parse_quote_ident(&mut self) -> Result<QuoteIdentExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(QuoteIdentExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_pad_call(&self) -> bool {
        self.starts_named_call("lpad") || self.starts_named_call("rpad")
    }

    fn parse_pad(&mut self) -> Result<PadExpr, Error> {
        let name = self.ident_spelling()?;
        let from_right = name.eq_ignore_ascii_case("rpad");
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(PadExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            from_right,
        })
    }

    fn starts_split_part_call(&self) -> bool {
        self.starts_named_call("split_part")
    }

    fn parse_split_part(&mut self) -> Result<SplitPartExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(SplitPartExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_strpos_call(&self) -> bool {
        self.starts_named_call("strpos") || self.starts_named_call("position")
    }

    fn parse_strpos(&mut self) -> Result<StrPosExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(StrPosExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_starts_with_call(&self) -> bool {
        self.starts_named_call("starts_with")
    }

    fn parse_starts_with(&mut self) -> Result<StartsWithExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(StartsWithExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_replace_call(&self) -> bool {
        self.starts_named_call("replace")
    }

    fn parse_replace(&mut self) -> Result<ReplaceExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(ReplaceExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_trim_call(&self) -> bool {
        self.starts_named_call("ltrim")
            || self.starts_named_call("rtrim")
            || self.starts_named_call("trim")
    }

    fn parse_trim(&mut self) -> Result<TrimExpr, Error> {
        let name = self.ident_spelling()?;
        let end = if name.eq_ignore_ascii_case("ltrim") {
            TrimEnd::Left
        } else if name.eq_ignore_ascii_case("rtrim") {
            TrimEnd::Right
        } else {
            TrimEnd::Both
        };
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(TrimExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            end,
        })
    }

    fn parse_substr(&mut self) -> Result<SubstrExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(SubstrExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_side(&mut self) -> Result<SideExpr, Error> {
        let name = self.ident_spelling()?;
        let from_right = name.eq_ignore_ascii_case("right");
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(SideExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            from_right,
        })
    }

    fn parse_upper(&mut self) -> Result<UpperExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(UpperExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_lower(&mut self) -> Result<LowerExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(LowerExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_length(&mut self) -> Result<LengthExpr, Error> {
        let name = self.ident_spelling()?;
        let bytes = name.eq_ignore_ascii_case("length_bytes");
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            let message = if bytes {
                "function `length_bytes` requires arguments: length_bytes(VARCHAR)".to_string()
            } else {
                format!(
                    "function `{}` requires arguments",
                    name.to_ascii_lowercase()
                )
            };
            return Err(Error::new(message));
        }
        Ok(LengthExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            bytes,
        })
    }

    fn parse_call_args(
        &mut self,
        name: &str,
        numbers: bool,
    ) -> Result<(Vec<String>, Vec<ConcatArg>), Error> {
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(format!(
                "unsupported: expected '(' after {name}"
            )));
        }
        self.skip();
        if self.peek() == Some(b')') {
            self.pos += 1;
            return Ok((Vec::new(), Vec::new()));
        }
        let mut args = Vec::new();
        let mut labels = Vec::new();
        loop {
            let (label, arg) = self.parse_call_arg(numbers)?;
            labels.push(label);
            args.push(arg);
            self.skip();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b')') {
                break;
            }
            return Err(Error::new(format!(
                "unsupported: expected ')' after {name}"
            )));
        }
        Ok((labels, args))
    }

    fn parse_call_arg(&mut self, numbers: bool) -> Result<(String, ConcatArg), Error> {
        self.skip();
        if self.peek() == Some(b'\'') {
            let start = self.pos;
            let text = self.parse_string()?;
            let label = std::str::from_utf8(&self.src[start..self.pos])
                .unwrap()
                .to_string();
            return Ok((label, ConcatArg::Literal(Scalar::Text(text))));
        }
        if self
            .peek()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        {
            let spelling = self.ident_spelling()?;
            let arg = if spelling.eq_ignore_ascii_case("null") {
                ConcatArg::Literal(Scalar::Null)
            } else {
                ConcatArg::Column(spelling.clone())
            };
            return Ok((spelling, arg));
        }
        if numbers
            && self
                .peek()
                .is_some_and(|byte| byte.is_ascii_digit() || byte == b'-')
        {
            let start = self.pos;
            let value = self.parse_number()?;
            let raw = std::str::from_utf8(&self.src[start..self.pos])
                .unwrap()
                .to_string();
            let label = match &value {
                Scalar::Long(number) if *number < 0 => format!("-({})", number.unsigned_abs()),
                Scalar::Double(number) if number.is_sign_negative() => {
                    format!("-({})", raw.trim_start_matches('-'))
                }
                _ => raw,
            };
            return Ok((label, ConcatArg::Literal(value)));
        }
        Err(Error::new(format!(
            "unsupported: function argument near `{}`",
            self.rest()
        )))
    }

    fn starts_named_call(&self, expected: &str) -> bool {
        let bytes = &self.src[self.pos..];
        let Some(first) = bytes.first() else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let mut index = 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            index += 1;
        }
        let name = std::str::from_utf8(&bytes[..index]).unwrap_or("");
        if !name.eq_ignore_ascii_case(expected) {
            return false;
        }
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        bytes.get(index) == Some(&b'(')
    }

    fn parse_concat(&mut self) -> Result<ConcatExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new("no arguments provided"));
        }
        Ok(ConcatExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_aggregate_call(&self) -> bool {
        let bytes = &self.src[self.pos..];
        let Some(first) = bytes.first() else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let mut index = 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            index += 1;
        }
        let name = std::str::from_utf8(&bytes[..index]).unwrap_or("");
        if !matches!(
            name.to_ascii_lowercase().as_str(),
            "count" | "min" | "max" | "sum" | "avg" | "first" | "last" | "count_distinct"
        ) {
            return false;
        }
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        bytes.get(index) == Some(&b'(')
    }

    fn parse_aggregate(&mut self) -> Result<AggregateExpr, Error> {
        let name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(format!(
                "unsupported: expected '(' after {name}"
            )));
        }
        self.skip();
        let starred = self.eat(b'*');
        let column = if starred || self.peek() == Some(b')') {
            None
        } else {
            Some(self.ident_spelling()?)
        };
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new(format!(
                "unsupported: expected ')' after {name}"
            )));
        }
        let func = match name.to_ascii_lowercase().as_str() {
            "count" => AggFunc::Count,
            "min" => AggFunc::Min,
            "max" => AggFunc::Max,
            "sum" => AggFunc::Sum,
            "avg" => AggFunc::Avg,
            "first" => AggFunc::First,
            "last" => AggFunc::Last,
            "count_distinct" => AggFunc::CountDistinct,
            _ => return Err(Error::new(format!("internal: aggregate `{name}`"))),
        };
        if func != AggFunc::Count && starred {
            return Err(Error::new("too few arguments for '*' [found=0,expected=2]"));
        }
        if func != AggFunc::Count && column.is_none() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        let label = match &column {
            Some(column) => format!("{name}({column})"),
            None => format!("{name}()"),
        };
        Ok(AggregateExpr {
            func,
            label,
            column,
        })
    }

    fn parse_order_call(&mut self, name: String) -> Result<String, Error> {
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new("unsupported: expected '(' after ORDER BY"));
        }
        self.skip();
        let argument = if self.eat(b'*') || self.peek() == Some(b')') {
            None
        } else {
            Some(self.ident()?)
        };
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new("unsupported: expected ')' after ORDER BY"));
        }
        Ok(match argument {
            Some(argument) => format!("{name}({argument})"),
            None => format!("{name}()"),
        })
    }

    fn ident_spelling(&mut self) -> Result<String, Error> {
        self.skip();
        let start = self.pos;
        let first = self
            .peek()
            .ok_or_else(|| Error::new(format!("expected identifier near `{}`", self.rest())))?;
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Err(Error::new(format!(
                "expected identifier near `{}`",
                self.rest()
            )));
        }
        self.pos += 1;
        while matches!(self.peek(), Some(byte) if byte.is_ascii_alphanumeric() || byte == b'_') {
            self.pos += 1;
        }
        Ok(std::str::from_utf8(&self.src[start..self.pos])
            .unwrap()
            .to_string())
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

    fn parse_limit(&mut self) -> Result<Limit, Error> {
        self.skip();
        let lo_present = self.starts_limit_int();
        let lo = if lo_present {
            self.parse_limit_int()?
        } else {
            0
        };
        self.skip();
        if !self.eat(b',') {
            if !lo_present {
                return Err(Error::new(
                    "unsupported: LIMIT requires an integer constant",
                ));
            }
            return Ok(single_limit(lo));
        }
        self.skip();
        if self.starts_limit_int() {
            let hi = self.parse_limit_int()?;
            return Ok(Limit { lo, hi });
        }
        if !lo_present {
            return Err(Error::new("unsupported: limit expression expected"));
        }
        // `LIMIT n,` is the single-argument form.
        Ok(single_limit(lo))
    }

    fn starts_limit_int(&self) -> bool {
        match self.peek() {
            Some(byte) if byte.is_ascii_digit() => true,
            Some(b'-') => self
                .src
                .get(self.pos + 1)
                .is_some_and(|byte| byte.is_ascii_digit()),
            _ => false,
        }
    }

    fn parse_limit_int(&mut self) -> Result<i64, Error> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        if !matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
            return Err(Error::new(
                "unsupported: LIMIT requires an integer constant",
            ));
        }
        while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
            self.pos += 1;
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(Error::new(
                "unsupported: LIMIT requires an integer constant",
            ));
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        text.parse::<i64>().map_err(|_| {
            Error::new(format!(
                "unsupported: LIMIT integer `{text}` is out of range"
            ))
        })
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

fn single_limit(n: i64) -> Limit {
    if n >= 0 {
        Limit { lo: 0, hi: n }
    } else {
        Limit { lo: n, hi: 0 }
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
        assert_eq!(
            query.order_by,
            vec![OrderBy {
                column: "ts".to_string(),
                descending: false,
            }]
        );
        let descending = parse_query("SELECT id FROM sensors ORDER BY location DESC, id").unwrap();
        assert_eq!(
            descending.order_by,
            vec![
                OrderBy {
                    column: "location".to_string(),
                    descending: true,
                },
                OrderBy {
                    column: "id".to_string(),
                    descending: false,
                },
            ]
        );
        assert_eq!(query.limit, None);
        let limited = parse_query("SELECT id FROM sensors ORDER BY id LIMIT -2").unwrap();
        assert_eq!(limited.limit, Some(Limit { lo: -2, hi: 0 }));
        let range = parse_query("SELECT id FROM sensors ORDER BY id LIMIT 3, 1").unwrap();
        assert_eq!(range.limit, Some(Limit { lo: 3, hi: 1 }));
        let open = parse_query("SELECT id FROM sensors LIMIT ,2").unwrap();
        assert_eq!(open.limit, Some(Limit { lo: 0, hi: 2 }));
        let err = parse_query("SELECT 1").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = parse_query("SELECT id FROM sensors ORDER BY id LIMIT 2.0").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = parse_query("SELECT id FROM sensors ORDER BY id LIMIT ,").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
    }
}
