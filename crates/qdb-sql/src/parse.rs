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
    NullIf(NullIfExpr),
    Cast(CastExpr),
    DateAdd(DateAddExpr),
    DateDiff(DateDiffExpr),
    Year(YearExpr),
    Month(MonthExpr),
    Day(DayExpr),
    Hour(HourExpr),
    Minute(MinuteExpr),
    Second(SecondExpr),
    Millis(MillisExpr),
    Micros(MicrosExpr),
    DayOfWeek(DayOfWeekExpr),
    WeekOfYear(WeekOfYearExpr),
    DateTrunc(DateTruncExpr),
    TimestampFloor(TimestampFloorExpr),
    TimestampCeil(TimestampCeilExpr),
    IsLeapYear(IsLeapYearExpr),
    DaysInMonth(DaysInMonthExpr),
    ToStr(ToStrExpr),
    IsEndOfMonth(IsEndOfMonthExpr),
    DayOfWeekSundayFirst(DayOfWeekSundayFirstExpr),
    ToTimestamp(ToTimestampExpr),
    ToDate(ToDateExpr),
    Md5(Md5Expr),
    Sha1(Sha1Expr),
    Sha256(Sha256Expr),
    Extract(ExtractExpr),
    Floor(FloorExpr),
    Ceil(CeilExpr),
    Sign(SignExpr),
    Sqrt(SqrtExpr),
    Sin(SinExpr),
    Cos(CosExpr),
    Tan(TanExpr),
    Asin(AsinExpr),
    Acos(AcosExpr),
    Atan(AtanExpr),
    Atan2(Atan2Expr),
    Degrees(DegreesExpr),
    Radians(RadiansExpr),
    Cot(CotExpr),
    Typeof(TypeofExpr),
    Not(NotExpr),
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

/// `nullif(value, value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullIfExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `cast(column AS type)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastExpr {
    pub label: String,
    pub column: String,
    pub type_name: String,
}

/// `dateadd(period, stride, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateAddExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `datediff(period, start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateDiffExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `day(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `hour(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `minute(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinuteExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `second(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `millis(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MillisExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `micros(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrosExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `day_of_week(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayOfWeekExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `week_of_year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekOfYearExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `date_trunc(unit, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateTruncExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `timestamp_floor(unit, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampFloorExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `timestamp_ceil(unit, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampCeilExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `is_leap_year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsLeapYearExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `days_in_month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaysInMonthExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_str(timestamp, format)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToStrExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `is_end_of_month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsEndOfMonthExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `day_of_week_sunday_first(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayOfWeekSundayFirstExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_timestamp(text)` parses a microsecond count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToTimestampExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_date(text, pattern)` parses a millisecond date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDateExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `md5(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Md5Expr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `sha1(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha1Expr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `sha256(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Expr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `extract(part FROM timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractExpr {
    pub label: String,
    pub part: String,
    pub arg: ConcatArg,
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

/// `sin(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `cos(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CosExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `tan(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TanExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `asin(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsinExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `acos(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcosExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `atan(number)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtanExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `atan2(y, x)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Atan2Expr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `degrees(radians)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DegreesExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `radians(degrees)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RadiansExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `cot(radians)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CotExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `typeof(value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeofExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `not(value)`. Extra arguments are ignored; the last one is negated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotExpr {
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
            } else if parser.starts_nullif_call() {
                parser.parse_nullif()?.label
            } else if parser.starts_cast_call() {
                parser.parse_cast()?.label
            } else if parser.starts_dateadd_call() {
                parser.parse_dateadd()?.label
            } else if parser.starts_datediff_call() {
                parser.parse_datediff()?.label
            } else if parser.starts_year_call() {
                parser.parse_year()?.label
            } else if parser.starts_month_call() {
                parser.parse_month()?.label
            } else if parser.starts_day_call() {
                parser.parse_day()?.label
            } else if parser.starts_hour_call() {
                parser.parse_hour()?.label
            } else if parser.starts_minute_call() {
                parser.parse_minute()?.label
            } else if parser.starts_second_call() {
                parser.parse_second()?.label
            } else if parser.starts_millis_call() {
                parser.parse_millis()?.label
            } else if parser.starts_micros_call() {
                parser.parse_micros()?.label
            } else if parser.starts_day_of_week_call() {
                parser.parse_day_of_week()?.label
            } else if parser.starts_week_of_year_call() {
                parser.parse_week_of_year()?.label
            } else if parser.starts_date_trunc_call() {
                parser.parse_date_trunc()?.label
            } else if parser.starts_timestamp_floor_call() {
                parser.parse_timestamp_floor()?.label
            } else if parser.starts_timestamp_ceil_call() {
                parser.parse_timestamp_ceil()?.label
            } else if parser.starts_is_leap_year_call() {
                parser.parse_is_leap_year()?.label
            } else if parser.starts_days_in_month_call() {
                parser.parse_days_in_month()?.label
            } else if parser.starts_extract_call() {
                parser.parse_extract()?.label
            } else if parser.starts_sha256_call() {
                parser.parse_sha256()?.label
            } else if parser.starts_sha1_call() {
                parser.parse_sha1()?.label
            } else if parser.starts_md5_call() {
                parser.parse_md5()?.label
            } else if parser.starts_to_date_call() {
                parser.parse_to_date()?.label
            } else if parser.starts_to_timestamp_call() {
                parser.parse_to_timestamp()?.label
            } else if parser.starts_day_of_week_sunday_first_call() {
                parser.parse_day_of_week_sunday_first()?.label
            } else if parser.starts_is_end_of_month_call() {
                parser.parse_is_end_of_month()?.label
            } else if parser.starts_to_str_call() {
                parser.parse_to_str()?.label
            } else if parser.starts_round_call() {
                parser.parse_round()?.label
            } else if parser.starts_floor_call() {
                parser.parse_floor()?.label
            } else if parser.starts_ceil_call() {
                parser.parse_ceil()?.label
            } else if parser.starts_sign_call() {
                parser.parse_sign()?.label
            } else if parser.starts_not_call() {
                parser.parse_not()?.label
            } else if parser.starts_typeof_call() {
                parser.parse_typeof()?.label
            } else if parser.starts_cot_call() {
                parser.parse_cot()?.label
            } else if parser.starts_radians_call() {
                parser.parse_radians()?.label
            } else if parser.starts_degrees_call() {
                parser.parse_degrees()?.label
            } else if parser.starts_atan2_call() {
                parser.parse_atan2()?.label
            } else if parser.starts_atan_call() {
                parser.parse_atan()?.label
            } else if parser.starts_acos_call() {
                parser.parse_acos()?.label
            } else if parser.starts_asin_call() {
                parser.parse_asin()?.label
            } else if parser.starts_tan_call() {
                parser.parse_tan()?.label
            } else if parser.starts_cos_call() {
                parser.parse_cos()?.label
            } else if parser.starts_sin_call() {
                parser.parse_sin()?.label
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
    NullIf(NullIfExpr),
    Cast(CastExpr),
    DateAdd(DateAddExpr),
    DateDiff(DateDiffExpr),
    Year(YearExpr),
    Month(MonthExpr),
    Day(DayExpr),
    Hour(HourExpr),
    Minute(MinuteExpr),
    Second(SecondExpr),
    Millis(MillisExpr),
    Micros(MicrosExpr),
    DayOfWeek(DayOfWeekExpr),
    WeekOfYear(WeekOfYearExpr),
    DateTrunc(DateTruncExpr),
    TimestampFloor(TimestampFloorExpr),
    TimestampCeil(TimestampCeilExpr),
    IsLeapYear(IsLeapYearExpr),
    DaysInMonth(DaysInMonthExpr),
    ToStr(ToStrExpr),
    IsEndOfMonth(IsEndOfMonthExpr),
    DayOfWeekSundayFirst(DayOfWeekSundayFirstExpr),
    ToTimestamp(ToTimestampExpr),
    ToDate(ToDateExpr),
    Md5(Md5Expr),
    Sha1(Sha1Expr),
    Sha256(Sha256Expr),
    Extract(ExtractExpr),
    Floor(FloorExpr),
    Ceil(CeilExpr),
    Sign(SignExpr),
    Sqrt(SqrtExpr),
    Sin(SinExpr),
    Cos(CosExpr),
    Tan(TanExpr),
    Asin(AsinExpr),
    Acos(AcosExpr),
    Atan(AtanExpr),
    Atan2(Atan2Expr),
    Degrees(DegreesExpr),
    Radians(RadiansExpr),
    Cot(CotExpr),
    Typeof(TypeofExpr),
    Not(NotExpr),
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
            } else if self.starts_nullif_call() {
                pieces.push(SelectPiece::NullIf(self.parse_nullif()?));
            } else if self.starts_cast_call() {
                pieces.push(SelectPiece::Cast(self.parse_cast()?));
            } else if self.starts_dateadd_call() {
                pieces.push(SelectPiece::DateAdd(self.parse_dateadd()?));
            } else if self.starts_datediff_call() {
                pieces.push(SelectPiece::DateDiff(self.parse_datediff()?));
            } else if self.starts_year_call() {
                pieces.push(SelectPiece::Year(self.parse_year()?));
            } else if self.starts_month_call() {
                pieces.push(SelectPiece::Month(self.parse_month()?));
            } else if self.starts_day_call() {
                pieces.push(SelectPiece::Day(self.parse_day()?));
            } else if self.starts_hour_call() {
                pieces.push(SelectPiece::Hour(self.parse_hour()?));
            } else if self.starts_minute_call() {
                pieces.push(SelectPiece::Minute(self.parse_minute()?));
            } else if self.starts_second_call() {
                pieces.push(SelectPiece::Second(self.parse_second()?));
            } else if self.starts_millis_call() {
                pieces.push(SelectPiece::Millis(self.parse_millis()?));
            } else if self.starts_micros_call() {
                pieces.push(SelectPiece::Micros(self.parse_micros()?));
            } else if self.starts_day_of_week_call() {
                pieces.push(SelectPiece::DayOfWeek(self.parse_day_of_week()?));
            } else if self.starts_week_of_year_call() {
                pieces.push(SelectPiece::WeekOfYear(self.parse_week_of_year()?));
            } else if self.starts_date_trunc_call() {
                pieces.push(SelectPiece::DateTrunc(self.parse_date_trunc()?));
            } else if self.starts_timestamp_floor_call() {
                pieces.push(SelectPiece::TimestampFloor(self.parse_timestamp_floor()?));
            } else if self.starts_timestamp_ceil_call() {
                pieces.push(SelectPiece::TimestampCeil(self.parse_timestamp_ceil()?));
            } else if self.starts_is_leap_year_call() {
                pieces.push(SelectPiece::IsLeapYear(self.parse_is_leap_year()?));
            } else if self.starts_days_in_month_call() {
                pieces.push(SelectPiece::DaysInMonth(self.parse_days_in_month()?));
            } else if self.starts_extract_call() {
                pieces.push(SelectPiece::Extract(self.parse_extract()?));
            } else if self.starts_sha256_call() {
                pieces.push(SelectPiece::Sha256(self.parse_sha256()?));
            } else if self.starts_sha1_call() {
                pieces.push(SelectPiece::Sha1(self.parse_sha1()?));
            } else if self.starts_md5_call() {
                pieces.push(SelectPiece::Md5(self.parse_md5()?));
            } else if self.starts_to_date_call() {
                pieces.push(SelectPiece::ToDate(self.parse_to_date()?));
            } else if self.starts_to_timestamp_call() {
                pieces.push(SelectPiece::ToTimestamp(self.parse_to_timestamp()?));
            } else if self.starts_day_of_week_sunday_first_call() {
                pieces.push(SelectPiece::DayOfWeekSundayFirst(
                    self.parse_day_of_week_sunday_first()?,
                ));
            } else if self.starts_is_end_of_month_call() {
                pieces.push(SelectPiece::IsEndOfMonth(self.parse_is_end_of_month()?));
            } else if self.starts_to_str_call() {
                pieces.push(SelectPiece::ToStr(self.parse_to_str()?));
            } else if self.starts_round_call() {
                pieces.push(SelectPiece::Round(self.parse_round()?));
            } else if self.starts_floor_call() {
                pieces.push(SelectPiece::Floor(self.parse_floor()?));
            } else if self.starts_ceil_call() {
                pieces.push(SelectPiece::Ceil(self.parse_ceil()?));
            } else if self.starts_sign_call() {
                pieces.push(SelectPiece::Sign(self.parse_sign()?));
            } else if self.starts_not_call() {
                pieces.push(SelectPiece::Not(self.parse_not()?));
            } else if self.starts_typeof_call() {
                pieces.push(SelectPiece::Typeof(self.parse_typeof()?));
            } else if self.starts_cot_call() {
                pieces.push(SelectPiece::Cot(self.parse_cot()?));
            } else if self.starts_radians_call() {
                pieces.push(SelectPiece::Radians(self.parse_radians()?));
            } else if self.starts_degrees_call() {
                pieces.push(SelectPiece::Degrees(self.parse_degrees()?));
            } else if self.starts_atan2_call() {
                pieces.push(SelectPiece::Atan2(self.parse_atan2()?));
            } else if self.starts_atan_call() {
                pieces.push(SelectPiece::Atan(self.parse_atan()?));
            } else if self.starts_acos_call() {
                pieces.push(SelectPiece::Acos(self.parse_acos()?));
            } else if self.starts_asin_call() {
                pieces.push(SelectPiece::Asin(self.parse_asin()?));
            } else if self.starts_tan_call() {
                pieces.push(SelectPiece::Tan(self.parse_tan()?));
            } else if self.starts_cos_call() {
                pieces.push(SelectPiece::Cos(self.parse_cos()?));
            } else if self.starts_sin_call() {
                pieces.push(SelectPiece::Sin(self.parse_sin()?));
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
                    | SelectPiece::NullIf(_)
                    | SelectPiece::Cast(_)
                    | SelectPiece::DateAdd(_)
                    | SelectPiece::DateDiff(_)
                    | SelectPiece::Year(_)
                    | SelectPiece::Month(_)
                    | SelectPiece::Day(_)
                    | SelectPiece::Hour(_)
                    | SelectPiece::Minute(_)
                    | SelectPiece::Second(_)
                    | SelectPiece::Millis(_)
                    | SelectPiece::Micros(_)
                    | SelectPiece::DayOfWeek(_)
                    | SelectPiece::WeekOfYear(_)
                    | SelectPiece::DateTrunc(_)
                    | SelectPiece::TimestampFloor(_)
                    | SelectPiece::TimestampCeil(_)
                    | SelectPiece::IsLeapYear(_)
                    | SelectPiece::DaysInMonth(_)
                    | SelectPiece::ToStr(_)
                    | SelectPiece::IsEndOfMonth(_)
                    | SelectPiece::DayOfWeekSundayFirst(_)
                    | SelectPiece::ToTimestamp(_)
                    | SelectPiece::ToDate(_)
                    | SelectPiece::Md5(_)
                    | SelectPiece::Sha1(_)
                    | SelectPiece::Sha256(_)
                    | SelectPiece::Extract(_)
                    | SelectPiece::Floor(_)
                    | SelectPiece::Ceil(_)
                    | SelectPiece::Sign(_)
                    | SelectPiece::Sqrt(_)
                    | SelectPiece::Sin(_)
                    | SelectPiece::Cos(_)
                    | SelectPiece::Tan(_)
                    | SelectPiece::Asin(_)
                    | SelectPiece::Acos(_)
                    | SelectPiece::Atan(_)
                    | SelectPiece::Atan2(_)
                    | SelectPiece::Degrees(_)
                    | SelectPiece::Radians(_)
                    | SelectPiece::Cot(_)
                    | SelectPiece::Typeof(_)
                    | SelectPiece::Not(_)
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
                    SelectPiece::NullIf(expr) => items.push(Proj::NullIf(expr)),
                    SelectPiece::Cast(expr) => items.push(Proj::Cast(expr)),
                    SelectPiece::DateAdd(expr) => items.push(Proj::DateAdd(expr)),
                    SelectPiece::DateDiff(expr) => items.push(Proj::DateDiff(expr)),
                    SelectPiece::Year(expr) => items.push(Proj::Year(expr)),
                    SelectPiece::Month(expr) => items.push(Proj::Month(expr)),
                    SelectPiece::Day(expr) => items.push(Proj::Day(expr)),
                    SelectPiece::Hour(expr) => items.push(Proj::Hour(expr)),
                    SelectPiece::Minute(expr) => items.push(Proj::Minute(expr)),
                    SelectPiece::Second(expr) => items.push(Proj::Second(expr)),
                    SelectPiece::Millis(expr) => items.push(Proj::Millis(expr)),
                    SelectPiece::Micros(expr) => items.push(Proj::Micros(expr)),
                    SelectPiece::DayOfWeek(expr) => items.push(Proj::DayOfWeek(expr)),
                    SelectPiece::WeekOfYear(expr) => items.push(Proj::WeekOfYear(expr)),
                    SelectPiece::DateTrunc(expr) => items.push(Proj::DateTrunc(expr)),
                    SelectPiece::TimestampFloor(expr) => items.push(Proj::TimestampFloor(expr)),
                    SelectPiece::TimestampCeil(expr) => items.push(Proj::TimestampCeil(expr)),
                    SelectPiece::IsLeapYear(expr) => items.push(Proj::IsLeapYear(expr)),
                    SelectPiece::DaysInMonth(expr) => items.push(Proj::DaysInMonth(expr)),
                    SelectPiece::ToStr(expr) => items.push(Proj::ToStr(expr)),
                    SelectPiece::IsEndOfMonth(expr) => items.push(Proj::IsEndOfMonth(expr)),
                    SelectPiece::DayOfWeekSundayFirst(expr) => {
                        items.push(Proj::DayOfWeekSundayFirst(expr))
                    }
                    SelectPiece::ToTimestamp(expr) => items.push(Proj::ToTimestamp(expr)),
                    SelectPiece::ToDate(expr) => items.push(Proj::ToDate(expr)),
                    SelectPiece::Md5(expr) => items.push(Proj::Md5(expr)),
                    SelectPiece::Sha1(expr) => items.push(Proj::Sha1(expr)),
                    SelectPiece::Sha256(expr) => items.push(Proj::Sha256(expr)),
                    SelectPiece::Extract(expr) => items.push(Proj::Extract(expr)),
                    SelectPiece::Floor(expr) => items.push(Proj::Floor(expr)),
                    SelectPiece::Ceil(expr) => items.push(Proj::Ceil(expr)),
                    SelectPiece::Sign(expr) => items.push(Proj::Sign(expr)),
                    SelectPiece::Sqrt(expr) => items.push(Proj::Sqrt(expr)),
                    SelectPiece::Sin(expr) => items.push(Proj::Sin(expr)),
                    SelectPiece::Cos(expr) => items.push(Proj::Cos(expr)),
                    SelectPiece::Tan(expr) => items.push(Proj::Tan(expr)),
                    SelectPiece::Asin(expr) => items.push(Proj::Asin(expr)),
                    SelectPiece::Acos(expr) => items.push(Proj::Acos(expr)),
                    SelectPiece::Atan(expr) => items.push(Proj::Atan(expr)),
                    SelectPiece::Atan2(expr) => items.push(Proj::Atan2(expr)),
                    SelectPiece::Degrees(expr) => items.push(Proj::Degrees(expr)),
                    SelectPiece::Radians(expr) => items.push(Proj::Radians(expr)),
                    SelectPiece::Cot(expr) => items.push(Proj::Cot(expr)),
                    SelectPiece::Typeof(expr) => items.push(Proj::Typeof(expr)),
                    SelectPiece::Not(expr) => items.push(Proj::Not(expr)),
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
                | SelectPiece::NullIf(_)
                | SelectPiece::Cast(_)
                | SelectPiece::DateAdd(_)
                | SelectPiece::DateDiff(_)
                | SelectPiece::Year(_)
                | SelectPiece::Month(_)
                | SelectPiece::Day(_)
                | SelectPiece::Hour(_)
                | SelectPiece::Minute(_)
                | SelectPiece::Second(_)
                | SelectPiece::Millis(_)
                | SelectPiece::Micros(_)
                | SelectPiece::DayOfWeek(_)
                | SelectPiece::WeekOfYear(_)
                | SelectPiece::DateTrunc(_)
                | SelectPiece::TimestampFloor(_)
                | SelectPiece::TimestampCeil(_)
                | SelectPiece::IsLeapYear(_)
                | SelectPiece::DaysInMonth(_)
                | SelectPiece::ToStr(_)
                | SelectPiece::IsEndOfMonth(_)
                | SelectPiece::DayOfWeekSundayFirst(_)
                | SelectPiece::ToTimestamp(_)
                | SelectPiece::ToDate(_)
                | SelectPiece::Md5(_)
                | SelectPiece::Sha1(_)
                | SelectPiece::Sha256(_)
                | SelectPiece::Extract(_)
                | SelectPiece::Floor(_)
                | SelectPiece::Ceil(_)
                | SelectPiece::Sign(_)
                | SelectPiece::Sqrt(_)
                | SelectPiece::Sin(_)
                | SelectPiece::Cos(_)
                | SelectPiece::Tan(_)
                | SelectPiece::Asin(_)
                | SelectPiece::Acos(_)
                | SelectPiece::Atan(_)
                | SelectPiece::Atan2(_)
                | SelectPiece::Degrees(_)
                | SelectPiece::Radians(_)
                | SelectPiece::Cot(_)
                | SelectPiece::Typeof(_)
                | SelectPiece::Not(_)
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

    fn starts_not_call(&self) -> bool {
        self.starts_named_call("not")
    }

    fn parse_not(&mut self) -> Result<NotExpr, Error> {
        let _name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args("not", true)?;
        if args.is_empty() {
            return Err(Error::new(
                "too few arguments for 'not' [found=0,expected=1]",
            ));
        }
        let last = labels.last().map(String::as_str).unwrap_or("");
        Ok(NotExpr {
            label: format!("not({last})"),
            args,
        })
    }

    fn starts_typeof_call(&self) -> bool {
        self.starts_named_call("typeof")
    }

    fn parse_typeof(&mut self) -> Result<TypeofExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("exactly one argument expected"));
        }
        Ok(TypeofExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_cot_call(&self) -> bool {
        self.starts_named_call("cot")
    }

    fn parse_cot(&mut self) -> Result<CotExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `cot` requires arguments: cot(DOUBLE)".to_string(),
            ));
        }
        Ok(CotExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_radians_call(&self) -> bool {
        self.starts_named_call("radians")
    }

    fn parse_radians(&mut self) -> Result<RadiansExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `radians` requires arguments: radians(DOUBLE)".to_string(),
            ));
        }
        Ok(RadiansExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_degrees_call(&self) -> bool {
        self.starts_named_call("degrees")
    }

    fn parse_degrees(&mut self) -> Result<DegreesExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `degrees` requires arguments: degrees(DOUBLE)".to_string(),
            ));
        }
        Ok(DegreesExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_atan2_call(&self) -> bool {
        self.starts_named_call("atan2")
    }

    fn parse_atan2(&mut self) -> Result<Atan2Expr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `atan2` requires arguments: atan2(DOUBLE, DOUBLE)".to_string(),
            ));
        }
        Ok(Atan2Expr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_atan_call(&self) -> bool {
        self.starts_named_call("atan")
    }

    fn parse_atan(&mut self) -> Result<AtanExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `atan` requires arguments: atan(DOUBLE)".to_string(),
            ));
        }
        Ok(AtanExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_acos_call(&self) -> bool {
        self.starts_named_call("acos")
    }

    fn parse_acos(&mut self) -> Result<AcosExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `acos` requires arguments: acos(DOUBLE)".to_string(),
            ));
        }
        Ok(AcosExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_asin_call(&self) -> bool {
        self.starts_named_call("asin")
    }

    fn parse_asin(&mut self) -> Result<AsinExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `asin` requires arguments: asin(DOUBLE)".to_string(),
            ));
        }
        Ok(AsinExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_tan_call(&self) -> bool {
        self.starts_named_call("tan")
    }

    fn parse_tan(&mut self) -> Result<TanExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `tan` requires arguments: tan(DOUBLE)".to_string(),
            ));
        }
        Ok(TanExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_cos_call(&self) -> bool {
        self.starts_named_call("cos")
    }

    fn parse_cos(&mut self) -> Result<CosExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `cos` requires arguments: cos(DOUBLE)".to_string(),
            ));
        }
        Ok(CosExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_sin_call(&self) -> bool {
        self.starts_named_call("sin")
    }

    fn parse_sin(&mut self) -> Result<SinExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `sin` requires arguments: sin(DOUBLE)".to_string(),
            ));
        }
        Ok(SinExpr {
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

    fn starts_extract_call(&self) -> bool {
        self.starts_named_call("extract")
    }

    fn parse_extract(&mut self) -> Result<ExtractExpr, Error> {
        self.skip();
        let name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(
                "unsupported: expected '(' after extract".to_string(),
            ));
        }
        self.skip();
        if self.eat(b')') {
            return Err(Error::new(
                "function `extract` requires arguments: extract(STRING constant, TIMESTAMP)"
                    .to_string(),
            ));
        }
        self.skip();
        let part_start = self.pos;
        let part = if self.peek() == Some(b'\'') {
            self.parse_string()?
        } else {
            self.ident_spelling()?
        };
        let part_src = std::str::from_utf8(&self.src[part_start..self.pos])
            .unwrap_or(part.as_str())
            .to_string();
        self.skip();
        if self.peek() == Some(b')') {
            return Err(Error::new("missing arguments".to_string()));
        }
        let keyword = self.ident_spelling()?;
        if !keyword.eq_ignore_ascii_case("from") {
            return Err(Error::new("missing arguments".to_string()));
        }
        self.skip();
        if self.peek() == Some(b')') {
            return Err(Error::new("missing arguments".to_string()));
        }
        let (arg_label, arg) = self.parse_call_arg(true)?;
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new("missing arguments".to_string()));
        }
        Ok(ExtractExpr {
            label: format!("{name}({part_src}, {arg_label})"),
            part,
            arg,
        })
    }

    fn starts_sha256_call(&self) -> bool {
        self.starts_named_call("sha256")
    }

    fn parse_sha256(&mut self) -> Result<Sha256Expr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `sha256` requires arguments".to_string(),
            ));
        }
        Ok(Sha256Expr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_sha1_call(&self) -> bool {
        self.starts_named_call("sha1")
    }

    fn parse_sha1(&mut self) -> Result<Sha1Expr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("function `sha1` requires arguments".to_string()));
        }
        Ok(Sha1Expr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_md5_call(&self) -> bool {
        self.starts_named_call("md5")
    }

    fn parse_md5(&mut self) -> Result<Md5Expr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("function `md5` requires arguments".to_string()));
        }
        Ok(Md5Expr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_to_date_call(&self) -> bool {
        self.starts_named_call("to_date")
    }

    fn parse_to_date(&mut self) -> Result<ToDateExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `to_date` requires arguments".to_string(),
            ));
        }
        Ok(ToDateExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_to_timestamp_call(&self) -> bool {
        self.starts_named_call("to_timestamp")
    }

    fn parse_to_timestamp(&mut self) -> Result<ToTimestampExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `to_timestamp` requires arguments".to_string(),
            ));
        }
        Ok(ToTimestampExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_day_of_week_sunday_first_call(&self) -> bool {
        self.starts_named_call("day_of_week_sunday_first")
    }

    fn parse_day_of_week_sunday_first(&mut self) -> Result<DayOfWeekSundayFirstExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `day_of_week_sunday_first` requires arguments: day_of_week_sunday_first(TIMESTAMP)".to_string(),
            ));
        }
        Ok(DayOfWeekSundayFirstExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_is_end_of_month_call(&self) -> bool {
        self.starts_named_call("is_end_of_month")
    }

    fn parse_is_end_of_month(&mut self) -> Result<IsEndOfMonthExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `is_end_of_month` requires arguments: is_end_of_month(TIMESTAMP)"
                    .to_string(),
            ));
        }
        Ok(IsEndOfMonthExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_to_str_call(&self) -> bool {
        self.starts_named_call("to_str")
    }

    fn parse_to_str(&mut self) -> Result<ToStrExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `to_str` requires arguments".to_string(),
            ));
        }
        Ok(ToStrExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_days_in_month_call(&self) -> bool {
        self.starts_named_call("days_in_month")
    }

    fn parse_days_in_month(&mut self) -> Result<DaysInMonthExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `days_in_month` requires arguments: days_in_month(TIMESTAMP)".to_string(),
            ));
        }
        Ok(DaysInMonthExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_is_leap_year_call(&self) -> bool {
        self.starts_named_call("is_leap_year")
    }

    fn parse_is_leap_year(&mut self) -> Result<IsLeapYearExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `is_leap_year` requires arguments: is_leap_year(TIMESTAMP)".to_string(),
            ));
        }
        Ok(IsLeapYearExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_timestamp_ceil_call(&self) -> bool {
        self.starts_named_call("timestamp_ceil")
    }

    fn parse_timestamp_ceil(&mut self) -> Result<TimestampCeilExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `timestamp_ceil` requires arguments: timestamp_ceil(STRING constant, TIMESTAMP)"
                    .to_string(),
            ));
        }
        Ok(TimestampCeilExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_timestamp_floor_call(&self) -> bool {
        self.starts_named_call("timestamp_floor")
    }

    fn parse_timestamp_floor(&mut self) -> Result<TimestampFloorExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `timestamp_floor` requires arguments".to_string(),
            ));
        }
        Ok(TimestampFloorExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_date_trunc_call(&self) -> bool {
        self.starts_named_call("date_trunc")
    }

    fn parse_date_trunc(&mut self) -> Result<DateTruncExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `date_trunc` requires arguments: date_trunc(STRING constant, TIMESTAMP)"
                    .to_string(),
            ));
        }
        Ok(DateTruncExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_week_of_year_call(&self) -> bool {
        self.starts_named_call("week_of_year")
    }

    fn parse_week_of_year(&mut self) -> Result<WeekOfYearExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `week_of_year` requires arguments: week_of_year(TIMESTAMP)".to_string(),
            ));
        }
        Ok(WeekOfYearExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_day_of_week_call(&self) -> bool {
        self.starts_named_call("day_of_week")
    }

    fn parse_day_of_week(&mut self) -> Result<DayOfWeekExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `day_of_week` requires arguments: day_of_week(TIMESTAMP)".to_string(),
            ));
        }
        Ok(DayOfWeekExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_micros_call(&self) -> bool {
        self.starts_named_call("micros")
    }

    fn parse_micros(&mut self) -> Result<MicrosExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `micros` requires arguments: micros(TIMESTAMP)".to_string(),
            ));
        }
        Ok(MicrosExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_millis_call(&self) -> bool {
        self.starts_named_call("millis")
    }

    fn parse_millis(&mut self) -> Result<MillisExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `millis` requires arguments: millis(TIMESTAMP)".to_string(),
            ));
        }
        Ok(MillisExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_second_call(&self) -> bool {
        self.starts_named_call("second")
    }

    fn parse_second(&mut self) -> Result<SecondExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `second` requires arguments: second(TIMESTAMP)".to_string(),
            ));
        }
        Ok(SecondExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_minute_call(&self) -> bool {
        self.starts_named_call("minute")
    }

    fn parse_minute(&mut self) -> Result<MinuteExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `minute` requires arguments: minute(TIMESTAMP)".to_string(),
            ));
        }
        Ok(MinuteExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_hour_call(&self) -> bool {
        self.starts_named_call("hour")
    }

    fn parse_hour(&mut self) -> Result<HourExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `hour` requires arguments: hour(TIMESTAMP)".to_string(),
            ));
        }
        Ok(HourExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_day_call(&self) -> bool {
        self.starts_named_call("day")
    }

    fn parse_day(&mut self) -> Result<DayExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `day` requires arguments: day(TIMESTAMP)".to_string(),
            ));
        }
        Ok(DayExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_month_call(&self) -> bool {
        self.starts_named_call("month")
    }

    fn parse_month(&mut self) -> Result<MonthExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `month` requires arguments: month(TIMESTAMP)".to_string(),
            ));
        }
        Ok(MonthExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_year_call(&self) -> bool {
        self.starts_named_call("year")
    }

    fn parse_year(&mut self) -> Result<YearExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `year` requires arguments: year(TIMESTAMP)".to_string(),
            ));
        }
        Ok(YearExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_datediff_call(&self) -> bool {
        self.starts_named_call("datediff")
    }

    fn parse_datediff(&mut self) -> Result<DateDiffExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `datediff` requires arguments: datediff(CHAR, TIMESTAMP, TIMESTAMP)"
                    .to_string(),
            ));
        }
        Ok(DateDiffExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_dateadd_call(&self) -> bool {
        self.starts_named_call("dateadd")
    }

    fn parse_dateadd(&mut self) -> Result<DateAddExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `dateadd` requires arguments".to_string(),
            ));
        }
        Ok(DateAddExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_cast_call(&self) -> bool {
        self.starts_named_call("cast")
    }

    fn parse_cast(&mut self) -> Result<CastExpr, Error> {
        let _name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(
                "unsupported: expected '(' after cast".to_string(),
            ));
        }
        self.skip();
        if self.peek() == Some(b')') {
            return Err(Error::new("'as' missing".to_string()));
        }
        let column = self.ident_spelling()?;
        self.skip();
        if self.peek() == Some(b')') {
            return Err(Error::new("'as' missing".to_string()));
        }
        let keyword = self.ident_spelling()?;
        if !keyword.eq_ignore_ascii_case("as") {
            return Err(Error::new("dangling literal".to_string()));
        }
        self.skip();
        if self.peek() == Some(b')') || !self.starts_value() {
            return Err(Error::new("unsupported cast".to_string()));
        }
        let type_name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new("unsupported cast".to_string()));
        }
        Ok(CastExpr {
            label: format!("{column}::{type_name}"),
            column,
            type_name,
        })
    }

    fn starts_nullif_call(&self) -> bool {
        self.starts_named_call("nullif")
    }

    fn parse_nullif(&mut self) -> Result<NullIfExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `nullif` requires arguments".to_string(),
            ));
        }
        Ok(NullIfExpr {
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
