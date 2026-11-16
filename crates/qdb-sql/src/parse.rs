use qdb_types::{parse_date_millis, parse_timestamp_micros, unix_days_from_civil, Error, Scalar};

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
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub columns: SelectList,
    pub table: String,
    pub filter: Option<Filter>,
    /// One or more expressions evaluated as the grouping key.
    pub group_by: Option<Vec<GroupByExpr>>,
    pub order_by: Vec<OrderBy>,
    /// `LIMIT` bounds after the single-argument form is rewritten.
    /// `LIMIT n` with `n >= 0` is `[0, n)`. A negative `n` is `[n, 0)`.
    pub limit: Option<Limit>,
    /// Row count of `FROM long_sequence(n)`. `None` reads `table` from disk.
    pub long_sequence: Option<i64>,
    /// `FROM generate_series(start, end[, step])`.
    pub generate_series: Option<GenerateSeries>,
    /// `JOIN` clauses, in order, each of the shape `a JOIN b ON a.col = b.col`.
    pub join: Vec<EquiJoin>,
}

/// Equijoin parsed from `alias JOIN table alias ON left = right`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquiJoin {
    pub left_alias: String,
    pub right_table: String,
    pub right_alias: String,
    pub left_column: String,
    pub right_column: String,
    /// `LEFT JOIN` keeps a left row when the right side has no match.
    pub left_outer: bool,
    /// `RIGHT JOIN` keeps a right row when the left side has no match.
    pub right_outer: bool,
    /// `CROSS JOIN` pairs every left row with every right row and has no `ON`.
    pub cross: bool,
}

/// Inclusive numeric series produced by `generate_series`.
#[derive(Debug, Clone, PartialEq)]
pub enum GenerateSeries {
    Long {
        start: i64,
        end: i64,
        step: i64,
    },
    Double {
        start: f64,
        end: f64,
        step: f64,
    },
    Timestamp {
        start: i64,
        end: i64,
        step: SeriesStep,
    },
}

/// Step of a timestamp `generate_series`. Months and years keep a calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeriesStep {
    Fixed(i64),
    Months(i32),
    Years(i32),
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
    StdDev,
    Variance,
    VarSamp,
    VarPop,
    StdDevPop,
    StdDevSamp,
    KSum,
    NSum,
    ApproxCountDistinct,
    StringAgg,
    StringDistinctAgg,
    CovarPop,
    Vwap,
    Twap,
    WeightedAvg,
    WeightedStdDev,
    WeightedStdDevRel,
    WeightedStdDevFreq,
    CovarSamp,
    Corr,
    RegrSlope,
    RegrIntercept,
    RegrR2,
    BoolAnd,
    BoolOr,
    IsOrdered,
    PercentileDisc,
    QuantileDisc,
    PercentileCont,
    QuantileCont,
    FirstNotNull,
    LastNotNull,
    BitAnd,
    BitOr,
    BitXor,
    GeoMean,
    Skewness,
    SkewnessPop,
    SkewnessSamp,
    KurtosisPop,
    KurtosisSamp,
    Kurtosis,
    Sparkline,
    Mode,
    ApproxMedian,
    ApproxPercentile,
    ArgMax,
    ArgMin,
    Haversine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateExpr {
    pub func: AggFunc,
    /// Byte position of the aggregate call in the SELECT or ORDER BY expression.
    pub position: usize,
    /// Result column name, such as `count()`, `min(id)` or `MAX(note)`.
    pub label: String,
    /// `None` for `count()` and `count(*)`. Every other aggregate names a column.
    pub column: Option<String>,
    /// Optional precision of `approx_count_distinct(col, precision)`.
    pub precision: Option<Scalar>,
    /// A third argument was present.
    pub precision_more: bool,
    /// Separator literal of `string_agg(col, sep)`.
    pub separator: Option<Scalar>,
    /// Second argument of `string_agg` when it names a column.
    pub separator_column: Option<String>,
    /// Third argument of `string_agg`, when present.
    pub separator_more: Option<Scalar>,
    /// Second column of `covar_pop(y, x)`.
    pub pair: Option<String>,
    /// Extra argument of `first_not_null`. QuestDB 10.1.0 has no two-argument form.
    pub extra: Option<AggregateExtra>,
    /// Type name of a third `approx_median` argument, when present.
    pub third_type: Option<String>,
    /// Literal in the third argument of `approx_percentile`, when it is a number.
    pub third_value: Option<Scalar>,
    /// Type name of a fourth `approx_percentile` argument, when present.
    pub fourth_type: Option<String>,
}

/// Constant `min`, `max` and `width` of `sparkline`. `None` means the SQL NULL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SparkParams {
    pub min_bits: Option<u64>,
    pub max_bits: Option<u64>,
    pub width: Option<i32>,
}

struct ParsedSpark {
    matched: bool,
    min_bits: Option<u64>,
    max_bits: Option<u64>,
    width: Option<i32>,
    suffix: String,
    mismatch: Option<String>,
}

enum SparkBound {
    Auto,
    Number(f64),
}

fn spark_bound(value: &Scalar, kind: &str) -> Result<Option<SparkBound>, Error> {
    if kind.starts_with("col:") {
        return Ok(None);
    }
    match value {
        Scalar::Null => Ok(Some(SparkBound::Auto)),
        Scalar::Double(number) => Ok(Some(SparkBound::Number(*number))),
        Scalar::Long(number) => Ok(Some(SparkBound::Number(*number as f64))),
        Scalar::Text(text) => {
            let parsed = text.parse::<f64>().ok().filter(|number| number.is_finite());
            let Some(number) = parsed else {
                if text.chars().count() <= 1 {
                    return Err(Error::new(format!(
                        "inconvertible value: {text} [CHAR -> DOUBLE]"
                    )));
                }
                return Err(Error::new(format!(
                    "inconvertible value: `{text}` [STRING -> DOUBLE]"
                )));
            };
            Ok(Some(SparkBound::Number(number)))
        }
        _ => Ok(None),
    }
}

fn median_literal_type(number: &Scalar) -> &'static str {
    match number {
        Scalar::Double(_) => "DOUBLE",
        Scalar::Long(value) if i32::try_from(*value).is_ok() => "INT",
        Scalar::Long(_) => "LONG",
        _ => "DOUBLE",
    }
}

/// A second argument that `first_not_null` rejects with the QuestDB type list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateExtra {
    Column(String),
    Type(String),
}

/// One grouping column plus one aggregate, in select-list order.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedSelect {
    /// Grouping expressions selected as columns, in SELECT order.
    pub keys: Vec<GroupedSelectKey>,
    pub aggregates: Vec<AggregateExpr>,
    /// SELECT-list order of keys and aggregates.
    pub select_order: Vec<GroupedSelectItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupByExpr {
    pub source: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupedSelectKey {
    pub label: String,
    pub operand: FilterOperand,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnExpr {
    pub name: String,
    pub label: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionList {
    pub items: Vec<Proj>,
    pub positions: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupedSelectItem {
    Key(usize),
    Aggregate(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectList {
    Star,
    Columns(Vec<ColumnExpr>),
    Aggregate(Box<AggregateExpr>),
    Grouped(GroupedSelect),
    /// A select list that contains `concat(...)`. Plain columns may sit beside it.
    Projection(ProjectionList),
}

/// One select-list value. A column name is stored lowercased; `concat` keeps its label.
#[derive(Debug, Clone, PartialEq)]
pub enum Proj {
    Column(String),
    Arithmetic(ArithmeticExpr),
    Concat(ConcatExpr),
    Length(LengthExpr),
    Lower(LowerExpr),
    Upper(UpperExpr),
    Side(SideExpr),
    Substr(SubstrExpr),
    Trim(TrimExpr),
    Replace(ReplaceExpr),
    RegexpReplace(RegexpReplaceExpr),
    StartsWith(StartsWithExpr),
    Glob(GlobExpr),
    Ilike(IlikeExpr),
    Like(LikeExpr),
    Switch(SwitchExpr),
    Bar(BarExpr),
    TimestampSequence(TimestampSequenceExpr),
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
    Nanos(NanosExpr),
    ToUtc(ToUtcExpr),
    ToTimezone(ToTimezoneExpr),
    ToPgDate(ToPgDateExpr),
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
    Mid(MidExpr),
    Spread(SpreadExpr),
    SpreadBps(SpreadBpsExpr),
    Wmid(WmidExpr),
    Between(BetweenExpr),
    InList(InListExpr),
    Eq(EqExpr),
    Add(AddExpr),
    Neg(NegExpr),
    BitNot(BitNotExpr),
    And(AndExpr),
    Ln(LnExpr),
    Log(LogExpr),
    Exp(ExpExpr),
    Pi(PiExpr),
    CurrentSchema(CurrentSchemaExpr),
    CurrentDatabase(CurrentDatabaseExpr),
    CurrentUser(CurrentUserExpr),
    SessionUser(SessionUserExpr),
    CurrentCatalog(CurrentCatalogExpr),
    Version(VersionExpr),
    PgIsInRecovery(PgIsInRecoveryExpr),
    FormatType(FormatTypeExpr),
    CurrentSetting(CurrentSettingExpr),
    PgGetPartKeyDef(PgGetPartKeyDefExpr),
    PgGetUserById(PgGetUserByIdExpr),
    PgTableIsVisible(PgTableIsVisibleExpr),
    PgAge(PgAgeExpr),
    PgGetExpr(PgGetExprExpr),
    Netmask(NetmaskExpr),
    GeoDistance(GeoDistanceExpr),
    GeoWithin(GeoWithinExpr),
    WithinRadius(WithinRadiusExpr),
    WithinBox(WithinBoxExpr),
    MakeGeohash(MakeGeohashExpr),
    ToUuid(ToUuidExpr),
    JsonExtract(JsonExtractExpr),
    ToChar(ToCharExpr),
    Greatest(GreatestExpr),
    Least(LeastExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LengthExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub bytes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub nested_trim: Option<Box<TrimExpr>>,
    pub nested_replace: Option<Box<ReplaceExpr>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpperExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub nested_trim: Option<Box<TrimExpr>>,
    pub nested_replace: Option<Box<ReplaceExpr>>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestedTextCase {
    Lower(LowerExpr),
    Upper(UpperExpr),
}

/// `trim`, `ltrim` or `rtrim`: one text argument, ASCII spaces removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub end: TrimEnd,
    pub nested_case: Option<NestedTextCase>,
    pub nested_replace: Option<Box<ReplaceExpr>>,
}

/// `replace(value, old, new)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub value_trim: Option<Box<TrimExpr>>,
}

/// `regexp_replace(value, pattern, replacement)`.
#[derive(Debug, Clone, PartialEq)]
pub struct RegexpReplaceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    /// Optional scalar expression for the input argument. Currently parsed for `lpad`/`rpad`.
    pub value_expr: Option<Box<Proj>>,
    /// Byte position of the function name and each argument in the SQL text.
    pub position: usize,
    pub arg_positions: Vec<usize>,
}

/// `starts_with(text, prefix)` returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartsWithExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `glob(text, pattern)` matches a constant glob and returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `ilike(text, pattern)` is a case-insensitive `LIKE` and returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IlikeExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after label rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
    pub negated: bool,
}

/// `like(text, pattern)` is a case-sensitive `LIKE` and returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LikeExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after label rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
    pub negated: bool,
    pub regex: bool,
}

/// `switch(key, case, result, ..., [else])` picks a result by key equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `bar(value, min, max, width)` renders a block bar as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `timestamp_sequence(start, step)` yields a timestamp that advances by `step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampSequenceExpr {
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
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
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
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `nullif(value, value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NullIfExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `cast(column AS type)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastExpr {
    pub label: String,
    /// Original cast spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub column: String,
    pub type_name: String,
    /// `true` for `column::type`. `false` for `cast(column AS type)`.
    pub infix: bool,
}

/// `dateadd(period, stride, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateAddExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `datediff(period, start, end)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateDiffExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `day(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `hour(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HourExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `minute(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinuteExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `second(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `millis(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MillisExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `micros(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicrosExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `nanos(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NanosExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_utc(timestamp, timezone)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToUtcExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_timezone(timestamp, timezone)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToTimezoneExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_pg_date(string)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToPgDateExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `day_of_week(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayOfWeekExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `week_of_year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeekOfYearExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
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
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `timestamp_ceil(unit, timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampCeilExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `is_leap_year(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsLeapYearExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `days_in_month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaysInMonthExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_str(timestamp, format)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToStrExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `is_end_of_month(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IsEndOfMonthExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `day_of_week_sunday_first(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayOfWeekSundayFirstExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_timestamp(text)` parses a microsecond count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToTimestampExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_date(text, pattern)` parses a millisecond date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToDateExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `md5(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Md5Expr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `sha1(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha1Expr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `sha256(text)` returns the lowercase hex digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256Expr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
}

/// `extract(part FROM timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: String,
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

/// `make_geohash(lon, lat, bits)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakeGeohashExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `to_uuid(lo, hi)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToUuidExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `json_extract(doc, path)` and the typed `json_extract(doc, path, type)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExtractExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `from_base64(text)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FromBase64Expr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_char(bin)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToCharExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub base64: Option<FromBase64Expr>,
}

/// `typeof(value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeofExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    /// Arithmetic argument whose static result type is being inspected.
    pub arithmetic: Option<Box<str>>,
    pub geohash: Option<MakeGeohashExpr>,
    pub uuid: Option<Box<ToUuidExpr>>,
    pub json: Option<Box<JsonExtractExpr>>,
    pub to_char: Option<Box<ToCharExpr>>,
}

/// `not(value)`. Extra arguments are ignored; the last one is negated.
#[derive(Debug, Clone, PartialEq)]
pub struct NotExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    /// Byte offset of the NOT expression in the query.
    pub position: usize,
    pub args: Vec<ConcatArg>,
    /// A prefix `NOT` expression parsed with the filter grammar.
    pub filter: Option<Box<Filter>>,
}

/// `power(base, exponent)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `mid(bid, ask)` returns the mean of two numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MidExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `spread(bid, ask)` returns `ask - bid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `spread_bps(bid, ask)` returns the spread over the midpoint, in basis points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpreadBpsExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `wmid(bid_size, bid_price, ask_price, ask_size)` returns the size-weighted midpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WmidExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `between(value, low, high)` reports whether `value` lies inside a closed interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BetweenExpr {
    pub label: String,
    /// Original expression spelling, preserved for grouped-source diagnostics.
    pub source_label: String,
    pub args: Vec<ConcatArg>,
    pub negated: bool,
}

/// `value in (list)` reports whether `value` equals one listed constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InListExpr {
    pub label: String,
    /// Original expression spelling, preserved for grouped-source diagnostics.
    pub source_label: String,
    pub value: String,
    pub args: Vec<ConcatArg>,
    pub negated: bool,
}

/// `+`, `-`, `*`, `/` or `%` of a column and a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
}

impl ArithOp {
    pub fn spelling(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
        }
    }
}

/// `and` or `or` of a boolean column and a column or literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

impl BoolOp {
    pub fn spelling(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

/// The right side of `value and other`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AndRight {
    Literal(Scalar),
    Column(String),
}

/// `value and other` or `value or other` joins a boolean column with a column or literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AndExpr {
    pub label: String,
    pub source_label: String,
    pub left: String,
    pub right: AndRight,
    pub op: BoolOp,
}

/// `~value` is the bitwise complement of one column. The header is `~(value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitNotExpr {
    pub label: String,
    pub value: String,
}

/// `-value` negates one column. The header is `-(value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegExpr {
    pub label: String,
    pub value: String,
}

/// `value + other` adds, subtracts, multiplies, divides or takes the remainder of a column and a constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddExpr {
    pub label: String,
    pub value: String,
    pub literal: Scalar,
    /// Right-hand column. `None` means `literal` is the constant operand.
    pub other: Option<String>,
    pub op: ArithOp,
}

/// A full arithmetic expression projected as one SELECT column.
#[derive(Debug, Clone, PartialEq)]
pub struct ArithmeticExpr {
    pub label: String,
    pub source: String,
    pub position: usize,
    pub operand: FilterOperand,
}

/// `value = other` compares a column with one constant or another column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqExpr {
    pub label: String,
    /// Original comparison spelling, preserved for grouped-source diagnostics after aliases.
    pub source_label: String,
    pub value: String,
    pub op: CmpOp,
    pub literal: Scalar,
    /// Right-hand column. `None` means `literal` is the constant operand.
    pub other: Option<String>,
}

fn eq_literal_label(literal: &Scalar) -> String {
    match literal {
        Scalar::Null => "null".to_string(),
        Scalar::Boolean(true) => "true".to_string(),
        Scalar::Boolean(false) => "false".to_string(),
        Scalar::Long(number) if *number < 0 => format!("-({})", number.unsigned_abs()),
        Scalar::Long(number) => number.to_string(),
        Scalar::Double(number) if number.is_sign_negative() => {
            format!("-({})", format_eq_magnitude(*number))
        }
        Scalar::Double(number) => format_eq_magnitude(*number),
        Scalar::Text(text) => format!("'{}'", text.replace('\'', "''")),
        Scalar::Char(unit) => format!("'{}'", char::from_u32(u32::from(*unit)).unwrap_or('\u{0}')),
        other => format!("{other:?}"),
    }
}

fn filter_expression_label(filter: &Filter) -> String {
    match filter {
        Filter::BooleanColumn { column } => column.clone(),
        Filter::BooleanLiteral(Some(value)) => value.to_string(),
        Filter::BooleanLiteral(None) => "NULL".to_string(),
        Filter::NonBooleanLiteral { value, .. } => eq_literal_label(value),
        Filter::Compare {
            column,
            op,
            literal,
        } => format!(
            "{column} {} {}",
            cmp_op_label(*op),
            eq_literal_label(literal)
        ),
        Filter::CompareColumns { left, op, right } => {
            format!("{left} {} {right}", cmp_op_label(*op))
        }
        Filter::CompareExpressions { left, op, right } => format!(
            "{} {} {}",
            filter_operand_label(left),
            cmp_op_label(*op),
            filter_operand_label(right)
        ),
        Filter::Between {
            column,
            low,
            high,
            negated,
        } => format!(
            "{column} {}between {} and {}",
            if *negated { "not " } else { "" },
            eq_literal_label(low),
            eq_literal_label(high)
        ),
        Filter::InList {
            column,
            items,
            negated,
            ..
        } => format!(
            "{column} {}in ({})",
            if *negated { "not " } else { "" },
            items
                .iter()
                .map(eq_literal_label)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Filter::InListExpression {
            value,
            items,
            negated,
            ..
        } => format!(
            "{} {}in ({})",
            filter_operand_label(value),
            if *negated { "not " } else { "" },
            items
                .iter()
                .map(eq_literal_label)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Filter::Like {
            column,
            pattern,
            case_insensitive,
            negated,
        } => format!(
            "{column} {}{}like {}",
            if *negated { "not " } else { "" },
            if *case_insensitive { "i" } else { "" },
            concat_arg_label(pattern)
        ),
        Filter::Regex {
            column,
            pattern,
            negated,
            ..
        } => format!(
            "{column} {} {}",
            if *negated { "!~" } else { "~" },
            concat_arg_label(pattern)
        ),
        Filter::RegexExpression {
            label,
            pattern,
            negated,
            ..
        } => format!(
            "{label} {} {}",
            if *negated { "!~" } else { "~" },
            concat_arg_label(pattern)
        ),
        Filter::LikeExpression {
            label,
            pattern,
            case_insensitive,
            negated,
            ..
        } => format!(
            "{label} {}{}like {}",
            if *negated { "not " } else { "" },
            if *case_insensitive { "i" } else { "" },
            concat_arg_label(pattern)
        ),
        Filter::Not(inner) => format!("not({})", filter_expression_label(inner)),
        Filter::And(left, right) => format!(
            "{} and {}",
            filter_expression_label(left),
            filter_expression_label(right)
        ),
        Filter::Or(left, right) => format!(
            "{} or {}",
            filter_expression_label(left),
            filter_expression_label(right)
        ),
    }
}

fn cmp_op_label(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "=",
        CmpOp::NotEq => "<>",
        CmpOp::Lt => "<",
        CmpOp::Gt => ">",
        CmpOp::LtEq => "<=",
        CmpOp::GtEq => ">=",
    }
}

fn filter_operand_label(operand: &FilterOperand) -> String {
    match operand {
        FilterOperand::Literal(value) => eq_literal_label(value),
        FilterOperand::Projection(projection) => match projection.as_ref() {
            Proj::Column(name) => name.clone(),
            Proj::Arithmetic(expression) => expression.label.clone(),
            Proj::Like(expression) => expression.label.clone(),
            Proj::Ilike(expression) => expression.label.clone(),
            Proj::Not(expression) => expression.label.clone(),
            Proj::And(expression) => expression.label.clone(),
            _ => "<expression>".to_string(),
        },
        FilterOperand::Arithmetic(expression) => format!(
            "{} {} {}",
            filter_operand_label(&expression.left),
            expression.op.spelling(),
            filter_operand_label(&expression.right)
        ),
    }
}

fn concat_arg_label(arg: &ConcatArg) -> String {
    match arg {
        ConcatArg::Column(name) => name.clone(),
        ConcatArg::Literal(value) => eq_literal_label(value),
    }
}

fn format_eq_magnitude(number: f64) -> String {
    let text = format!("{}", number.abs());
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
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

/// `current_schema()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentSchemaExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `current_database()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentDatabaseExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `current_user()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUserExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `session_user()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUserExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `current_catalog()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentCatalogExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `version()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `pg_is_in_recovery()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgIsInRecoveryExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `format_type(oid, typmod)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatTypeExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `current_setting(name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentSettingExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `pg_get_partkeydef(oid)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetPartKeyDefExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `pg_catalog.pg_get_userbyid(id)` returns the fixed role name `public`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetUserByIdExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_catalog.pg_table_is_visible(oid)` returns the fixed boolean `true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgTableIsVisibleExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_catalog.age(xid)` returns the fixed long `0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgAgeExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_get_expr(expr, relation [, pretty])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetExprExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `netmask(text)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetmaskExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `geo_distance_meters(lat1, lon1, lat2, lon2)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoDistanceExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `geo_within_radius_latlon(lat, lon, center_lat, center_lon, radius)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoWithinExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `within_radius(x, y, center_x, center_y, radius)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinRadiusExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
    pub args: Vec<ConcatArg>,
}

/// `within_box(x, y, min_x, min_y, max_x, max_y)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinBoxExpr {
    pub label: String,
    /// Original call spelling, preserved for source positions after alias rewriting.
    pub source_label: Box<str>,
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

type ParsedCallArgsWithPositions = (Vec<String>, Vec<ConcatArg>, Vec<usize>);

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperand {
    Projection(Box<Proj>),
    Literal(Scalar),
    Arithmetic(Box<FilterArithmetic>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterArithmetic {
    pub left: FilterOperand,
    pub op: ArithOp,
    pub right: FilterOperand,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    BooleanColumn {
        column: String,
    },
    BooleanLiteral(Option<bool>),
    NonBooleanLiteral {
        value: Scalar,
        position: usize,
    },
    Compare {
        column: String,
        op: CmpOp,
        literal: Scalar,
    },
    CompareColumns {
        left: String,
        op: CmpOp,
        right: String,
    },
    CompareExpressions {
        left: FilterOperand,
        op: CmpOp,
        right: FilterOperand,
    },
    Between {
        column: String,
        low: Scalar,
        high: Scalar,
        negated: bool,
    },
    InList {
        column: String,
        items: Vec<Scalar>,
        item_positions: Vec<usize>,
        negated: bool,
        position: usize,
    },
    InListExpression {
        value: FilterOperand,
        items: Vec<Scalar>,
        item_positions: Vec<usize>,
        negated: bool,
        position: usize,
    },
    Like {
        column: String,
        pattern: ConcatArg,
        case_insensitive: bool,
        negated: bool,
    },
    Regex {
        column: String,
        pattern: ConcatArg,
        negated: bool,
        position: usize,
    },
    RegexExpression {
        label: String,
        value: Box<Proj>,
        pattern: ConcatArg,
        negated: bool,
        position: usize,
        operator_position: usize,
        value_position: usize,
    },
    LikeExpression {
        label: String,
        value: Box<Proj>,
        pattern: ConcatArg,
        case_insensitive: bool,
        negated: bool,
        operator_position: usize,
        value_position: usize,
    },
    Not(Box<Filter>),
    And(Box<Filter>, Box<Filter>),
    Or(Box<Filter>, Box<Filter>),
}

fn strip_table_alias(name: &str, alias: &str) -> String {
    let prefix = format!("{alias}.");
    if name.len() > prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(&prefix) {
        name[prefix.len()..].to_string()
    } else {
        name.to_string()
    }
}

fn strip_alias_refs(
    columns: &mut SelectList,
    filter: &mut Option<Filter>,
    order_by: &mut [OrderBy],
    group_by: &mut Option<Vec<GroupByExpr>>,
    alias: &str,
) {
    match columns {
        SelectList::Projection(items) => {
            for item in &mut items.items {
                if let Proj::Column(name) = item {
                    *name = strip_table_alias(name, alias);
                }
                if let Proj::PgGetUserById(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::PgTableIsVisible(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::PgAge(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::PgGetExpr(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::PgGetPartKeyDef(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::CurrentSetting(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::FormatType(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Netmask(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::GeoDistance(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::GeoWithin(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::WithinRadius(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::WithinBox(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::MakeGeohash(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToUuid(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Md5(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Sha1(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Sha256(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Floor(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Ceil(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Sign(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Sqrt(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Sin(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Cos(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Tan(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Asin(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Acos(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Atan(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Atan2(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Degrees(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Radians(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Cot(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Power(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Ln(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Log(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Exp(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Not(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(filter) = &mut expr.filter {
                        strip_filter_alias(filter, alias);
                    }
                }
                if let Proj::Mid(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Spread(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::SpreadBps(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Wmid(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Cast(expr) = item {
                    expr.column = strip_table_alias(&expr.column, alias);
                }
                if let Proj::Between(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::InList(expr) = item {
                    expr.value = strip_table_alias(&expr.value, alias);
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Eq(expr) = item {
                    expr.value = strip_table_alias(&expr.value, alias);
                    if let Some(other) = &mut expr.other {
                        *other = strip_table_alias(other, alias);
                    }
                }
                if let Proj::Add(expr) = item {
                    expr.value = strip_table_alias(&expr.value, alias);
                    if let Some(other) = &mut expr.other {
                        *other = strip_table_alias(other, alias);
                    }
                }
                if let Proj::Arithmetic(expr) = item {
                    strip_filter_operand_alias(&mut expr.operand, alias);
                }
                if let Proj::Neg(expr) = item {
                    expr.value = strip_table_alias(&expr.value, alias);
                }
                if let Proj::BitNot(expr) = item {
                    expr.value = strip_table_alias(&expr.value, alias);
                }
                if let Proj::And(expr) = item {
                    expr.left = strip_table_alias(&expr.left, alias);
                    if let AndRight::Column(name) = &mut expr.right {
                        *name = strip_table_alias(name, alias);
                    }
                }
                if let Proj::ToChar(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(base64) = &mut expr.base64 {
                        for arg in &mut base64.args {
                            if let ConcatArg::Column(name) = arg {
                                *name = strip_table_alias(name, alias);
                            }
                        }
                    }
                }
                if let Proj::Length(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Lower(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(trim) = &mut expr.nested_trim {
                        strip_trim_aliases(trim, alias);
                    }
                    if let Some(replace) = &mut expr.nested_replace {
                        strip_replace_aliases(replace, alias);
                    }
                }
                if let Proj::Upper(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(trim) = &mut expr.nested_trim {
                        strip_trim_aliases(trim, alias);
                    }
                    if let Some(replace) = &mut expr.nested_replace {
                        strip_replace_aliases(replace, alias);
                    }
                }
                if let Proj::Concat(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Side(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Substr(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Trim(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    strip_trim_aliases(expr, alias);
                }
                if let Proj::StartsWith(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Glob(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Ilike(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Like(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Switch(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Bar(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::TimestampSequence(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Pad(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DateAdd(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DateDiff(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Year(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Month(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Day(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Hour(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Minute(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Second(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Millis(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Micros(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Nanos(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DayOfWeek(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DayOfWeekSundayFirst(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::WeekOfYear(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DateTrunc(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::TimestampFloor(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::TimestampCeil(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::IsLeapYear(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::DaysInMonth(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::IsEndOfMonth(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToUtc(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToTimezone(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToPgDate(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToDate(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToStr(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::ToTimestamp(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Extract(expr) = item {
                    if let ConcatArg::Column(name) = &mut expr.arg {
                        *name = strip_table_alias(name, alias);
                    }
                }
                if let Proj::Typeof(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(json) = &mut expr.json {
                        for arg in &mut json.args {
                            if let ConcatArg::Column(name) = arg {
                                *name = strip_table_alias(name, alias);
                            }
                        }
                    }
                    if let Some(uuid) = &mut expr.uuid {
                        for arg in &mut uuid.args {
                            if let ConcatArg::Column(name) = arg {
                                *name = strip_table_alias(name, alias);
                            }
                        }
                    }
                    if let Some(geohash) = &mut expr.geohash {
                        for arg in &mut geohash.args {
                            if let ConcatArg::Column(name) = arg {
                                *name = strip_table_alias(name, alias);
                            }
                        }
                    }
                    if let Some(to_char) = &mut expr.to_char {
                        for arg in &mut to_char.args {
                            if let ConcatArg::Column(name) = arg {
                                *name = strip_table_alias(name, alias);
                            }
                        }
                        if let Some(base64) = &mut to_char.base64 {
                            for arg in &mut base64.args {
                                if let ConcatArg::Column(name) = arg {
                                    *name = strip_table_alias(name, alias);
                                }
                            }
                        }
                    }
                }
                if let Proj::JsonExtract(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Replace(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                    if let Some(trim) = &mut expr.value_trim {
                        strip_trim_aliases(trim, alias);
                    }
                }
                if let Proj::RegexpReplace(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::StrPos(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::SplitPart(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::QuoteIdent(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::SizePretty(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Abs(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Round(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::RoundDown(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::RoundUp(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::RoundHalfEven(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Greatest(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Least(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::Coalesce(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
                if let Proj::NullIf(expr) = item {
                    for arg in &mut expr.args {
                        if let ConcatArg::Column(name) = arg {
                            *name = strip_table_alias(name, alias);
                        }
                    }
                }
            }
        }
        SelectList::Columns(names) => {
            for column in names {
                column.name = strip_table_alias(&column.name, alias);
                column.label = strip_table_alias(&column.label, alias);
            }
        }
        SelectList::Grouped(grouped) => {
            for key in &mut grouped.keys {
                key.label = strip_table_alias(&key.label, alias);
                strip_filter_operand_alias(&mut key.operand, alias);
            }
            for aggregate in &mut grouped.aggregates {
                for column in [
                    &mut aggregate.column,
                    &mut aggregate.separator_column,
                    &mut aggregate.pair,
                ]
                .into_iter()
                .flatten()
                {
                    *column = strip_table_alias(column, alias);
                }
                if let Some(AggregateExtra::Column(column)) = &mut aggregate.extra {
                    *column = strip_table_alias(column, alias);
                }
            }
        }
        SelectList::Star | SelectList::Aggregate(_) => {}
    }
    if let Some(filter) = filter {
        strip_filter_alias(filter, alias);
    }
    for order in order_by.iter_mut() {
        order.column = strip_table_alias(&order.column, alias);
    }
    if let Some(keys) = group_by {
        for key in keys {
            key.source = strip_alias_from_expression(&key.source, alias);
        }
    }
}

fn strip_alias_from_expression(source: &str, alias: &str) -> String {
    let bytes = source.as_bytes();
    let alias_bytes = alias.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut pos = 0usize;
    let mut quoted = false;
    while pos < bytes.len() {
        if bytes[pos] == b'\'' {
            output.push(bytes[pos]);
            if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                output.push(b'\'');
                pos += 2;
                continue;
            }
            quoted = !quoted;
            pos += 1;
            continue;
        }
        if !quoted
            && bytes
                .get(pos..pos + alias_bytes.len())
                .is_some_and(|candidate| candidate.eq_ignore_ascii_case(alias_bytes))
            && bytes.get(pos + alias_bytes.len()) == Some(&b'.')
        {
            pos += alias_bytes.len() + 1;
            continue;
        }
        output.push(bytes[pos]);
        pos += 1;
    }
    String::from_utf8(output).expect("alias stripping preserves UTF-8")
}

fn strip_filter_alias(filter: &mut Filter, alias: &str) {
    match filter {
        Filter::BooleanColumn { column }
        | Filter::Compare { column, .. }
        | Filter::Between { column, .. }
        | Filter::InList { column, .. } => {
            *column = strip_table_alias(column, alias);
        }
        Filter::InListExpression { value, .. } => match value {
            FilterOperand::Projection(expr) => strip_filter_expr_alias(expr, alias),
            FilterOperand::Arithmetic(expr) => strip_filter_arithmetic_alias(expr, alias),
            FilterOperand::Literal(_) => {}
        },
        Filter::CompareColumns { left, right, .. } => {
            *left = strip_table_alias(left, alias);
            *right = strip_table_alias(right, alias);
        }
        Filter::CompareExpressions { left, right, .. } => {
            if let FilterOperand::Projection(expr) = left {
                strip_filter_expr_alias(expr, alias);
            }
            if let FilterOperand::Projection(expr) = right {
                strip_filter_expr_alias(expr, alias);
            }
            if let FilterOperand::Arithmetic(expr) = left {
                strip_filter_arithmetic_alias(expr, alias);
            }
            if let FilterOperand::Arithmetic(expr) = right {
                strip_filter_arithmetic_alias(expr, alias);
            }
        }
        Filter::Like {
            column, pattern, ..
        } => {
            *column = strip_table_alias(column, alias);
            if let ConcatArg::Column(name) = pattern {
                *name = strip_table_alias(name, alias);
            }
        }
        Filter::Regex {
            column, pattern, ..
        } => {
            *column = strip_table_alias(column, alias);
            if let ConcatArg::Column(name) = pattern {
                *name = strip_table_alias(name, alias);
            }
        }
        Filter::RegexExpression { value, pattern, .. } => {
            strip_filter_expr_alias(value, alias);
            if let ConcatArg::Column(name) = pattern {
                *name = strip_table_alias(name, alias);
            }
        }
        Filter::LikeExpression { value, pattern, .. } => {
            strip_filter_expr_alias(value, alias);
            if let ConcatArg::Column(name) = pattern {
                *name = strip_table_alias(name, alias);
            }
        }
        Filter::BooleanLiteral(_) | Filter::NonBooleanLiteral { .. } => {}
        Filter::Not(inner) => strip_filter_alias(inner, alias),
        Filter::And(left, right) | Filter::Or(left, right) => {
            strip_filter_alias(left, alias);
            strip_filter_alias(right, alias);
        }
    }
}

fn strip_filter_expr_alias(expr: &mut Proj, alias: &str) {
    match expr {
        Proj::Column(name) => *name = strip_table_alias(name, alias),
        Proj::Add(expr) => {
            expr.value = strip_table_alias(&expr.value, alias);
            if let Some(other) = &mut expr.other {
                *other = strip_table_alias(other, alias);
            }
        }
        Proj::Abs(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Length(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Lower(expr) => {
            strip_concat_aliases(&mut expr.args, alias);
            if let Some(trim) = &mut expr.nested_trim {
                strip_trim_aliases(trim, alias);
            }
        }
        Proj::Upper(expr) => {
            strip_concat_aliases(&mut expr.args, alias);
            if let Some(trim) = &mut expr.nested_trim {
                strip_trim_aliases(trim, alias);
            }
            if let Some(replace) = &mut expr.nested_replace {
                strip_replace_aliases(replace, alias);
            }
        }
        Proj::Trim(expr) => strip_trim_aliases(expr, alias),
        Proj::Concat(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Replace(expr) => strip_replace_aliases(expr, alias),
        Proj::Substr(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Pad(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::RegexpReplace(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::QuoteIdent(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Md5(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Sha1(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Sha256(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Side(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::SplitPart(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::ToStr(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::FormatType(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::JsonExtract(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::ToUuid(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::PgGetUserById(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::PgGetPartKeyDef(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::PgGetExpr(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Bar(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::MakeGeohash(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::Netmask(expr) => strip_concat_aliases(&mut expr.args, alias),
        Proj::ToChar(expr) => {
            strip_concat_aliases(&mut expr.args, alias);
            if let Some(base64) = &mut expr.base64 {
                strip_concat_aliases(&mut base64.args, alias);
            }
        }
        _ => {}
    }
}

fn strip_concat_aliases(args: &mut [ConcatArg], alias: &str) {
    for arg in args {
        if let ConcatArg::Column(name) = arg {
            *name = strip_table_alias(name, alias);
        }
    }
}

fn strip_trim_aliases(expr: &mut TrimExpr, alias: &str) {
    strip_concat_aliases(&mut expr.args, alias);
    match &mut expr.nested_case {
        Some(NestedTextCase::Lower(lower)) => {
            strip_concat_aliases(&mut lower.args, alias);
            if let Some(trim) = &mut lower.nested_trim {
                strip_trim_aliases(trim, alias);
            }
            if let Some(replace) = &mut lower.nested_replace {
                strip_replace_aliases(replace, alias);
            }
        }
        Some(NestedTextCase::Upper(upper)) => {
            strip_concat_aliases(&mut upper.args, alias);
            if let Some(trim) = &mut upper.nested_trim {
                strip_trim_aliases(trim, alias);
            }
            if let Some(replace) = &mut upper.nested_replace {
                strip_replace_aliases(replace, alias);
            }
        }
        None => {
            if let Some(replace) = &mut expr.nested_replace {
                strip_replace_aliases(replace, alias);
            }
        }
    }
}

fn strip_replace_aliases(expr: &mut ReplaceExpr, alias: &str) {
    strip_concat_aliases(&mut expr.args, alias);
    if let Some(trim) = &mut expr.value_trim {
        strip_trim_aliases(trim, alias);
    }
}

fn strip_filter_arithmetic_alias(expr: &mut FilterArithmetic, alias: &str) {
    strip_filter_operand_alias(&mut expr.left, alias);
    strip_filter_operand_alias(&mut expr.right, alias);
}

fn strip_filter_operand_alias(operand: &mut FilterOperand, alias: &str) {
    match operand {
        FilterOperand::Projection(proj) => strip_filter_expr_alias(proj, alias),
        FilterOperand::Arithmetic(inner) => strip_filter_arithmetic_alias(inner, alias),
        FilterOperand::Literal(_) => {}
    }
}

fn validate_between_order_by(columns: &SelectList, order_by: &[OrderBy]) -> Result<(), Error> {
    if order_by.is_empty() {
        return Ok(());
    }
    let (selected_columns, between_columns): (Vec<&str>, Vec<&str>) = match columns {
        SelectList::Projection(items) => (
            items
                .items
                .iter()
                .filter_map(|item| match item {
                    Proj::Column(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect(),
            items
                .items
                .iter()
                .filter_map(|item| match item {
                    Proj::Between(expr) => match expr.args.first() {
                        Some(ConcatArg::Column(name)) => Some(name.as_str()),
                        _ => None,
                    },
                    _ => None,
                })
                .collect(),
        ),
        SelectList::Columns(names) => (
            names.iter().map(|column| column.name.as_str()).collect(),
            Vec::new(),
        ),
        _ => (Vec::new(), Vec::new()),
    };

    for column in between_columns {
        let is_selected = selected_columns
            .iter()
            .any(|name| name.eq_ignore_ascii_case(column));
        let orders_by_operand = order_by
            .iter()
            .any(|order| order.column.eq_ignore_ascii_case(column));
        if !is_selected && !orders_by_operand {
            return Err(Error::new(format!("Invalid column: {column}")));
        }
    }
    Ok(())
}

pub fn parse_query(sql: &str) -> Result<Query, Error> {
    let mut parser = Parser {
        src: sql.as_bytes(),
        pos: 0,
    };
    parser.skip();
    parser.expect_keyword("select")?;
    let mut columns = parser.parse_select_list()?;
    parser.expect_keyword("from")?;
    parser.skip();
    let (table, long_sequence, generate_series) = if parser.starts_named_call("long_sequence") {
        let (_, args) = parser.parse_long_sequence_call()?;
        (
            String::from("long_sequence"),
            Some(long_sequence_count(&args)?),
            None,
        )
    } else if parser.starts_named_call("generate_series") {
        let (_, args) = parser.parse_generate_series_call()?;
        (
            String::from("generate_series"),
            None,
            Some(generate_series_spec(&args)?),
        )
    } else {
        (parser.ident()?, None, None)
    };
    parser.skip();
    if parser.eat_keyword("as") {
        parser.skip();
    }
    let table_alias = if parser.clause_keyword_ahead() || !parser.ident_ahead() {
        None
    } else {
        Some(parser.ident()?)
    };
    let mut join = Vec::new();
    loop {
        parser.skip();
        match parser.parse_equi_join()? {
            Some(next) => join.push(next),
            None => break,
        }
    }
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
        let mut keys = Vec::new();
        loop {
            parser.skip();
            let start = parser.pos;
            let end = group_by_expr_end(parser.src, start);
            let raw = std::str::from_utf8(&parser.src[start..end])
                .map_err(|_| Error::new("invalid UTF-8 in GROUP BY expression"))?;
            let expression = raw.trim();
            if expression.is_empty() {
                return Err(Error::new("expected GROUP BY expression"));
            }
            let leading = raw.len() - raw.trim_start().len();
            if let Some(offset) = aggregate_call_offset(expression.as_bytes()) {
                return Err(Error::at(
                    "aggregate functions are not allowed in GROUP BY",
                    start + leading + offset,
                ));
            }
            keys.push(GroupByExpr {
                source: expression.to_string(),
                position: start + leading,
            });
            parser.pos = end;
            if !parser.eat(b',') {
                break;
            }
        }
        group_by = Some(keys);
    }
    if parser.eat_keyword("order") {
        parser.expect_keyword("by")?;
        loop {
            parser.skip();
            let position = parser.pos;
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
            } else if parser.starts_regexp_replace_call() {
                parser.parse_regexp_replace()?.label
            } else if parser.starts_starts_with_call() {
                parser.parse_starts_with()?.label
            } else if parser.starts_glob_call() {
                parser.parse_glob()?.label
            } else if parser.starts_ilike_call() {
                parser.parse_ilike()?.label
            } else if parser.starts_like_call() {
                parser.parse_like()?.label
            } else if parser.starts_switch_call() {
                parser.parse_switch()?.label
            } else if parser.starts_bar_call() {
                parser.parse_bar()?.label
            } else if parser.starts_timestamp_sequence_call() {
                parser.parse_timestamp_sequence()?.label
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
            } else if parser.starts_nanos_call() {
                parser.parse_nanos()?.label
            } else if parser.starts_to_utc_call() {
                parser.parse_to_utc()?.label
            } else if parser.starts_to_timezone_call() {
                parser.parse_to_timezone()?.label
            } else if parser.starts_to_pg_date_call() {
                parser.parse_to_pg_date()?.label
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
            } else if parser.starts_mid_call() {
                parser.parse_mid()?.label
            } else if parser.starts_wmid_call() {
                parser.parse_wmid()?.label
            } else if parser.starts_between_call() {
                parser.parse_between()?.label
            } else if parser.starts_spread_bps_call() {
                parser.parse_spread_bps()?.label
            } else if parser.starts_spread_call() {
                parser.parse_spread()?.label
            } else if parser.starts_ln_call() {
                parser.parse_ln()?.label
            } else if parser.starts_log_call() {
                parser.parse_log()?.label
            } else if parser.starts_exp_call() {
                parser.parse_exp()?.label
            } else if parser.starts_pi_call() {
                parser.parse_pi()?.label
            } else if parser.starts_current_schema_call() {
                parser.parse_current_schema()?.label
            } else if parser.starts_current_database_call() {
                parser.parse_current_database()?.label
            } else if parser.starts_current_user_call() {
                parser.parse_current_user()?.label
            } else if parser.starts_session_user_call() {
                parser.parse_session_user()?.label
            } else if parser.starts_current_catalog_call() {
                parser.parse_current_catalog()?.label
            } else if parser.starts_version_call() {
                parser.parse_version()?.label
            } else if parser.starts_pg_is_in_recovery_call() {
                parser.parse_pg_is_in_recovery()?.label
            } else if parser.starts_format_type_call() {
                parser.parse_format_type()?.label
            } else if parser.starts_current_setting_call() {
                parser.parse_current_setting()?.label
            } else if parser.starts_pg_get_partkeydef_call() {
                parser.parse_pg_get_partkeydef()?.label
            } else if parser.starts_pg_get_userbyid_call() {
                parser.parse_pg_get_userbyid()?.label
            } else if parser.starts_pg_table_is_visible_call() {
                parser.parse_pg_table_is_visible()?.label
            } else if parser.starts_pg_age_call() {
                parser.parse_pg_age()?.label
            } else if parser.starts_pg_get_expr_call() {
                parser.parse_pg_get_expr()?.label
            } else if parser.starts_netmask_call() {
                parser.parse_netmask()?.label
            } else if parser.starts_geo_distance_call() {
                parser.parse_geo_distance()?.label
            } else if parser.starts_geo_within_call() {
                parser.parse_geo_within()?.label
            } else if parser.starts_within_radius_call() {
                parser.parse_within_radius()?.label
            } else if parser.starts_within_box_call() {
                parser.parse_within_box()?.label
            } else if parser.starts_make_geohash_call() {
                parser.parse_make_geohash()?.label
            } else if parser.starts_to_uuid_call() {
                parser.parse_to_uuid()?.label
            } else if parser.starts_json_extract_call() {
                parser.parse_json_extract()?.label
            } else if parser.starts_to_char_call() {
                parser.parse_to_char()?.label
            } else if parser.starts_from_base64_call() {
                parser.parse_from_base64()?.label
            } else if parser.starts_greatest_call() {
                parser.parse_greatest()?.label
            } else if parser.starts_least_call() {
                parser.parse_least()?.label
            } else {
                let mut column = parser.ident()?;
                parser.skip();
                if parser.eat(b'.') {
                    column.push('.');
                    column.push_str(&parser.ident()?);
                    parser.skip();
                }
                if parser.eat_keyword("in") {
                    column = parser.parse_in_list(column)?.label;
                } else if parser.starts_eq_or_ne() {
                    column = parser.parse_eq(column)?.label;
                } else if parser.starts_add() {
                    column = parser.parse_add(column)?.label;
                } else if parser.peek() == Some(b'(') {
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
            order_by.push(OrderBy {
                column,
                descending,
                position,
            });
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
    if join.is_empty() {
        if let Some(alias) = &table_alias {
            strip_alias_refs(
                &mut columns,
                &mut filter,
                &mut order_by,
                &mut group_by,
                alias,
            );
        }
    } else if let Some(alias) = table_alias {
        join[0].left_alias = alias;
    }
    if let Some(group_expr) = group_by.as_ref().and_then(|items| {
        items.iter().find_map(|item| {
            let source = item.source.to_ascii_lowercase();
            group_by_between_keyword(&item.source)
                .map(|offset| (item, offset))
                .or_else(|| source.starts_with("between(").then_some((item, 0)))
        })
    }) {
        return Err(Error::at(
            "literal expected",
            group_expr.0.position + group_expr.1,
        ));
    }
    validate_between_order_by(&columns, &order_by)?;
    Ok(Query {
        columns,
        table,
        filter,
        group_by,
        order_by,
        limit,
        long_sequence,
        generate_series,
        join,
    })
}

fn parse_filter_projection(source: &str) -> Result<Proj, Error> {
    let sql = format!("SELECT {source} FROM __filter_expression");
    let query = parse_query(&sql)?;
    match query.columns {
        SelectList::Columns(mut columns) if columns.len() == 1 => {
            Ok(Proj::Column(columns.remove(0).name))
        }
        SelectList::Projection(mut projections) if projections.items.len() == 1 => {
            Ok(projections.items.remove(0))
        }
        _ => Err(Error::new("unsupported expression in WHERE comparison")),
    }
}

fn parse_boolean_projection(source: &str, position: usize) -> Result<NotExpr, Error> {
    let mut parser = Parser {
        src: source.as_bytes(),
        pos: 0,
    };
    let filter = parser.parse_filter()?;
    parser.skip();
    if parser.pos != parser.src.len() {
        return Err(Error::new("unsupported boolean projection expression"));
    }
    let label = filter_expression_label(&filter).replace("NULL", "null");
    Ok(NotExpr {
        label,
        source_label: source.trim().into(),
        position,
        args: Vec::new(),
        filter: Some(Box::new(filter)),
    })
}

fn is_boolean_projection(source: &str) -> bool {
    let mut expression = source.trim();
    while let Some(inner) = unwrapped_filter_expr(expression) {
        expression = inner.trim();
    }
    has_top_level_boolean_operator(expression.as_bytes())
}

fn top_level_cmp(src: &[u8], start: usize) -> Option<(usize, CmpOp)> {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            b'=' if !quoted && depth == 0 => return Some((pos, CmpOp::Eq)),
            b'!' if !quoted && depth == 0 && src.get(pos + 1) == Some(&b'=') => {
                return Some((pos, CmpOp::NotEq));
            }
            b'<' if !quoted && depth == 0 => {
                return Some((
                    pos,
                    if src.get(pos + 1) == Some(&b'=') {
                        CmpOp::LtEq
                    } else if src.get(pos + 1) == Some(&b'>') {
                        CmpOp::NotEq
                    } else {
                        CmpOp::Lt
                    },
                ));
            }
            b'>' if !quoted && depth == 0 => {
                return Some((
                    pos,
                    if src.get(pos + 1) == Some(&b'=') {
                        CmpOp::GtEq
                    } else {
                        CmpOp::Gt
                    },
                ));
            }
            _ => {}
        }
        pos += 1;
    }
    None
}

fn top_level_is_null(src: &[u8], start: usize) -> Option<(usize, bool, usize)> {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
                pos += 1;
                continue;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
        if !quoted
            && depth == 0
            && src[pos..].len() >= 2
            && src[pos..pos + 2].eq_ignore_ascii_case(b"is")
        {
            let before_is = pos == start || !is_filter_keyword_byte(src[pos - 1]);
            let after_is = src
                .get(pos + 2)
                .is_none_or(|byte| !is_filter_keyword_byte(*byte));
            if before_is && after_is {
                let mut end = pos + 2;
                while src.get(end).is_some_and(u8::is_ascii_whitespace) {
                    end += 1;
                }
                let negated = src
                    .get(end..end + 3)
                    .is_some_and(|word| word.eq_ignore_ascii_case(b"not"))
                    && src
                        .get(end + 3)
                        .is_none_or(|byte| !is_filter_keyword_byte(*byte));
                if negated {
                    end += 3;
                    while src.get(end).is_some_and(u8::is_ascii_whitespace) {
                        end += 1;
                    }
                }
                if src
                    .get(end..end + 4)
                    .is_some_and(|word| word.eq_ignore_ascii_case(b"null"))
                    && src
                        .get(end + 4)
                        .is_none_or(|byte| !is_filter_keyword_byte(*byte))
                {
                    return Some((pos, negated, end + 4));
                }
            }
        }
        pos += 1;
    }
    None
}

fn top_level_in_operator(src: &[u8], start: usize) -> Option<(usize, usize, bool, usize)> {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
                pos += 1;
                continue;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            _ => {}
        }
        if !quoted
            && depth == 0
            && src[pos..].len() >= 2
            && src[pos..pos + 2].eq_ignore_ascii_case(b"in")
        {
            let before_in = pos == start || !is_filter_keyword_byte(src[pos - 1]);
            let after_in = src
                .get(pos + 2)
                .is_none_or(|byte| !is_filter_keyword_byte(*byte));
            if before_in && after_in {
                let mut previous_end = pos;
                while previous_end > start && src[previous_end - 1].is_ascii_whitespace() {
                    previous_end -= 1;
                }
                let mut previous_start = previous_end;
                while previous_start > start && is_filter_keyword_byte(src[previous_start - 1]) {
                    previous_start -= 1;
                }
                let negated = previous_start < previous_end
                    && src[previous_start..previous_end].eq_ignore_ascii_case(b"not");
                return Some((
                    if negated { previous_start } else { pos },
                    pos,
                    negated,
                    pos + 2,
                ));
            }
        }
        pos += 1;
    }
    None
}

fn is_filter_keyword_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn top_level_regex_operator(src: &[u8], start: usize) -> Option<(usize, bool, usize)> {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            b'!' if !quoted && depth == 0 && src.get(pos + 1) == Some(&b'~') => {
                return Some((pos, true, 2));
            }
            b'~' if !quoted && depth == 0 => return Some((pos, false, 1)),
            _ => {}
        }
        pos += 1;
    }
    None
}

fn top_level_like_operator(src: &[u8], start: usize) -> Option<(usize, bool, bool, usize)> {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            byte if !quoted && depth == 0 && (byte.is_ascii_alphabetic() || byte == b'_') => {
                let word_start = pos;
                pos += 1;
                while matches!(src.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
                {
                    pos += 1;
                }
                let word = &src[word_start..pos];
                if word.eq_ignore_ascii_case(b"like") || word.eq_ignore_ascii_case(b"ilike") {
                    return Some((
                        word_start,
                        false,
                        word.eq_ignore_ascii_case(b"ilike"),
                        pos - word_start,
                    ));
                }
                if word.eq_ignore_ascii_case(b"not") {
                    let mut next = pos;
                    while matches!(src.get(next), Some(byte) if byte.is_ascii_whitespace()) {
                        next += 1;
                    }
                    let like_start = next;
                    while matches!(src.get(next), Some(byte) if byte.is_ascii_alphabetic()) {
                        next += 1;
                    }
                    let like = &src[like_start..next];
                    if like.eq_ignore_ascii_case(b"like") || like.eq_ignore_ascii_case(b"ilike") {
                        return Some((
                            word_start,
                            true,
                            like.eq_ignore_ascii_case(b"ilike"),
                            next - word_start,
                        ));
                    }
                }
                continue;
            }
            _ => {}
        }
        pos += 1;
    }
    None
}

fn top_level_boolean_operator_count(source: &[u8]) -> usize {
    let mut pos = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut count = 0;
    let mut between_and_pending = false;
    while pos < source.len() {
        match source[pos] {
            b'\'' => {
                if quoted && source.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            byte if !quoted && depth == 0 && (byte.is_ascii_alphabetic() || byte == b'_') => {
                let start = pos;
                pos += 1;
                while matches!(source.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
                {
                    pos += 1;
                }
                let word = &source[start..pos];
                let before_is_ident = start > 0
                    && (source[start - 1].is_ascii_alphanumeric() || source[start - 1] == b'_');
                let after_is_ident = matches!(source.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_');
                if !before_is_ident && !after_is_ident {
                    if word.eq_ignore_ascii_case(b"between") {
                        between_and_pending = true;
                    } else if word.eq_ignore_ascii_case(b"and") && between_and_pending {
                        between_and_pending = false;
                    } else if word.eq_ignore_ascii_case(b"and") || word.eq_ignore_ascii_case(b"or")
                    {
                        count += 1;
                    }
                }
                continue;
            }
            _ => {}
        }
        pos += 1;
    }
    count
}

fn has_top_level_boolean_operator(source: &[u8]) -> bool {
    top_level_boolean_operator_count(source) > 0
}

fn filter_expr_end(src: &[u8], start: usize) -> usize {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => {
                if depth == 0 {
                    return pos;
                }
                depth -= 1;
            }
            b';' if !quoted && depth == 0 => return pos,
            _ if !quoted && depth == 0 => {
                for keyword in ["and", "or", "group", "order", "limit"] {
                    let end = pos + keyword.len();
                    if src
                        .get(pos..end)
                        .is_some_and(|word| word.eq_ignore_ascii_case(keyword.as_bytes()))
                        && (pos == start || src[pos - 1].is_ascii_whitespace())
                        && (end == src.len() || src[end].is_ascii_whitespace())
                    {
                        return pos;
                    }
                }
            }
            _ => {}
        }
        pos += 1;
    }
    pos
}

pub(crate) fn parse_filter_operand(source: &str, position: usize) -> Result<FilterOperand, Error> {
    let leading = source.len() - source.trim_start().len();
    let source = source.trim();
    let position = position + leading;
    if let Some(inner) = unwrapped_filter_expr(source) {
        return parse_filter_operand(inner, position + 1);
    }
    let mut literal_parser = Parser {
        src: source.as_bytes(),
        pos: 0,
    };
    let numeric_start = source
        .strip_prefix('-')
        .unwrap_or(source)
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_digit);
    match literal_parser.parse_literal() {
        Ok(value) => {
            literal_parser.skip();
            if literal_parser.pos == source.len() {
                return Ok(FilterOperand::Literal(value));
            }
        }
        Err(_) if numeric_start => {
            let literal_start = usize::from(source.starts_with('-'));
            return Err(Error::at(
                format!("invalid constant: {}", &source[literal_start..]),
                position + literal_start,
            ));
        }
        Err(_) => {}
    }
    if let Some((pos, op)) = top_level_arithmetic(source.as_bytes()) {
        let left = parse_filter_operand(&source[..pos], position)?;
        let right = parse_filter_operand(&source[pos + 1..], position + pos + 1)?;
        return Ok(FilterOperand::Arithmetic(Box::new(FilterArithmetic {
            left,
            op,
            right,
            position: position + pos,
        })));
    }
    Ok(FilterOperand::Projection(Box::new(
        parse_filter_projection(source)?,
    )))
}

fn apply_projection_alias(item: &mut Proj, alias: &str) {
    match item {
        Proj::Concat(expr) => expr.label = alias.to_string(),
        Proj::Length(expr) => expr.label = alias.to_string(),
        Proj::Lower(expr) => expr.label = alias.to_string(),
        Proj::Upper(expr) => expr.label = alias.to_string(),
        Proj::SizePretty(expr) => expr.label = alias.to_string(),
        Proj::Typeof(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone().into();
            }
            expr.label = alias.to_string();
        }
        Proj::Not(expr) => {
            expr.label = alias.to_string();
        }
        Proj::And(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::Between(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::Eq(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::InList(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::CurrentSchema(expr) => expr.label = alias.to_string(),
        Proj::CurrentDatabase(expr) => expr.label = alias.to_string(),
        Proj::CurrentUser(expr) => expr.label = alias.to_string(),
        Proj::SessionUser(expr) => expr.label = alias.to_string(),
        Proj::CurrentCatalog(expr) => expr.label = alias.to_string(),
        Proj::Version(expr) => expr.label = alias.to_string(),
        Proj::PgIsInRecovery(expr) => expr.label = alias.to_string(),
        Proj::FormatType(expr) => expr.label = alias.to_string(),
        Proj::CurrentSetting(expr) => expr.label = alias.to_string(),
        Proj::PgGetPartKeyDef(expr) => expr.label = alias.to_string(),
        Proj::PgGetExpr(expr) => expr.label = alias.to_string(),
        Proj::PgGetUserById(expr) => expr.label = alias.to_string(),
        Proj::PgTableIsVisible(expr) => expr.label = alias.to_string(),
        Proj::PgAge(expr) => expr.label = alias.to_string(),
        Proj::Netmask(expr) => expr.label = alias.to_string(),
        Proj::GeoDistance(expr) => expr.label = alias.to_string(),
        Proj::GeoWithin(expr) => expr.label = alias.to_string(),
        Proj::WithinRadius(expr) => expr.label = alias.to_string(),
        Proj::WithinBox(expr) => expr.label = alias.to_string(),
        Proj::MakeGeohash(expr) => expr.label = alias.to_string(),
        Proj::Side(expr) => expr.label = alias.to_string(),
        Proj::Substr(expr) => expr.label = alias.to_string(),
        Proj::Trim(expr) => expr.label = alias.to_string(),
        Proj::Replace(expr) => expr.label = alias.to_string(),
        Proj::RegexpReplace(expr) => expr.label = alias.to_string(),
        Proj::Pad(expr) => expr.label = alias.to_string(),
        Proj::StartsWith(expr) => expr.label = alias.to_string(),
        Proj::SplitPart(expr) => expr.label = alias.to_string(),
        Proj::QuoteIdent(expr) => expr.label = alias.to_string(),
        Proj::Glob(expr) => expr.label = alias.to_string(),
        Proj::Ilike(expr) => expr.label = alias.to_string(),
        Proj::Like(expr) => expr.label = alias.to_string(),
        Proj::Switch(expr) => expr.label = alias.to_string(),
        Proj::Bar(expr) => expr.label = alias.to_string(),
        Proj::StrPos(expr) => expr.label = alias.to_string(),
        Proj::Mid(expr) => expr.label = alias.to_string(),
        Proj::Spread(expr) => expr.label = alias.to_string(),
        Proj::SpreadBps(expr) => expr.label = alias.to_string(),
        Proj::Wmid(expr) => expr.label = alias.to_string(),
        Proj::Round(expr) => expr.label = alias.to_string(),
        Proj::Floor(expr) => expr.label = alias.to_string(),
        Proj::Ceil(expr) => expr.label = alias.to_string(),
        Proj::Sqrt(expr) => expr.label = alias.to_string(),
        Proj::Power(expr) => expr.label = alias.to_string(),
        Proj::Sign(expr) => expr.label = alias.to_string(),
        Proj::Ln(expr) => expr.label = alias.to_string(),
        Proj::Log(expr) => expr.label = alias.to_string(),
        Proj::Exp(expr) => expr.label = alias.to_string(),
        Proj::Sin(expr) => expr.label = alias.to_string(),
        Proj::Cos(expr) => expr.label = alias.to_string(),
        Proj::Tan(expr) => expr.label = alias.to_string(),
        Proj::Asin(expr) => expr.label = alias.to_string(),
        Proj::Acos(expr) => expr.label = alias.to_string(),
        Proj::Atan(expr) => expr.label = alias.to_string(),
        Proj::Atan2(expr) => expr.label = alias.to_string(),
        Proj::Degrees(expr) => expr.label = alias.to_string(),
        Proj::Radians(expr) => expr.label = alias.to_string(),
        Proj::Cot(expr) => expr.label = alias.to_string(),
        Proj::Pi(expr) => expr.label = alias.to_string(),
        Proj::Greatest(expr) => expr.label = alias.to_string(),
        Proj::Least(expr) => expr.label = alias.to_string(),
        Proj::RoundDown(expr) => expr.label = alias.to_string(),
        Proj::RoundUp(expr) => expr.label = alias.to_string(),
        Proj::RoundHalfEven(expr) => expr.label = alias.to_string(),
        Proj::Coalesce(expr) => expr.label = alias.to_string(),
        Proj::NullIf(expr) => expr.label = alias.to_string(),
        Proj::Cast(expr) => expr.label = alias.to_string(),
        Proj::Year(expr) => expr.label = alias.to_string(),
        Proj::Month(expr) => expr.label = alias.to_string(),
        Proj::Day(expr) => expr.label = alias.to_string(),
        Proj::Hour(expr) => expr.label = alias.to_string(),
        Proj::Minute(expr) => expr.label = alias.to_string(),
        Proj::Second(expr) => expr.label = alias.to_string(),
        Proj::Millis(expr) => expr.label = alias.to_string(),
        Proj::Micros(expr) => expr.label = alias.to_string(),
        Proj::Nanos(expr) => expr.label = alias.to_string(),
        Proj::DateAdd(expr) => expr.label = alias.to_string(),
        Proj::DateDiff(expr) => expr.label = alias.to_string(),
        Proj::IsLeapYear(expr) => expr.label = alias.to_string(),
        Proj::DaysInMonth(expr) => expr.label = alias.to_string(),
        Proj::ToStr(expr) => expr.label = alias.to_string(),
        Proj::IsEndOfMonth(expr) => expr.label = alias.to_string(),
        Proj::DayOfWeek(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::WeekOfYear(expr) => {
            if expr.source_label.is_empty() {
                expr.source_label = expr.label.clone();
            }
            expr.label = alias.to_string();
        }
        Proj::DayOfWeekSundayFirst(expr) => expr.label = alias.to_string(),
        Proj::Md5(expr) => expr.label = alias.to_string(),
        Proj::Sha1(expr) => expr.label = alias.to_string(),
        Proj::Sha256(expr) => expr.label = alias.to_string(),
        Proj::ToUuid(expr) => expr.label = alias.to_string(),
        Proj::JsonExtract(expr) => expr.label = alias.to_string(),
        Proj::ToChar(expr) => expr.label = alias.to_string(),
        Proj::ToTimestamp(expr) => expr.label = alias.to_string(),
        Proj::ToDate(expr) => expr.label = alias.to_string(),
        Proj::ToUtc(expr) => expr.label = alias.to_string(),
        Proj::ToTimezone(expr) => expr.label = alias.to_string(),
        Proj::ToPgDate(expr) => expr.label = alias.to_string(),
        Proj::Extract(expr) => expr.label = alias.to_string(),
        Proj::TimestampFloor(expr) => expr.label = alias.to_string(),
        Proj::TimestampCeil(expr) => expr.label = alias.to_string(),
        Proj::Abs(expr) => expr.label = alias.to_string(),
        Proj::DateTrunc(expr) => expr.label = alias.to_string(),
        Proj::Arithmetic(expr) => expr.label = alias.to_string(),
        Proj::Column(_) => {}
        _ => {}
    }
}

fn select_item_end(source: &[u8], start: usize) -> usize {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < source.len() {
        match source[pos] {
            b'\'' => {
                if quoted && source.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            b',' if !quoted && depth == 0 => return pos,
            _ if !quoted && depth == 0 && source[pos..].len() >= 4 => {
                let from = &source[pos..pos + 4];
                if from.eq_ignore_ascii_case(b"from")
                    && (pos == start || source[pos - 1].is_ascii_whitespace())
                    && source
                        .get(pos + 4)
                        .is_none_or(|byte| byte.is_ascii_whitespace())
                {
                    return pos;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    source.len()
}

fn group_by_expr_end(source: &[u8], start: usize) -> usize {
    let mut pos = start;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos < source.len() {
        match source[pos] {
            b'\'' => {
                if quoted && source.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            b',' if !quoted && depth == 0 => return pos,
            b';' if !quoted && depth == 0 => return pos,
            _ if !quoted && depth == 0 => {
                for keyword in ["order", "limit"] {
                    let end = pos + keyword.len();
                    if source
                        .get(pos..end)
                        .is_some_and(|word| word.eq_ignore_ascii_case(keyword.as_bytes()))
                        && pos > start
                        && source[pos - 1].is_ascii_whitespace()
                        && source
                            .get(end)
                            .is_none_or(|byte| byte.is_ascii_whitespace())
                    {
                        return pos;
                    }
                }
            }
            _ => {}
        }
        pos += 1;
    }
    pos
}

fn group_by_between_keyword(source: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut pos = 0;
    let mut depth = 0usize;
    let mut quote = None;
    while pos < bytes.len() {
        let byte = bytes[pos];
        if let Some(delimiter) = quote {
            if byte == delimiter {
                if bytes.get(pos + 1) == Some(&delimiter) {
                    pos += 2;
                    continue;
                }
                quote = None;
            }
            pos += 1;
            continue;
        }
        if byte == b'\'' || byte == b'"' {
            quote = Some(byte);
            pos += 1;
            continue;
        }
        if byte == b'(' {
            depth += 1;
            pos += 1;
            continue;
        }
        if byte == b')' {
            depth = depth.saturating_sub(1);
            pos += 1;
            continue;
        }
        let end = pos + b"between".len();
        if depth == 0
            && bytes
                .get(pos..end)
                .is_some_and(|word| word.eq_ignore_ascii_case(b"between"))
        {
            let before = pos.checked_sub(1).and_then(|index| bytes.get(index));
            let after = bytes.get(end);
            let is_identifier = |byte: Option<&u8>| {
                byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            };
            if !is_identifier(before) && !is_identifier(after) && after != Some(&b'(') {
                return Some(pos);
            }
        }
        pos += 1;
    }
    None
}

fn aggregate_call_offset(source: &[u8]) -> Option<usize> {
    let mut parser = Parser {
        src: source,
        pos: 0,
    };
    let mut single_quoted = false;
    let mut double_quoted = false;
    while let Some(&byte) = source.get(parser.pos) {
        if byte == b'\'' && !double_quoted {
            if single_quoted && source.get(parser.pos + 1) == Some(&b'\'') {
                parser.pos += 2;
                continue;
            }
            single_quoted = !single_quoted;
            parser.pos += 1;
            continue;
        }
        if byte == b'"' && !single_quoted {
            if double_quoted && source.get(parser.pos + 1) == Some(&b'"') {
                parser.pos += 2;
                continue;
            }
            double_quoted = !double_quoted;
            parser.pos += 1;
            continue;
        }
        if !single_quoted && !double_quoted && (byte.is_ascii_alphabetic() || byte == b'_') {
            if parser.starts_aggregate_call() {
                return Some(parser.pos);
            }
            parser.pos += 1;
            while source
                .get(parser.pos)
                .is_some_and(|next| next.is_ascii_alphanumeric() || *next == b'_')
            {
                parser.pos += 1;
            }
        } else {
            parser.pos += 1;
        }
    }
    None
}

fn normalize_typeof_cast_labels(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut pos = 0usize;
    let mut quoted = false;
    while pos < bytes.len() {
        if bytes[pos] == b'\'' {
            output.push(bytes[pos]);
            if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                output.push(b'\'');
                pos += 2;
                continue;
            }
            quoted = !quoted;
            pos += 1;
            continue;
        }
        if !quoted && bytes.get(pos..pos + 2) == Some(b"::") {
            output.extend_from_slice(b"::");
            pos += 2;
            let start = pos;
            while matches!(bytes.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                pos += 1;
            }
            let type_name = &bytes[start..pos];
            if type_name.eq_ignore_ascii_case(b"float") {
                output.extend_from_slice(b"DOUBLE");
            } else {
                output.extend_from_slice(type_name);
            }
            continue;
        }
        output.push(bytes[pos]);
        pos += 1;
    }
    String::from_utf8(output).expect("cast label normalization preserves UTF-8")
}

fn normalize_select_arithmetic_label(source: &str) -> String {
    let source = normalize_typeof_cast_labels(source);
    let bytes = source.as_bytes();
    let mut output = Vec::with_capacity(bytes.len() + 8);
    let mut pos = 0usize;
    let mut quoted = false;
    while pos < bytes.len() {
        if bytes[pos] == b'\'' {
            output.push(bytes[pos]);
            if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                output.push(b'\'');
                pos += 2;
                continue;
            }
            quoted = !quoted;
            pos += 1;
            continue;
        }
        if !quoted && matches!(bytes[pos], b'-' | b'~') {
            let previous = output
                .iter()
                .rev()
                .copied()
                .find(|byte| !byte.is_ascii_whitespace());
            let unary =
                previous.is_none_or(|byte| matches!(byte, b'(' | b'+' | b'-' | b'*' | b'/' | b'%'));
            let mut end = pos + 1;
            while matches!(bytes.get(end), Some(b' ' | b'\t' | b'\n' | b'\r')) {
                end += 1;
            }
            if unary
                && matches!(bytes.get(end), Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_')
            {
                let mut ident_end = end + 1;
                while matches!(bytes.get(ident_end), Some(byte) if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
                {
                    ident_end += 1;
                }
                output.push(bytes[pos]);
                output.push(b'(');
                output.extend_from_slice(&bytes[end..ident_end]);
                output.push(b')');
                pos = ident_end;
                continue;
            }
        }
        output.push(bytes[pos]);
        pos += 1;
    }
    String::from_utf8(output).expect("SQL source is UTF-8")
}

fn split_select_alias(item: &str) -> (&str, Option<&str>) {
    let bytes = item.as_bytes();
    let mut pos = 0usize;
    let mut depth = 0usize;
    let mut quoted = false;
    while pos + 2 <= bytes.len() {
        match bytes[pos] {
            b'\'' => {
                if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            b'a' | b'A'
                if !quoted
                    && depth == 0
                    && bytes[pos + 1].eq_ignore_ascii_case(&b's')
                    && (pos == 0 || bytes[pos - 1].is_ascii_whitespace())
                    && bytes.get(pos + 2).is_some_and(u8::is_ascii_whitespace) =>
            {
                let expression = item[..pos].trim_end();
                let alias = item[pos + 2..].trim();
                if !expression.is_empty() && !alias.is_empty() {
                    return (expression, Some(alias));
                }
            }
            _ => {}
        }
        pos += 1;
    }
    (item, None)
}

fn unwrapped_filter_expr(source: &str) -> Option<&str> {
    let bytes = source.as_bytes();
    if bytes.first() != Some(&b'(') || bytes.last() != Some(&b')') {
        return None;
    }
    let mut depth = 0usize;
    let mut quoted = false;
    let mut pos = 0usize;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\'' => {
                if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => {
                depth = depth.checked_sub(1)?;
                if depth == 0 && pos + 1 != bytes.len() {
                    return None;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    (depth == 0 && !quoted).then(|| &source[1..source.len() - 1])
}

fn top_level_arithmetic(src: &[u8]) -> Option<(usize, ArithOp)> {
    let mut pos = 0usize;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut best: Option<(usize, u8, ArithOp)> = None;
    while pos < src.len() {
        match src[pos] {
            b'\'' => {
                if quoted && src.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => depth += 1,
            b')' if !quoted => depth = depth.saturating_sub(1),
            op @ (b'+' | b'-' | b'*' | b'/' | b'%') if !quoted && depth == 0 => {
                if matches!(op, b'+' | b'-')
                    && pos > 0
                    && matches!(src[pos - 1], b'e' | b'E')
                    && pos > 1
                    && src[pos - 2].is_ascii_digit()
                {
                    pos += 1;
                    continue;
                }
                let previous = src[..pos]
                    .iter()
                    .rposition(|byte| !byte.is_ascii_whitespace())
                    .map(|index| src[index]);
                let next = src[pos + 1..]
                    .iter()
                    .position(|byte| !byte.is_ascii_whitespace())
                    .map(|offset| src[pos + 1 + offset]);
                let unary = previous
                    .is_none_or(|byte| matches!(byte, b'(' | b'+' | b'-' | b'*' | b'/' | b'%'))
                    || next.is_none();
                if !unary {
                    let (priority, arith) = match op {
                        b'+' => (1, ArithOp::Add),
                        b'-' => (1, ArithOp::Sub),
                        b'*' => (2, ArithOp::Mul),
                        b'/' => (2, ArithOp::Div),
                        b'%' => (2, ArithOp::Mod),
                        _ => unreachable!(),
                    };
                    if best.is_none_or(|(best_pos, best_priority, _)| {
                        priority < best_priority || (priority == best_priority && pos > best_pos)
                    }) {
                        best = Some((pos, priority, arith));
                    }
                }
            }
            _ => {}
        }
        pos += 1;
    }
    best.map(|(pos, _, op)| (pos, op))
}

fn needs_extended_filter_expr(source: &str) -> bool {
    let mut pos = 0usize;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut arithmetic_count = 0usize;
    let mut has_call = false;
    let mut has_cast = false;
    let bytes = source.as_bytes();
    while pos < bytes.len() {
        match bytes[pos] {
            b'\'' => {
                if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            b'(' if !quoted => {
                if pos > 0
                    && (bytes[pos - 1].is_ascii_alphanumeric()
                        || matches!(bytes[pos - 1], b'_' | b'.'))
                    && !is_in_operator_before(bytes, pos)
                {
                    has_call = true;
                }
                depth += 1;
            }
            b')' if !quoted => depth = depth.saturating_sub(1),
            b':' if !quoted && bytes.get(pos + 1) == Some(&b':') => {
                has_cast = true;
                pos += 1;
            }
            op @ (b'+' | b'-' | b'*' | b'/' | b'%') if !quoted => {
                if matches!(op, b'+' | b'-')
                    && pos > 0
                    && matches!(bytes[pos - 1], b'e' | b'E')
                    && pos > 1
                    && bytes[pos - 2].is_ascii_digit()
                {
                    pos += 1;
                    continue;
                }
                let previous = bytes[..pos]
                    .iter()
                    .rposition(|byte| !byte.is_ascii_whitespace())
                    .map(|index| bytes[index]);
                if previous.is_some_and(|byte| {
                    !matches!(byte, b'(' | b',' | b'+' | b'-' | b'*' | b'/' | b'%')
                }) {
                    let _ = op;
                    arithmetic_count += 1;
                }
            }
            _ => {}
        }
        pos += 1;
    }
    has_call
        || has_cast
        || arithmetic_count > 0
        || contains_scientific_literal(bytes)
        || (unwrapped_filter_expr(source).is_some() && arithmetic_count > 0)
}

fn contains_scientific_literal(bytes: &[u8]) -> bool {
    let mut pos = 0usize;
    let mut quoted = false;
    while pos < bytes.len() {
        match bytes[pos] {
            b'\'' => {
                if quoted && bytes.get(pos + 1) == Some(&b'\'') {
                    pos += 2;
                    continue;
                }
                quoted = !quoted;
            }
            byte if !quoted && byte.is_ascii_digit() => {
                let mut end = pos + 1;
                while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
                    end += 1;
                }
                if bytes.get(end) == Some(&b'.') {
                    end += 1;
                    while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
                        end += 1;
                    }
                }
                if matches!(bytes.get(end), Some(b'e' | b'E')) {
                    return true;
                }
                pos = end;
            }
            _ => {}
        }
        pos += 1;
    }
    false
}

fn is_in_operator_before(src: &[u8], paren_pos: usize) -> bool {
    let mut end = paren_pos;
    while end > 0 && src[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && (src[start - 1].is_ascii_alphanumeric() || src[start - 1] == b'_') {
        start -= 1;
    }
    src.get(start..end)
        .is_some_and(|word| word.eq_ignore_ascii_case(b"in"))
}

fn long_sequence_count(args: &[ConcatArg]) -> Result<i64, Error> {
    let [arg] = args else {
        return Err(Error::new("invalid arguments"));
    };
    match arg {
        ConcatArg::Column(_) => Err(Error::new("constant expected")),
        ConcatArg::Literal(Scalar::Null) => Ok(0),
        ConcatArg::Literal(Scalar::Boolean(_)) => Err(Error::new("invalid arguments")),
        ConcatArg::Literal(Scalar::Double(_)) => {
            Err(Error::new("argument type DOUBLE is not supported"))
        }
        ConcatArg::Literal(Scalar::Float(_)) => {
            Err(Error::new("argument type FLOAT is not supported"))
        }
        ConcatArg::Literal(Scalar::Long(number)) => Ok((*number).max(0)),
        ConcatArg::Literal(Scalar::Int(number)) => Ok(i64::from(*number).max(0)),
        ConcatArg::Literal(Scalar::Byte(number)) => Ok(i64::from(*number).max(0)),
        ConcatArg::Literal(Scalar::Short(number)) => Ok(i64::from(*number).max(0)),
        ConcatArg::Literal(Scalar::Text(text)) => match text.parse::<i64>() {
            Ok(number) => Ok(number.max(0)),
            Err(_) => Err(Error::new("invalid arguments")),
        },
        ConcatArg::Literal(Scalar::Char(_) | Scalar::Date(_) | Scalar::Timestamp(_)) => {
            Err(Error::new("invalid arguments"))
        }
    }
}

fn generate_series_spec(args: &[ConcatArg]) -> Result<GenerateSeries, Error> {
    if args.is_empty() {
        return Err(Error::new(
            "function `generate_series` requires arguments".to_string(),
        ));
    }
    if args.len() != 2 && args.len() != 3 {
        return Err(Error::new(format!(
            "there is no matching function `generate_series` with the argument types: ({})",
            series_type_list(args)
        )));
    }
    if args.len() == 3 {
        if let (Some(start), Some(end)) = (series_clock_arg(&args[0]), series_clock_arg(&args[1])) {
            if matches!(
                &args[2],
                ConcatArg::Literal(Scalar::Null) | ConcatArg::Column(_)
            ) {
                return Err(Error::new(
                    "step argument must be a non-null constant or bind variable constant",
                ));
            }
            if let ConcatArg::Literal(Scalar::Text(text)) = &args[2] {
                if text.is_empty() {
                    return Err(Error::new("empty step"));
                }
                let Some(step) = series_period(text) else {
                    return Err(Error::new(format!("invalid period [period={text}]")));
                };
                let zero = match step {
                    SeriesStep::Fixed(width) => width == 0,
                    SeriesStep::Months(count) | SeriesStep::Years(count) => count == 0,
                };
                if zero {
                    return Err(Error::new("step cannot be zero"));
                }
                return Ok(GenerateSeries::Timestamp { start, end, step });
            }
        }
    }
    if args.len() == 2 {
        if let ConcatArg::Literal(Scalar::Text(text)) = &args[0] {
            if series_clock(text).is_some() {
                return Err(Error::new(format!(
                    "inconvertible value: `{text}` [STRING -> DOUBLE]"
                )));
            }
        }
    }
    if args.iter().all(series_long_ok) {
        let start = series_i64(&args[0], "start")?;
        let end = series_i64(&args[1], "end")?;
        let step = if args.len() == 3 {
            series_i64(&args[2], "step")?
        } else {
            1
        };
        if step == 0 {
            return Err(Error::new("step cannot be zero"));
        }
        return Ok(GenerateSeries::Long { start, end, step });
    }
    if args.iter().all(series_double_ok) {
        let start = series_f64(&args[0], "start")?;
        let end = series_f64(&args[1], "end")?;
        let step = if args.len() == 3 {
            series_f64(&args[2], "step")?
        } else {
            1.0
        };
        if step == 0.0 {
            return Err(Error::new("step cannot be zero"));
        }
        return Ok(GenerateSeries::Double { start, end, step });
    }
    Err(Error::new(format!(
        "there is no matching function `generate_series` with the argument types: ({})",
        series_type_list(args)
    )))
}

fn series_clock_arg(arg: &ConcatArg) -> Option<i64> {
    match arg {
        ConcatArg::Literal(Scalar::Text(text)) => series_clock(text),
        _ => None,
    }
}

fn series_clock(text: &str) -> Option<i64> {
    if let Some(micros) = parse_timestamp_micros(text) {
        return Some(micros);
    }
    if text.len() != 10 {
        return None;
    }
    let mut parts = text.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    unix_days_from_civil(year, month, day)?.checked_mul(86_400_000_000)
}

fn series_period(text: &str) -> Option<SeriesStep> {
    let (sign, rest) = if let Some(rest) = text.strip_prefix('-') {
        (-1, rest)
    } else {
        (1, text)
    };
    let unit = rest.chars().last()?;
    let digits = &rest[..rest.len() - unit.len_utf8()];
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let count: i64 = digits.parse().ok()?;
    let signed = count.checked_mul(sign)?;
    match unit {
        's' => Some(SeriesStep::Fixed(signed.checked_mul(1_000_000)?)),
        'm' => Some(SeriesStep::Fixed(signed.checked_mul(60_000_000)?)),
        'h' | 'H' => Some(SeriesStep::Fixed(signed.checked_mul(3_600_000_000)?)),
        'd' => Some(SeriesStep::Fixed(signed.checked_mul(86_400_000_000)?)),
        'w' => Some(SeriesStep::Fixed(signed.checked_mul(604_800_000_000)?)),
        'T' => Some(SeriesStep::Fixed(signed.checked_mul(1_000)?)),
        'U' | 'u' => Some(SeriesStep::Fixed(signed)),
        'n' => Some(SeriesStep::Fixed(signed / 1_000)),
        'M' => Some(SeriesStep::Months(i32::try_from(signed).ok()?)),
        'y' => Some(SeriesStep::Years(i32::try_from(signed).ok()?)),
        _ => None,
    }
}

fn series_type_list(args: &[ConcatArg]) -> String {
    args.iter()
        .map(series_type_name)
        .collect::<Vec<_>>()
        .join(", ")
}

fn series_type_name(arg: &ConcatArg) -> &'static str {
    match arg {
        ConcatArg::Column(_) => "STRING",
        ConcatArg::Literal(Scalar::Null) => "NULL",
        ConcatArg::Literal(Scalar::Boolean(_)) => "BOOLEAN",
        ConcatArg::Literal(Scalar::Byte(_)) => "BYTE",
        ConcatArg::Literal(Scalar::Short(_)) => "SHORT",
        ConcatArg::Literal(Scalar::Char(_)) => "CHAR",
        ConcatArg::Literal(Scalar::Int(_)) => "INT",
        ConcatArg::Literal(Scalar::Long(number)) => {
            if *number >= i64::from(i32::MIN) && *number <= i64::from(i32::MAX) {
                "INT"
            } else {
                "LONG"
            }
        }
        ConcatArg::Literal(Scalar::Float(_)) => "FLOAT",
        ConcatArg::Literal(Scalar::Double(_)) => "DOUBLE",
        ConcatArg::Literal(Scalar::Date(_)) => "DATE",
        ConcatArg::Literal(Scalar::Timestamp(_)) => "TIMESTAMP",
        ConcatArg::Literal(Scalar::Text(text)) => {
            if text.chars().count() == 1 {
                "CHAR"
            } else {
                "STRING"
            }
        }
    }
}

fn series_long_ok(arg: &ConcatArg) -> bool {
    matches!(
        series_type_name(arg),
        "LONG" | "INT" | "BYTE" | "SHORT" | "CHAR" | "STRING" | "DATE" | "TIMESTAMP" | "NULL"
    ) || matches!(arg, ConcatArg::Column(_))
}

fn series_double_ok(arg: &ConcatArg) -> bool {
    matches!(
        series_type_name(arg),
        "DOUBLE"
            | "FLOAT"
            | "LONG"
            | "INT"
            | "BYTE"
            | "SHORT"
            | "CHAR"
            | "STRING"
            | "DATE"
            | "TIMESTAMP"
            | "NULL"
    )
}

fn series_constant(arg: &ConcatArg, place: &str) -> Result<(), Error> {
    if matches!(arg, ConcatArg::Column(_) | ConcatArg::Literal(Scalar::Null)) {
        Err(Error::new(format!(
            "{place} argument must be a non-null constant or bind variable constant"
        )))
    } else {
        Ok(())
    }
}

fn series_i64(arg: &ConcatArg, place: &str) -> Result<i64, Error> {
    series_constant(arg, place)?;
    match arg {
        ConcatArg::Literal(Scalar::Long(number)) => Ok(*number),
        ConcatArg::Literal(Scalar::Int(number)) => Ok(i64::from(*number)),
        ConcatArg::Literal(Scalar::Byte(number)) => Ok(i64::from(*number)),
        ConcatArg::Literal(Scalar::Short(number)) => Ok(i64::from(*number)),
        ConcatArg::Literal(Scalar::Text(text)) => text
            .parse::<i64>()
            .map_err(|_| Error::new("invalid arguments".to_string())),
        _ => Err(Error::new("invalid arguments")),
    }
}

fn series_f64(arg: &ConcatArg, place: &str) -> Result<f64, Error> {
    series_constant(arg, place)?;
    match arg {
        ConcatArg::Literal(Scalar::Double(number)) => Ok(*number),
        ConcatArg::Literal(Scalar::Float(number)) => Ok(f64::from(*number)),
        ConcatArg::Literal(Scalar::Long(number)) => Ok(*number as f64),
        ConcatArg::Literal(Scalar::Int(number)) => Ok(f64::from(*number)),
        ConcatArg::Literal(Scalar::Byte(number)) => Ok(f64::from(*number)),
        ConcatArg::Literal(Scalar::Short(number)) => Ok(f64::from(*number)),
        ConcatArg::Literal(Scalar::Text(text)) => text
            .parse::<f64>()
            .map_err(|_| Error::new("invalid arguments".to_string())),
        _ => Err(Error::new("invalid arguments")),
    }
}

enum SelectPiece {
    Column(String),
    Arithmetic(ArithmeticExpr),
    Aggregate(AggregateExpr),
    Concat(ConcatExpr),
    Length(LengthExpr),
    Lower(LowerExpr),
    Upper(UpperExpr),
    Side(SideExpr),
    Substr(SubstrExpr),
    Trim(TrimExpr),
    Replace(ReplaceExpr),
    RegexpReplace(RegexpReplaceExpr),
    StartsWith(StartsWithExpr),
    Glob(GlobExpr),
    Ilike(IlikeExpr),
    Like(LikeExpr),
    Switch(SwitchExpr),
    Bar(BarExpr),
    TimestampSequence(TimestampSequenceExpr),
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
    Nanos(NanosExpr),
    ToUtc(ToUtcExpr),
    ToTimezone(ToTimezoneExpr),
    ToPgDate(ToPgDateExpr),
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
    Mid(MidExpr),
    Spread(SpreadExpr),
    SpreadBps(SpreadBpsExpr),
    Wmid(WmidExpr),
    Between(BetweenExpr),
    InList(InListExpr),
    Eq(EqExpr),
    Add(AddExpr),
    Neg(NegExpr),
    BitNot(BitNotExpr),
    And(AndExpr),
    Ln(LnExpr),
    Log(LogExpr),
    Exp(ExpExpr),
    Pi(PiExpr),
    CurrentSchema(CurrentSchemaExpr),
    CurrentDatabase(CurrentDatabaseExpr),
    CurrentUser(CurrentUserExpr),
    SessionUser(SessionUserExpr),
    CurrentCatalog(CurrentCatalogExpr),
    Version(VersionExpr),
    PgIsInRecovery(PgIsInRecoveryExpr),
    FormatType(FormatTypeExpr),
    CurrentSetting(CurrentSettingExpr),
    PgGetPartKeyDef(PgGetPartKeyDefExpr),
    PgGetUserById(PgGetUserByIdExpr),
    PgTableIsVisible(PgTableIsVisibleExpr),
    PgAge(PgAgeExpr),
    PgGetExpr(PgGetExprExpr),
    Netmask(NetmaskExpr),
    GeoDistance(GeoDistanceExpr),
    GeoWithin(GeoWithinExpr),
    WithinRadius(WithinRadiusExpr),
    WithinBox(WithinBoxExpr),
    MakeGeohash(MakeGeohashExpr),
    ToUuid(ToUuidExpr),
    JsonExtract(JsonExtractExpr),
    ToChar(ToCharExpr),
    Greatest(GreatestExpr),
    Least(LeastExpr),
}

struct Parser<'a> {
    src: &'a [u8],
    pos: usize,
}

fn qualified_ident_len(bytes: &[u8]) -> Option<usize> {
    let first = *bytes.first()?;
    if !first.is_ascii_alphabetic() && first != b'_' {
        return None;
    }
    let mut index = 1;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        index += 1;
    }
    Some(index)
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

    fn parse_equi_join(&mut self) -> Result<Option<EquiJoin>, Error> {
        self.skip();
        let left_outer = self.eat_keyword("left");
        let right_outer = if left_outer {
            false
        } else {
            self.eat_keyword("right")
        };
        let full = if left_outer || right_outer {
            false
        } else {
            self.eat_keyword("full")
        };
        let inner = if left_outer || right_outer || full {
            false
        } else {
            self.eat_keyword("inner")
        };
        let cross = if left_outer || right_outer || full || inner {
            false
        } else {
            self.eat_keyword("cross")
        };
        let joined = if left_outer || right_outer || full || inner || cross {
            self.expect_keyword("join")?;
            true
        } else {
            self.eat_keyword("join")
        };
        if !joined {
            return Ok(None);
        }
        let right_table = self.ident()?;
        self.skip();
        if self.eat_keyword("as") {
            self.skip();
        }
        let right_alias = self.ident()?;
        if cross {
            if self.eat_keyword("on") {
                return Err(Error::new("Cross joins cannot have join clauses"));
            }
            return Ok(Some(EquiJoin {
                left_alias: String::new(),
                right_table,
                right_alias,
                left_column: String::new(),
                right_column: String::new(),
                left_outer: false,
                right_outer: false,
                cross: true,
            }));
        }
        if !self.eat_keyword("on") {
            return Err(Error::new("'on' expected"));
        }
        let left_name = self.qualified_ident()?;
        self.skip();
        if !self.eat(b'=') {
            return Err(Error::new("unsupported: expected ="));
        }
        let right_name = self.qualified_ident()?;
        Ok(Some(EquiJoin {
            left_alias: String::new(),
            right_table,
            right_alias,
            left_column: left_name,
            right_column: right_name
                .rsplit('.')
                .next()
                .unwrap_or(&right_name)
                .to_string(),
            left_outer: left_outer || full,
            right_outer: right_outer || full,
            cross: false,
        }))
    }

    fn clause_keyword_ahead(&mut self) -> bool {
        [
            "where", "group", "order", "limit", "join", "inner", "left", "right", "full", "cross",
            "as",
        ]
        .into_iter()
        .any(|word| {
            let pos = self.pos;
            let found = self.eat_keyword(word);
            self.pos = pos;
            found
        })
    }

    fn ident_ahead(&self) -> bool {
        let mut pos = self.pos;
        while matches!(self.src.get(pos), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            pos += 1;
        }
        matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_'
        )
    }

    fn qualified_ident(&mut self) -> Result<String, Error> {
        let mut name = self.ident()?;
        self.skip();
        if self.eat(b'.') {
            name.push('.');
            name.push_str(&self.ident()?);
        }
        Ok(name)
    }

    fn qualified_ident_spelling(&mut self) -> Result<String, Error> {
        let mut name = self.ident_spelling()?;
        self.skip();
        if self.eat(b'.') {
            name.push('.');
            name.push_str(&self.ident_spelling()?);
        }
        Ok(name)
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
        let first_item_end = select_item_end(self.src, self.pos);
        let first_item = std::str::from_utf8(&self.src[self.pos..first_item_end]).unwrap_or("");
        if !self.starts_value()
            && !self.starts_unary_minus()
            && !self.starts_tilde()
            && top_level_arithmetic(first_item.trim().as_bytes()).is_none()
        {
            return Err(Error::new(
                "unsupported: SELECT list must name columns or `*`",
            ));
        }
        let mut pieces = Vec::new();
        let mut piece_positions = Vec::new();
        let mut piece_aliases = Vec::new();
        loop {
            self.skip();
            let item_position = self.pos;
            let item_end = select_item_end(self.src, self.pos);
            let item = std::str::from_utf8(&self.src[self.pos..item_end])
                .unwrap_or("")
                .trim();
            let (expression, alias) = split_select_alias(item);
            if is_boolean_projection(expression) {
                pieces.push(SelectPiece::Not(parse_boolean_projection(
                    expression, self.pos,
                )?));
                self.pos = item_end;
            } else if top_level_arithmetic(expression.as_bytes()).is_some() {
                let operand = parse_filter_operand(expression, self.pos)?;
                if !matches!(operand, FilterOperand::Arithmetic(_)) {
                    return Err(Error::new("unsupported: expected arithmetic expression"));
                }
                pieces.push(SelectPiece::Arithmetic(ArithmeticExpr {
                    label: alias
                        .map(str::to_string)
                        .unwrap_or_else(|| normalize_select_arithmetic_label(expression)),
                    source: expression.to_string(),
                    position: self.pos,
                    operand,
                }));
                self.pos = item_end;
            } else if self.starts_named_call("generate_series") {
                let (label, _) = self.parse_generate_series_call()?;
                return Err(Error::new(format!(
                    "cursor function cannot be used as a column [column={label}]"
                )));
            } else if self.starts_named_call("long_sequence") {
                let (label, _) = self.parse_long_sequence_call()?;
                return Err(Error::new(format!(
                    "cursor function cannot be used as a column [column={label}]"
                )));
            } else if self.starts_aggregate_call() {
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
            } else if self.starts_regexp_replace_call() {
                pieces.push(SelectPiece::RegexpReplace(self.parse_regexp_replace()?));
            } else if self.starts_starts_with_call() {
                pieces.push(SelectPiece::StartsWith(self.parse_starts_with()?));
            } else if self.starts_glob_call() {
                pieces.push(SelectPiece::Glob(self.parse_glob()?));
            } else if self.starts_ilike_call() {
                pieces.push(SelectPiece::Ilike(self.parse_ilike()?));
            } else if self.starts_like_call() {
                pieces.push(SelectPiece::Like(self.parse_like()?));
            } else if self.starts_switch_call() {
                pieces.push(SelectPiece::Switch(self.parse_switch()?));
            } else if self.starts_bar_call() {
                pieces.push(SelectPiece::Bar(self.parse_bar()?));
            } else if self.starts_timestamp_sequence_call() {
                pieces.push(SelectPiece::TimestampSequence(
                    self.parse_timestamp_sequence()?,
                ));
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
            } else if self.starts_nanos_call() {
                pieces.push(SelectPiece::Nanos(self.parse_nanos()?));
            } else if self.starts_to_utc_call() {
                pieces.push(SelectPiece::ToUtc(self.parse_to_utc()?));
            } else if self.starts_to_timezone_call() {
                pieces.push(SelectPiece::ToTimezone(self.parse_to_timezone()?));
            } else if self.starts_to_pg_date_call() {
                pieces.push(SelectPiece::ToPgDate(self.parse_to_pg_date()?));
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
            } else if self.starts_not_prefix() {
                pieces.push(SelectPiece::Not(self.parse_not_prefix()?));
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
            } else if self.starts_mid_call() {
                pieces.push(SelectPiece::Mid(self.parse_mid()?));
            } else if self.starts_wmid_call() {
                pieces.push(SelectPiece::Wmid(self.parse_wmid()?));
            } else if self.starts_between_call() {
                pieces.push(SelectPiece::Between(self.parse_between()?));
            } else if self.starts_spread_bps_call() {
                pieces.push(SelectPiece::SpreadBps(self.parse_spread_bps()?));
            } else if self.starts_spread_call() {
                pieces.push(SelectPiece::Spread(self.parse_spread()?));
            } else if self.starts_ln_call() {
                pieces.push(SelectPiece::Ln(self.parse_ln()?));
            } else if self.starts_log_call() {
                pieces.push(SelectPiece::Log(self.parse_log()?));
            } else if self.starts_exp_call() {
                pieces.push(SelectPiece::Exp(self.parse_exp()?));
            } else if self.starts_pi_call() {
                pieces.push(SelectPiece::Pi(self.parse_pi()?));
            } else if self.starts_current_schema_call() {
                pieces.push(SelectPiece::CurrentSchema(self.parse_current_schema()?));
            } else if self.starts_current_database_call() {
                pieces.push(SelectPiece::CurrentDatabase(self.parse_current_database()?));
            } else if self.starts_current_user_call() {
                pieces.push(SelectPiece::CurrentUser(self.parse_current_user()?));
            } else if self.starts_session_user_call() {
                pieces.push(SelectPiece::SessionUser(self.parse_session_user()?));
            } else if self.starts_current_catalog_call() {
                pieces.push(SelectPiece::CurrentCatalog(self.parse_current_catalog()?));
            } else if self.starts_version_call() {
                pieces.push(SelectPiece::Version(self.parse_version()?));
            } else if self.starts_pg_is_in_recovery_call() {
                pieces.push(SelectPiece::PgIsInRecovery(self.parse_pg_is_in_recovery()?));
            } else if self.starts_format_type_call() {
                pieces.push(SelectPiece::FormatType(self.parse_format_type()?));
            } else if self.starts_current_setting_call() {
                pieces.push(SelectPiece::CurrentSetting(self.parse_current_setting()?));
            } else if self.starts_pg_get_partkeydef_call() {
                pieces.push(SelectPiece::PgGetPartKeyDef(
                    self.parse_pg_get_partkeydef()?,
                ));
            } else if self.starts_pg_get_userbyid_call() {
                pieces.push(SelectPiece::PgGetUserById(self.parse_pg_get_userbyid()?));
            } else if self.starts_pg_table_is_visible_call() {
                pieces.push(SelectPiece::PgTableIsVisible(
                    self.parse_pg_table_is_visible()?,
                ));
            } else if self.starts_pg_age_call() {
                pieces.push(SelectPiece::PgAge(self.parse_pg_age()?));
            } else if self.starts_pg_get_expr_call() {
                pieces.push(SelectPiece::PgGetExpr(self.parse_pg_get_expr()?));
            } else if self.starts_netmask_call() {
                pieces.push(SelectPiece::Netmask(self.parse_netmask()?));
            } else if self.starts_geo_distance_call() {
                pieces.push(SelectPiece::GeoDistance(self.parse_geo_distance()?));
            } else if self.starts_geo_within_call() {
                pieces.push(SelectPiece::GeoWithin(self.parse_geo_within()?));
            } else if self.starts_within_radius_call() {
                pieces.push(SelectPiece::WithinRadius(self.parse_within_radius()?));
            } else if self.starts_within_box_call() {
                pieces.push(SelectPiece::WithinBox(self.parse_within_box()?));
            } else if self.starts_make_geohash_call() {
                pieces.push(SelectPiece::MakeGeohash(self.parse_make_geohash()?));
            } else if self.starts_to_uuid_call() {
                pieces.push(SelectPiece::ToUuid(self.parse_to_uuid()?));
            } else if self.starts_json_extract_call() {
                pieces.push(SelectPiece::JsonExtract(self.parse_json_extract()?));
            } else if self.starts_to_char_call() {
                pieces.push(SelectPiece::ToChar(self.parse_to_char()?));
            } else if self.starts_from_base64_call() {
                self.parse_from_base64()?;
                return Err(Error::new("unsupported: binary value".to_string()));
            } else if self.starts_greatest_call() {
                pieces.push(SelectPiece::Greatest(self.parse_greatest()?));
            } else if self.starts_least_call() {
                pieces.push(SelectPiece::Least(self.parse_least()?));
            } else if self.starts_unary_minus() {
                pieces.push(SelectPiece::Neg(self.parse_unary_minus()?));
            } else if self.starts_tilde() {
                pieces.push(SelectPiece::BitNot(self.parse_tilde()?));
            } else {
                let mut name = self.ident_spelling()?;
                self.skip();
                if self.eat(b'.') {
                    name.push('.');
                    name.push_str(&self.ident_spelling()?);
                    self.skip();
                }
                if self.starts_colon_cast() {
                    pieces.push(SelectPiece::Cast(self.parse_colon_cast(name)?));
                } else if self.starts_pipes() {
                    pieces.push(SelectPiece::Concat(self.parse_concat_op(name)?));
                } else if self.starts_not_in() {
                    pieces.push(SelectPiece::InList(self.parse_not_in_list(name)?));
                } else if self.eat_keyword("in") {
                    pieces.push(SelectPiece::InList(self.parse_in_list(name)?));
                } else if self.eat_keyword("and") {
                    pieces.push(SelectPiece::And(self.parse_and(name, BoolOp::And)?));
                } else if self.eat_keyword("or") {
                    pieces.push(SelectPiece::And(self.parse_and(name, BoolOp::Or)?));
                } else if self.starts_not_like() {
                    pieces.push(SelectPiece::Like(self.parse_not_like(name)?));
                } else if self.starts_like_infix() {
                    pieces.push(SelectPiece::Like(self.parse_like_op(name)?));
                } else if self.starts_not_ilike() {
                    pieces.push(SelectPiece::Ilike(self.parse_not_ilike(name)?));
                } else if self.starts_ilike_infix() {
                    pieces.push(SelectPiece::Ilike(self.parse_ilike_op(name)?));
                } else if self.starts_not_regex() {
                    pieces.push(SelectPiece::Like(self.parse_not_regex(name)?));
                } else if self.starts_regex_infix() {
                    pieces.push(SelectPiece::Like(self.parse_regex_op(name)?));
                } else if self.starts_not_between() {
                    pieces.push(SelectPiece::Between(self.parse_not_between(name)?));
                } else if self.starts_between_infix() {
                    pieces.push(SelectPiece::Between(self.parse_between_op(name)?));
                } else if self.starts_is_not_null() {
                    pieces.push(SelectPiece::Eq(self.parse_is_not_null(name)?));
                } else if self.starts_is_null() {
                    pieces.push(SelectPiece::Eq(self.parse_is_null(name)?));
                } else if self.starts_column_eq() {
                    pieces.push(SelectPiece::Eq(self.parse_column_eq(name)?));
                } else if self.starts_column_ne() {
                    pieces.push(SelectPiece::Eq(self.parse_column_ne(name)?));
                } else if self.starts_column_lt() {
                    pieces.push(SelectPiece::Eq(self.parse_column_lt(name)?));
                } else if self.starts_column_gt() {
                    pieces.push(SelectPiece::Eq(self.parse_column_gt(name)?));
                } else if self.starts_column_ge() {
                    pieces.push(SelectPiece::Eq(self.parse_column_ge(name)?));
                } else if self.starts_column_le() {
                    pieces.push(SelectPiece::Eq(self.parse_column_le(name)?));
                } else if self.starts_column_neq() {
                    pieces.push(SelectPiece::Eq(self.parse_column_neq(name)?));
                } else if self.starts_eq_or_ne() {
                    pieces.push(SelectPiece::Eq(self.parse_eq(name)?));
                } else if self.starts_bitand() {
                    pieces.push(SelectPiece::Add(self.parse_bitand(name)?));
                } else if self.starts_bitor() {
                    pieces.push(SelectPiece::Add(self.parse_bitor(name)?));
                } else if self.starts_bitxor() {
                    pieces.push(SelectPiece::Add(self.parse_bitxor(name)?));
                } else if self.starts_add() {
                    pieces.push(SelectPiece::Add(self.parse_add(name)?));
                } else if self.peek() == Some(b'(') {
                    return Err(Error::new(format!("unsupported: function `{name}`")));
                } else {
                    pieces.push(SelectPiece::Column(name));
                    if alias.is_some() {
                        self.pos = item_end;
                    }
                }
            }
            self.pos = item_end;
            piece_positions.push(item_position);
            piece_aliases.push(alias.map(str::to_string));
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
                    | SelectPiece::RegexpReplace(_)
                    | SelectPiece::StartsWith(_)
                    | SelectPiece::Glob(_)
                    | SelectPiece::Ilike(_)
                    | SelectPiece::Like(_)
                    | SelectPiece::Switch(_)
                    | SelectPiece::Bar(_)
                    | SelectPiece::TimestampSequence(_)
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
                    | SelectPiece::Nanos(_)
                    | SelectPiece::ToUtc(_)
                    | SelectPiece::ToTimezone(_)
                    | SelectPiece::ToPgDate(_)
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
                    | SelectPiece::Mid(_)
                    | SelectPiece::Spread(_)
                    | SelectPiece::SpreadBps(_)
                    | SelectPiece::Wmid(_)
                    | SelectPiece::Between(_)
                    | SelectPiece::InList(_)
                    | SelectPiece::Eq(_)
                    | SelectPiece::Add(_)
                    | SelectPiece::Arithmetic(_)
                    | SelectPiece::Neg(_)
                    | SelectPiece::BitNot(_)
                    | SelectPiece::And(_)
                    | SelectPiece::Ln(_)
                    | SelectPiece::Log(_)
                    | SelectPiece::Exp(_)
                    | SelectPiece::Pi(_)
                    | SelectPiece::CurrentSchema(_)
                    | SelectPiece::CurrentDatabase(_)
                    | SelectPiece::CurrentUser(_)
                    | SelectPiece::SessionUser(_)
                    | SelectPiece::CurrentCatalog(_)
                    | SelectPiece::Version(_)
                    | SelectPiece::PgIsInRecovery(_)
                    | SelectPiece::FormatType(_)
                    | SelectPiece::CurrentSetting(_)
                    | SelectPiece::PgGetPartKeyDef(_)
                    | SelectPiece::PgGetUserById(_)
                    | SelectPiece::PgTableIsVisible(_)
                    | SelectPiece::PgAge(_)
                    | SelectPiece::PgGetExpr(_)
                    | SelectPiece::Netmask(_)
                    | SelectPiece::GeoDistance(_)
                    | SelectPiece::GeoWithin(_)
                    | SelectPiece::WithinRadius(_)
                    | SelectPiece::WithinBox(_)
                    | SelectPiece::MakeGeohash(_)
                    | SelectPiece::ToUuid(_)
                    | SelectPiece::JsonExtract(_)
                    | SelectPiece::ToChar(_)
                    | SelectPiece::Greatest(_)
                    | SelectPiece::Least(_)
            )
        });
        if counts > 0
            && projected
            && pieces.iter().any(|piece| {
                !matches!(
                    piece,
                    SelectPiece::Column(_) | SelectPiece::Arithmetic(_) | SelectPiece::Aggregate(_)
                )
            })
        {
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
                    SelectPiece::RegexpReplace(expr) => items.push(Proj::RegexpReplace(expr)),
                    SelectPiece::StartsWith(expr) => items.push(Proj::StartsWith(expr)),
                    SelectPiece::Glob(expr) => items.push(Proj::Glob(expr)),
                    SelectPiece::Ilike(expr) => items.push(Proj::Ilike(expr)),
                    SelectPiece::Like(expr) => items.push(Proj::Like(expr)),
                    SelectPiece::Switch(expr) => items.push(Proj::Switch(expr)),
                    SelectPiece::Bar(expr) => items.push(Proj::Bar(expr)),
                    SelectPiece::TimestampSequence(expr) => {
                        items.push(Proj::TimestampSequence(expr))
                    }
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
                    SelectPiece::Nanos(expr) => items.push(Proj::Nanos(expr)),
                    SelectPiece::ToUtc(expr) => items.push(Proj::ToUtc(expr)),
                    SelectPiece::ToTimezone(expr) => items.push(Proj::ToTimezone(expr)),
                    SelectPiece::ToPgDate(expr) => items.push(Proj::ToPgDate(expr)),
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
                    SelectPiece::Mid(expr) => items.push(Proj::Mid(expr)),
                    SelectPiece::Spread(expr) => items.push(Proj::Spread(expr)),
                    SelectPiece::SpreadBps(expr) => items.push(Proj::SpreadBps(expr)),
                    SelectPiece::Wmid(expr) => items.push(Proj::Wmid(expr)),
                    SelectPiece::Between(expr) => items.push(Proj::Between(expr)),
                    SelectPiece::InList(expr) => items.push(Proj::InList(expr)),
                    SelectPiece::Eq(expr) => items.push(Proj::Eq(expr)),
                    SelectPiece::Add(expr) => items.push(Proj::Add(expr)),
                    SelectPiece::Arithmetic(expr) => items.push(Proj::Arithmetic(expr)),
                    SelectPiece::Neg(expr) => items.push(Proj::Neg(expr)),
                    SelectPiece::BitNot(expr) => items.push(Proj::BitNot(expr)),
                    SelectPiece::And(expr) => items.push(Proj::And(expr)),
                    SelectPiece::Ln(expr) => items.push(Proj::Ln(expr)),
                    SelectPiece::Log(expr) => items.push(Proj::Log(expr)),
                    SelectPiece::Exp(expr) => items.push(Proj::Exp(expr)),
                    SelectPiece::Pi(expr) => items.push(Proj::Pi(expr)),
                    SelectPiece::CurrentSchema(expr) => items.push(Proj::CurrentSchema(expr)),
                    SelectPiece::CurrentDatabase(expr) => items.push(Proj::CurrentDatabase(expr)),
                    SelectPiece::CurrentUser(expr) => items.push(Proj::CurrentUser(expr)),
                    SelectPiece::SessionUser(expr) => items.push(Proj::SessionUser(expr)),
                    SelectPiece::CurrentCatalog(expr) => items.push(Proj::CurrentCatalog(expr)),
                    SelectPiece::Version(expr) => items.push(Proj::Version(expr)),
                    SelectPiece::PgIsInRecovery(expr) => items.push(Proj::PgIsInRecovery(expr)),
                    SelectPiece::FormatType(expr) => items.push(Proj::FormatType(expr)),
                    SelectPiece::CurrentSetting(expr) => items.push(Proj::CurrentSetting(expr)),
                    SelectPiece::PgGetPartKeyDef(expr) => items.push(Proj::PgGetPartKeyDef(expr)),
                    SelectPiece::PgGetUserById(expr) => items.push(Proj::PgGetUserById(expr)),
                    SelectPiece::PgTableIsVisible(expr) => items.push(Proj::PgTableIsVisible(expr)),
                    SelectPiece::PgAge(expr) => items.push(Proj::PgAge(expr)),
                    SelectPiece::PgGetExpr(expr) => items.push(Proj::PgGetExpr(expr)),
                    SelectPiece::Netmask(expr) => items.push(Proj::Netmask(expr)),
                    SelectPiece::GeoDistance(expr) => items.push(Proj::GeoDistance(expr)),
                    SelectPiece::GeoWithin(expr) => items.push(Proj::GeoWithin(expr)),
                    SelectPiece::WithinRadius(expr) => items.push(Proj::WithinRadius(expr)),
                    SelectPiece::WithinBox(expr) => items.push(Proj::WithinBox(expr)),
                    SelectPiece::MakeGeohash(expr) => items.push(Proj::MakeGeohash(expr)),
                    SelectPiece::ToUuid(expr) => items.push(Proj::ToUuid(expr)),
                    SelectPiece::JsonExtract(expr) => items.push(Proj::JsonExtract(expr)),
                    SelectPiece::ToChar(expr) => items.push(Proj::ToChar(expr)),
                    SelectPiece::Greatest(expr) => items.push(Proj::Greatest(expr)),
                    SelectPiece::Least(expr) => items.push(Proj::Least(expr)),
                    SelectPiece::Aggregate(_) => {
                        return Err(Error::new("internal: aggregate piece"));
                    }
                }
            }
            for (item, alias) in items.iter_mut().zip(&piece_aliases) {
                if let Some(alias) = alias {
                    apply_projection_alias(item, alias);
                }
            }
            return Ok(SelectList::Projection(ProjectionList {
                items,
                positions: piece_positions,
            }));
        }
        if counts == 0 {
            let mut columns = Vec::with_capacity(pieces.len());
            for (index, piece) in pieces.into_iter().enumerate() {
                let SelectPiece::Column(name) = piece else {
                    return Err(Error::new("internal: column piece"));
                };
                columns.push(ColumnExpr {
                    name: name.to_ascii_lowercase(),
                    label: piece_aliases[index].clone().unwrap_or_else(|| name.clone()),
                    position: piece_positions[index],
                });
            }
            return Ok(SelectList::Columns(columns));
        }
        if counts == 0 {
            return Err(Error::new(
                "unsupported: aggregate mixed with other select expressions",
            ));
        }
        if pieces.len() == 1 {
            let Some(SelectPiece::Aggregate(aggregate)) = pieces.pop() else {
                return Err(Error::new("internal: aggregate piece"));
            };
            let mut aggregate = aggregate;
            if let Some(alias) = piece_aliases.first().and_then(Option::as_ref) {
                aggregate.label.clone_from(alias);
            }
            return Ok(SelectList::Aggregate(Box::new(aggregate)));
        }
        let mut keys = Vec::new();
        let mut select_order = Vec::new();
        let mut aggregates = Vec::new();
        for (select_index, piece) in pieces.into_iter().enumerate() {
            match piece {
                SelectPiece::Column(name) => {
                    let key_index = keys.len();
                    let operand = FilterOperand::Projection(Box::new(Proj::Column(
                        name.to_ascii_lowercase(),
                    )));
                    keys.push(GroupedSelectKey {
                        label: piece_aliases[select_index].clone().unwrap_or(name),
                        operand,
                        position: piece_positions[select_index],
                    });
                    select_order.push(GroupedSelectItem::Key(key_index));
                }
                SelectPiece::Arithmetic(expr) => {
                    let key_index = keys.len();
                    keys.push(GroupedSelectKey {
                        label: expr.label,
                        operand: expr.operand,
                        position: expr.position,
                    });
                    select_order.push(GroupedSelectItem::Key(key_index));
                }
                SelectPiece::Aggregate(mut expr) => {
                    if let Some(alias) = &piece_aliases[select_index] {
                        expr.label.clone_from(alias);
                    }
                    let aggregate_index = aggregates.len();
                    aggregates.push(expr);
                    select_order.push(GroupedSelectItem::Aggregate(aggregate_index));
                }
                SelectPiece::Concat(_)
                | SelectPiece::Length(_)
                | SelectPiece::Lower(_)
                | SelectPiece::Upper(_)
                | SelectPiece::Side(_)
                | SelectPiece::Substr(_)
                | SelectPiece::Trim(_)
                | SelectPiece::Replace(_)
                | SelectPiece::RegexpReplace(_)
                | SelectPiece::StartsWith(_)
                | SelectPiece::Glob(_)
                | SelectPiece::Ilike(_)
                | SelectPiece::Like(_)
                | SelectPiece::Switch(_)
                | SelectPiece::Bar(_)
                | SelectPiece::TimestampSequence(_)
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
                | SelectPiece::Nanos(_)
                | SelectPiece::ToUtc(_)
                | SelectPiece::ToTimezone(_)
                | SelectPiece::ToPgDate(_)
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
                | SelectPiece::Mid(_)
                | SelectPiece::Spread(_)
                | SelectPiece::SpreadBps(_)
                | SelectPiece::Wmid(_)
                | SelectPiece::Between(_)
                | SelectPiece::InList(_)
                | SelectPiece::Eq(_)
                | SelectPiece::Add(_)
                | SelectPiece::Neg(_)
                | SelectPiece::BitNot(_)
                | SelectPiece::And(_)
                | SelectPiece::Ln(_)
                | SelectPiece::Log(_)
                | SelectPiece::Exp(_)
                | SelectPiece::Pi(_)
                | SelectPiece::CurrentSchema(_)
                | SelectPiece::CurrentDatabase(_)
                | SelectPiece::CurrentUser(_)
                | SelectPiece::SessionUser(_)
                | SelectPiece::CurrentCatalog(_)
                | SelectPiece::Version(_)
                | SelectPiece::PgIsInRecovery(_)
                | SelectPiece::FormatType(_)
                | SelectPiece::CurrentSetting(_)
                | SelectPiece::PgGetPartKeyDef(_)
                | SelectPiece::PgGetUserById(_)
                | SelectPiece::PgTableIsVisible(_)
                | SelectPiece::PgAge(_)
                | SelectPiece::PgGetExpr(_)
                | SelectPiece::Netmask(_)
                | SelectPiece::GeoDistance(_)
                | SelectPiece::GeoWithin(_)
                | SelectPiece::WithinRadius(_)
                | SelectPiece::WithinBox(_)
                | SelectPiece::MakeGeohash(_)
                | SelectPiece::ToUuid(_)
                | SelectPiece::JsonExtract(_)
                | SelectPiece::ToChar(_)
                | SelectPiece::Greatest(_)
                | SelectPiece::Least(_) => {
                    return Err(Error::new("internal: projection piece"));
                }
            }
        }
        if aggregates.is_empty() {
            return Err(Error::new(
                "unsupported: aggregate mixed with other select expressions",
            ));
        }
        Ok(SelectList::Grouped(GroupedSelect {
            keys,
            aggregates,
            select_order,
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

    fn starts_current_schema_call(&self) -> bool {
        self.starts_named_call("current_schema")
            || self.starts_qualified_call("pg_catalog", "current_schema")
    }

    fn parse_current_schema(&mut self) -> Result<CurrentSchemaExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(CurrentSchemaExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_current_database_call(&self) -> bool {
        self.starts_named_call("current_database")
            || self.starts_qualified_call("pg_catalog", "current_database")
    }

    fn parse_current_database(&mut self) -> Result<CurrentDatabaseExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(CurrentDatabaseExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_current_user_call(&self) -> bool {
        self.starts_named_call("current_user")
    }

    fn parse_current_user(&mut self) -> Result<CurrentUserExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(CurrentUserExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_session_user_call(&self) -> bool {
        self.starts_named_call("session_user")
    }

    fn parse_session_user(&mut self) -> Result<SessionUserExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(SessionUserExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_current_catalog_call(&self) -> bool {
        self.starts_named_call("current_catalog")
    }

    fn parse_current_catalog(&mut self) -> Result<CurrentCatalogExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(CurrentCatalogExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_version_call(&self) -> bool {
        self.starts_named_call("version") || self.starts_qualified_call("pg_catalog", "version")
    }

    fn parse_version(&mut self) -> Result<VersionExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(VersionExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_qualified_call(&self, schema: &str, name: &str) -> bool {
        let bytes = &self.src[self.pos..];
        let Some(schema_len) = qualified_ident_len(bytes) else {
            return false;
        };
        let schema_text = std::str::from_utf8(&bytes[..schema_len]).unwrap_or("");
        if !schema_text.eq_ignore_ascii_case(schema) {
            return false;
        }
        let mut index = schema_len;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        if bytes.get(index) != Some(&b'.') {
            return false;
        }
        index += 1;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        let Some(name_len) = qualified_ident_len(&bytes[index..]) else {
            return false;
        };
        let name_text = std::str::from_utf8(&bytes[index..index + name_len]).unwrap_or("");
        if !name_text.eq_ignore_ascii_case(name) {
            return false;
        }
        index += name_len;
        while bytes
            .get(index)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            index += 1;
        }
        bytes.get(index) == Some(&b'(')
    }

    fn starts_pg_is_in_recovery_call(&self) -> bool {
        self.starts_named_call("pg_is_in_recovery")
            || self.starts_qualified_call("pg_catalog", "pg_is_in_recovery")
    }

    fn parse_pg_is_in_recovery(&mut self) -> Result<PgIsInRecoveryExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(PgIsInRecoveryExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_current_setting_call(&self) -> bool {
        self.starts_named_call("current_setting")
    }

    fn parse_current_setting(&mut self) -> Result<CurrentSettingExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `current_setting` requires arguments: current_setting(STRING constant)"
                    .to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(CurrentSettingExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_pg_get_userbyid_call(&self) -> bool {
        self.starts_qualified_call("pg_catalog", "pg_get_userbyid")
            || self.starts_named_call("pg_get_userbyid")
    }

    fn parse_pg_get_userbyid(&mut self) -> Result<PgGetUserByIdExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let (qualified, name) = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            (true, format!("{first}.{second}"))
        } else {
            (false, first)
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        if qualified && args.is_empty() {
            return Err(Error::new(
                "function `pg_catalog.pg_get_userbyid` requires arguments: pg_catalog.pg_get_userbyid(INT)"
                    .to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(PgGetUserByIdExpr {
            source_label: label.clone().into(),
            label,
            args,
            qualified,
        })
    }

    fn starts_pg_table_is_visible_call(&self) -> bool {
        self.starts_qualified_call("pg_catalog", "pg_table_is_visible")
            || self.starts_named_call("pg_table_is_visible")
    }

    fn parse_pg_table_is_visible(&mut self) -> Result<PgTableIsVisibleExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let (qualified, name) = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            (true, format!("{first}.{second}"))
        } else {
            (false, first)
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        if qualified && args.is_empty() {
            return Err(Error::new(
                "function `pg_catalog.pg_table_is_visible` requires arguments: pg_catalog.pg_table_is_visible(INT)"
                    .to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(PgTableIsVisibleExpr {
            source_label: label.clone().into(),
            label,
            args,
            qualified,
        })
    }

    fn starts_pg_age_call(&self) -> bool {
        self.starts_qualified_call("pg_catalog", "age") || self.starts_named_call("age")
    }

    fn parse_pg_age(&mut self) -> Result<PgAgeExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let (qualified, name) = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            (true, format!("{first}.{second}"))
        } else {
            (false, first)
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        if qualified && args.is_empty() {
            return Err(Error::new(
                "function `pg_catalog.age` requires arguments: pg_catalog.age(LONG)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(PgAgeExpr {
            source_label: label.clone().into(),
            label,
            args,
            qualified,
        })
    }

    fn starts_pg_get_partkeydef_call(&self) -> bool {
        self.starts_named_call("pg_get_partkeydef")
            || self.starts_qualified_call("pg_catalog", "pg_get_partkeydef")
    }

    fn parse_pg_get_partkeydef(&mut self) -> Result<PgGetPartKeyDefExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            let reported = if name.to_ascii_lowercase().contains("pg_catalog.") {
                "pg_catalog.pg_get_partkeydef"
            } else {
                "pg_get_partkeydef"
            };
            return Err(Error::new(format!(
                "function `{reported}` requires arguments: {reported}(INT)"
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(PgGetPartKeyDefExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn parse_generate_series_call(&mut self) -> Result<(String, Vec<ConcatArg>), Error> {
        let name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(
                "function `generate_series` requires arguments".to_string(),
            ));
        }
        self.skip();
        if self.eat(b')') {
            return Err(Error::new(
                "function `generate_series` requires arguments".to_string(),
            ));
        }
        let mut labels = Vec::new();
        let mut args = Vec::new();
        loop {
            let (label, arg) = self.parse_call_arg(true)?;
            labels.push(label);
            args.push(arg);
            self.skip();
            if !self.eat(b',') {
                break;
            }
        }
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new(format!(
                "unsupported: expected ')' after generate_series near `{}`",
                self.rest()
            )));
        }
        Ok((format!("{name}({})", labels.join(", ")), args))
    }

    fn parse_long_sequence_call(&mut self) -> Result<(String, Vec<ConcatArg>), Error> {
        let name = self.ident_spelling()?;
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new("invalid arguments"));
        }
        self.skip();
        if self.eat(b')') {
            return Err(Error::new("invalid arguments"));
        }
        let mut labels = Vec::new();
        let mut args = Vec::new();
        loop {
            let (label, arg) = self.parse_call_arg(true)?;
            labels.push(label);
            args.push(arg);
            self.skip();
            if !self.eat(b',') {
                break;
            }
        }
        self.skip();
        if !self.eat(b')') {
            return Err(Error::new(format!(
                "unsupported: expected ')' after long_sequence near `{}`",
                self.rest()
            )));
        }
        if args.len() != 1 {
            return Err(Error::new("invalid arguments"));
        }
        Ok((format!("{name}({})", labels.join(", ")), args))
    }

    fn starts_pg_get_expr_call(&self) -> bool {
        self.starts_named_call("pg_get_expr")
            || self.starts_qualified_call("pg_catalog", "pg_get_expr")
    }

    fn parse_pg_get_expr(&mut self) -> Result<PgGetExprExpr, Error> {
        let first = self.ident_spelling()?;
        self.skip();
        let name = if self.eat(b'.') {
            self.skip();
            let second = self.ident_spelling()?;
            format!("{first}.{second}")
        } else {
            first
        };
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            let reported = if name.to_ascii_lowercase().contains("pg_catalog.") {
                "pg_catalog.pg_get_expr"
            } else {
                "pg_get_expr"
            };
            return Err(Error::new(format!(
                "function `{reported}` requires arguments"
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(PgGetExprExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_netmask_call(&self) -> bool {
        self.starts_named_call("netmask")
    }

    fn parse_netmask(&mut self) -> Result<NetmaskExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `netmask` requires arguments".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(NetmaskExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_geo_distance_call(&self) -> bool {
        self.starts_named_call("geo_distance_meters")
    }

    fn parse_geo_distance(&mut self) -> Result<GeoDistanceExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `geo_distance_meters` requires arguments: geo_distance_meters(DOUBLE, DOUBLE, DOUBLE, DOUBLE)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(GeoDistanceExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_geo_within_call(&self) -> bool {
        self.starts_named_call("geo_within_radius_latlon")
    }

    fn parse_geo_within(&mut self) -> Result<GeoWithinExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `geo_within_radius_latlon` requires arguments: geo_within_radius_latlon(DOUBLE, DOUBLE, DOUBLE, DOUBLE, DOUBLE)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(GeoWithinExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_within_radius_call(&self) -> bool {
        self.starts_named_call("within_radius")
    }

    fn parse_within_radius(&mut self) -> Result<WithinRadiusExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `within_radius` requires arguments: within_radius(DOUBLE, DOUBLE, DOUBLE, DOUBLE, DOUBLE)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(WithinRadiusExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_within_box_call(&self) -> bool {
        self.starts_named_call("within_box")
    }

    fn parse_within_box(&mut self) -> Result<WithinBoxExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `within_box` requires arguments: within_box(DOUBLE, DOUBLE, DOUBLE, DOUBLE, DOUBLE, DOUBLE)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(WithinBoxExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_make_geohash_call(&self) -> bool {
        self.starts_named_call("make_geohash")
    }

    fn parse_make_geohash(&mut self) -> Result<MakeGeohashExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `make_geohash` requires arguments: make_geohash(DOUBLE, DOUBLE, INT constant)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(MakeGeohashExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_to_uuid_call(&self) -> bool {
        self.starts_named_call("to_uuid")
    }

    fn parse_to_uuid(&mut self) -> Result<ToUuidExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `to_uuid` requires arguments: to_uuid(LONG, LONG)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToUuidExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_json_extract_call(&self) -> bool {
        self.starts_named_call("json_extract")
    }

    fn parse_json_extract(&mut self) -> Result<JsonExtractExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `json_extract` requires arguments".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(JsonExtractExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
    }

    fn starts_to_char_call(&self) -> bool {
        self.starts_named_call("to_char")
    }

    fn parse_to_char(&mut self) -> Result<ToCharExpr, Error> {
        let name = self.ident_spelling()?;
        if self.argument_is_from_base64() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_from_base64()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after to_char".to_string(),
                ));
            }
            let label = format!("{name}({})", inner.label);
            return Ok(ToCharExpr {
                source_label: label.clone().into(),
                label,
                args: Vec::new(),
                base64: Some(inner),
            });
        }
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `to_char` requires arguments: to_char(BINARY)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToCharExpr {
            source_label: label.clone().into(),
            label,
            args,
            base64: None,
        })
    }

    fn starts_from_base64_call(&self) -> bool {
        self.starts_named_call("from_base64")
    }

    fn parse_from_base64(&mut self) -> Result<FromBase64Expr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `from_base64` requires arguments: from_base64(STRING)".to_string(),
            ));
        }
        Ok(FromBase64Expr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn argument_is_from_base64(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_from_base64_call();
        self.pos = saved;
        nested
    }

    fn starts_format_type_call(&self) -> bool {
        self.starts_named_call("format_type")
    }

    fn parse_format_type(&mut self) -> Result<FormatTypeExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `format_type` requires arguments: format_type(INT, INT)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(FormatTypeExpr {
            source_label: label.clone().into(),
            label,
            args,
        })
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

    fn starts_mid_call(&self) -> bool {
        self.starts_named_call("mid")
    }

    fn starts_spread_call(&self) -> bool {
        self.starts_named_call("spread")
    }

    fn starts_wmid_call(&self) -> bool {
        self.starts_named_call("wmid")
    }

    fn starts_between_call(&self) -> bool {
        self.starts_named_call("between")
    }

    fn starts_not_between(&self) -> bool {
        let Some(after_not) = self.keyword_at(self.pos, "not") else {
            return false;
        };
        self.keyword_at(after_not, "between").is_some()
    }

    fn parse_not_between(&mut self, name: String) -> Result<BetweenExpr, Error> {
        if !self.eat_keyword("not") {
            return Err(Error::new(
                "unsupported: expected NOT BETWEEN after column".to_string(),
            ));
        }
        let mut expr = self.parse_between_op(name)?;
        expr.label = format!("not({})", expr.label);
        expr.source_label = expr.source_label.replacen(" between ", " not between ", 1);
        expr.negated = true;
        Ok(expr)
    }

    fn starts_between_infix(&self) -> bool {
        self.keyword_at(self.pos, "between").is_some()
    }

    fn parse_between_op(&mut self, name: String) -> Result<BetweenExpr, Error> {
        if !self.eat_keyword("between") {
            return Err(Error::new(
                "unsupported: expected BETWEEN after column".to_string(),
            ));
        }
        let (low_label, low) = self.parse_call_arg(true)?;
        if !self.eat_keyword("and") {
            return Err(Error::new(
                "unsupported: expected AND after BETWEEN".to_string(),
            ));
        }
        let (high_label, high) = self.parse_call_arg(true)?;
        let source_label = format!("{name} between {low_label} and {high_label}");
        Ok(BetweenExpr {
            label: format!("{name} between ({low_label}, {high_label})"),
            source_label,
            args: vec![ConcatArg::Column(name), low, high],
            negated: false,
        })
    }

    fn parse_between(&mut self) -> Result<BetweenExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        let label = if labels.len() == 3 {
            format!("{} between ({}, {})", labels[0], labels[1], labels[2])
        } else {
            format!("{name}({})", labels.join(", "))
        };
        let source_label = format!("{name}({})", labels.join(", "));
        Ok(BetweenExpr {
            label,
            source_label,
            args,
            negated: false,
        })
    }

    fn parse_in_list(&mut self, name: String) -> Result<InListExpr, Error> {
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new("unsupported: expected '(' after in".to_string()));
        }
        self.skip();
        if self.eat(b')') {
            return Err(Error::new("too few arguments for 'in'".to_string()));
        }
        let mut args = Vec::new();
        let mut labels = Vec::new();
        loop {
            let (label, arg) = self.parse_call_arg(true)?;
            labels.push(label);
            args.push(arg);
            self.skip();
            if self.eat(b',') {
                continue;
            }
            if self.eat(b')') {
                break;
            }
            return Err(Error::new("unsupported: expected ')' after in".to_string()));
        }
        let source_label = format!("{name} in ({})", labels.join(", "));
        Ok(InListExpr {
            label: source_label.clone(),
            source_label,
            value: name,
            args,
            negated: false,
        })
    }

    fn parse_not_in_list(&mut self, name: String) -> Result<InListExpr, Error> {
        self.expect_keyword("not")?;
        self.expect_keyword("in")?;
        let mut expression = self.parse_in_list(name)?;
        expression.negated = true;
        expression.label = format!("not({})", expression.label);
        expression.source_label = expression.source_label.replacen(" in ", " not in ", 1);
        Ok(expression)
    }

    fn starts_not_in(&self) -> bool {
        self.keyword_at(self.pos, "not")
            .and_then(|after_not| self.keyword_at(after_not, "in"))
            .is_some()
    }

    fn parse_and(&mut self, left: String, op: BoolOp) -> Result<AndExpr, Error> {
        self.skip();
        let ident = matches!(
            self.peek(),
            Some(byte) if byte.is_ascii_alphabetic() || byte == b'_'
        );
        if !ident {
            return Err(Error::new(format!(
                "unsupported: expected boolean after {}",
                op.spelling()
            )));
        }
        let mut spelling = self.ident_spelling()?;
        let right = match spelling.to_ascii_lowercase().as_str() {
            "null" => AndRight::Literal(Scalar::Null),
            "true" => AndRight::Literal(Scalar::Boolean(true)),
            "false" => AndRight::Literal(Scalar::Boolean(false)),
            _ => {
                self.skip();
                if self.eat(b'.') {
                    spelling.push('.');
                    spelling.push_str(&self.ident_spelling()?);
                }
                AndRight::Column(spelling)
            }
        };
        let right_label = match &right {
            AndRight::Literal(Scalar::Null) => "null".to_string(),
            AndRight::Literal(Scalar::Boolean(true)) => "true".to_string(),
            AndRight::Literal(Scalar::Boolean(false)) => "false".to_string(),
            AndRight::Column(name) => name.clone(),
            AndRight::Literal(_) => "null".to_string(),
        };
        Ok(AndExpr {
            label: format!("{left} {} {right_label}", op.spelling()),
            source_label: format!("{left} {} {right_label}", op.spelling()),
            left,
            right,
            op,
        })
    }

    fn starts_tilde(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'~') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_')
    }

    fn parse_tilde(&mut self) -> Result<BitNotExpr, Error> {
        self.skip();
        if !self.eat(b'~') {
            return Err(Error::new(
                "unsupported: expected '~' before column".to_string(),
            ));
        }
        let name = self.qualified_ident()?;
        Ok(BitNotExpr {
            label: format!("~({name})"),
            value: name,
        })
    }

    fn starts_unary_minus(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'-') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_')
    }

    fn parse_unary_minus(&mut self) -> Result<NegExpr, Error> {
        self.skip();
        if !self.eat(b'-') {
            return Err(Error::new(
                "unsupported: expected '-' before column".to_string(),
            ));
        }
        let name = self.qualified_ident()?;
        Ok(NegExpr {
            label: format!("-({name})"),
            value: name,
        })
    }

    fn starts_bitxor(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'^') || self.src.get(pos + 1) == Some(&b'^') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_bitxor(&mut self, name: String) -> Result<AddExpr, Error> {
        self.skip();
        if !self.eat(b'^') {
            return Err(Error::new(
                "unsupported: expected '^' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '^'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(AddExpr {
            label: format!("{name} ^ {column}"),
            value: name,
            literal: Scalar::Null,
            other: Some(column),
            op: ArithOp::BitXor,
        })
    }

    fn starts_bitor(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'|') || self.src.get(pos + 1) == Some(&b'|') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_bitor(&mut self, name: String) -> Result<AddExpr, Error> {
        self.skip();
        if !self.eat(b'|') {
            return Err(Error::new(
                "unsupported: expected '|' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '|'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(AddExpr {
            label: format!("{name} | {column}"),
            value: name,
            literal: Scalar::Null,
            other: Some(column),
            op: ArithOp::BitOr,
        })
    }

    fn starts_bitand(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'&') || self.src.get(pos + 1) == Some(&b'&') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_bitand(&mut self, name: String) -> Result<AddExpr, Error> {
        self.skip();
        if !self.eat(b'&') {
            return Err(Error::new(
                "unsupported: expected '&' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '&'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(AddExpr {
            label: format!("{name} & {column}"),
            value: name,
            literal: Scalar::Null,
            other: Some(column),
            op: ArithOp::BitAnd,
        })
    }

    fn starts_add(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        matches!(self.src.get(pos), Some(b'+' | b'-' | b'*' | b'/' | b'%'))
    }

    fn parse_add(&mut self, name: String) -> Result<AddExpr, Error> {
        self.skip();
        let op = if self.eat(b'-') {
            ArithOp::Sub
        } else if self.eat(b'+') {
            ArithOp::Add
        } else if self.eat(b'*') {
            ArithOp::Mul
        } else if self.eat(b'/') {
            ArithOp::Div
        } else if self.eat(b'%') {
            ArithOp::Mod
        } else {
            return Err(Error::new(
                "unsupported: expected '+', '-', '*', '/' or '%' after column".to_string(),
            ));
        };
        let (literal, other) = if matches!(
            op,
            ArithOp::Add | ArithOp::Sub | ArithOp::Mul | ArithOp::Div | ArithOp::Mod
        ) && self.starts_add_column()
        {
            (Scalar::Null, Some(self.ident_spelling()?))
        } else {
            (self.parse_literal()?, None)
        };
        let label = match &other {
            Some(column) => format!("{name} {} {column}", op.spelling()),
            None => format!("{name} {} {}", op.spelling(), eq_literal_label(&literal)),
        };
        Ok(AddExpr {
            label,
            value: name,
            literal,
            other,
            op,
        })
    }

    fn starts_add_column(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn keyword_at(&self, pos: usize, word: &str) -> Option<usize> {
        let mut pos = pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let bytes = word.as_bytes();
        let end = pos + bytes.len();
        if self.src.len() < end || !self.src[pos..end].eq_ignore_ascii_case(bytes) {
            return None;
        }
        if matches!(self.src.get(end), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            return None;
        }
        Some(end)
    }

    fn starts_is_not_null(&self) -> bool {
        let Some(after_is) = self.keyword_at(self.pos, "is") else {
            return false;
        };
        let Some(after_not) = self.keyword_at(after_is, "not") else {
            return false;
        };
        self.keyword_at(after_not, "null").is_some()
    }

    fn parse_is_not_null(&mut self, name: String) -> Result<EqExpr, Error> {
        if !self.eat_keyword("is") || !self.eat_keyword("not") || !self.eat_keyword("null") {
            return Err(Error::new(
                "unsupported: expected IS NOT NULL after column".to_string(),
            ));
        }
        Ok(EqExpr {
            label: format!("{name} != NULL"),
            source_label: String::new(),
            value: name,
            op: CmpOp::NotEq,
            literal: Scalar::Null,
            other: None,
        })
    }

    fn starts_is_null(&self) -> bool {
        let Some(after_is) = self.keyword_at(self.pos, "is") else {
            return false;
        };
        self.keyword_at(after_is, "null").is_some()
    }

    fn parse_is_null(&mut self, name: String) -> Result<EqExpr, Error> {
        if !self.eat_keyword("is") || !self.eat_keyword("null") {
            return Err(Error::new(
                "unsupported: expected IS NULL after column".to_string(),
            ));
        }
        Ok(EqExpr {
            label: format!("{name} = NULL"),
            source_label: String::new(),
            value: name,
            op: CmpOp::Eq,
            literal: Scalar::Null,
            other: None,
        })
    }

    fn starts_column_eq(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'=') || self.src.get(pos + 1) == Some(&b'=') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_eq(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !self.eat(b'=') {
            return Err(Error::new(
                "unsupported: expected '=' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '='".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} = {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::Eq,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_ne(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'<') || self.src.get(pos + 1) != Some(&b'>') {
            return false;
        }
        pos += 2;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_ne(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !(self.eat(b'<') && self.eat(b'>')) {
            return Err(Error::new(
                "unsupported: expected '<>' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '<>'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} <> {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::NotEq,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_lt(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'<') || matches!(self.src.get(pos + 1), Some(b'>' | b'=')) {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_lt(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !self.eat(b'<') {
            return Err(Error::new(
                "unsupported: expected '<' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '<'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} < {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::Lt,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_gt(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'>') || self.src.get(pos + 1) == Some(&b'=') {
            return false;
        }
        pos += 1;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_gt(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !self.eat(b'>') {
            return Err(Error::new(
                "unsupported: expected '>' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '>'".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} > {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::Gt,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_ge(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'>') || self.src.get(pos + 1) != Some(&b'=') {
            return false;
        }
        pos += 2;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_ge(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !(self.eat(b'>') && self.eat(b'=')) {
            return Err(Error::new(
                "unsupported: expected '>=' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '>='".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} >= {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::GtEq,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_le(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'<') || self.src.get(pos + 1) != Some(&b'=') {
            return false;
        }
        pos += 2;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_le(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !(self.eat(b'<') && self.eat(b'=')) {
            return Err(Error::new(
                "unsupported: expected '<=' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '<='".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} <= {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::LtEq,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_column_neq(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if self.src.get(pos) != Some(&b'!') || self.src.get(pos + 1) != Some(&b'=') {
            return false;
        }
        pos += 2;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        let start = pos;
        pos += 1;
        while matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_'
        ) {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        !matches!(word.as_str(), "null" | "true" | "false")
    }

    fn parse_column_neq(&mut self, name: String) -> Result<EqExpr, Error> {
        self.skip();
        if !(self.eat(b'!') && self.eat(b'=')) {
            return Err(Error::new(
                "unsupported: expected '!=' after column".to_string(),
            ));
        }
        if !self.starts_add_column() {
            return Err(Error::new(
                "unsupported: expected a column after '!='".to_string(),
            ));
        }
        let column = self.ident_spelling()?;
        Ok(EqExpr {
            label: format!("{name} != {column}"),
            source_label: String::new(),
            value: name,
            op: CmpOp::NotEq,
            literal: Scalar::Null,
            other: Some(column),
        })
    }

    fn starts_eq_or_ne(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let rest = &self.src[pos..];
        rest.starts_with(b"=")
            || rest.starts_with(b"!=")
            || rest.starts_with(b"<>")
            || (rest.starts_with(b"<") && !rest.starts_with(b"<>"))
            || rest.starts_with(b">")
    }

    fn parse_eq(&mut self, name: String) -> Result<EqExpr, Error> {
        let spelling = self.cmp_spelling();
        let op = self.parse_op()?;
        let literal = self.parse_literal()?;
        Ok(EqExpr {
            label: format!("{name} {spelling} {}", eq_literal_label(&literal)),
            source_label: String::new(),
            value: name,
            op,
            literal,
            other: None,
        })
    }

    fn cmp_spelling(&self) -> String {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let rest = &self.src[pos..];
        if rest.starts_with(b"!=") {
            "!=".to_string()
        } else if rest.starts_with(b"<>") {
            "<>".to_string()
        } else if rest.starts_with(b"<=") {
            "<=".to_string()
        } else if rest.starts_with(b"<") {
            "<".to_string()
        } else if rest.starts_with(b">=") {
            ">=".to_string()
        } else if rest.starts_with(b">") {
            ">".to_string()
        } else {
            "=".to_string()
        }
    }

    fn starts_spread_bps_call(&self) -> bool {
        self.starts_named_call("spread_bps")
    }

    fn parse_wmid(&mut self) -> Result<WmidExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `wmid` requires arguments: wmid(DOUBLE, DOUBLE, DOUBLE, DOUBLE)",
            ));
        }
        Ok(WmidExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_spread_bps(&mut self) -> Result<SpreadBpsExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `spread_bps` requires arguments: spread_bps(DOUBLE, DOUBLE)",
            ));
        }
        Ok(SpreadBpsExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_spread(&mut self) -> Result<SpreadExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `spread` requires arguments: spread(DOUBLE, DOUBLE)",
            ));
        }
        Ok(SpreadExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_mid(&mut self) -> Result<MidExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `mid` requires arguments: mid(DOUBLE, DOUBLE)",
            ));
        }
        Ok(MidExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
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

    fn starts_not_prefix(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        let start = pos;
        let Some(first) = self.src.get(pos) else {
            return false;
        };
        if !first.is_ascii_alphabetic() && *first != b'_' {
            return false;
        }
        pos += 1;
        while matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            pos += 1;
        }
        let word = std::str::from_utf8(&self.src[start..pos])
            .unwrap_or("")
            .to_ascii_lowercase();
        if word != "not" {
            return false;
        }
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_') {
            return true;
        }
        if self.src.get(pos) != Some(&b'(') {
            return false;
        }
        let mut depth = 0usize;
        let mut quoted = false;
        let mut cursor = pos;
        while cursor < self.src.len() {
            match self.src[cursor] {
                b'\'' => {
                    if quoted && self.src.get(cursor + 1) == Some(&b'\'') {
                        cursor += 2;
                        continue;
                    }
                    quoted = !quoted;
                }
                b'(' if !quoted => depth += 1,
                b')' if !quoted => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let inner = std::str::from_utf8(&self.src[pos + 1..cursor])
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        return inner.bytes().any(|byte| b"=<>!~&|".contains(&byte))
                            || [
                                " and ",
                                " or ",
                                " is ",
                                " in ",
                                " like ",
                                " ilike ",
                                " between ",
                            ]
                            .iter()
                            .any(|operator| inner.contains(operator));
                    }
                }
                b',' if !quoted && depth == 1 => return false,
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    fn parse_not_prefix(&mut self) -> Result<NotExpr, Error> {
        self.skip();
        let start = self.pos;
        if !self.eat_keyword("not") {
            return Err(Error::new("unsupported: expected NOT prefix"));
        }
        let inner = self.parse_filter_atom()?;
        let mut filter = Filter::Not(Box::new(inner));
        filter = self.parse_filter_and_tail(filter)?;
        filter = self.parse_filter_or_tail(filter)?;
        let source_label = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| Error::new("unsupported expression after NOT"))?
            .trim()
            .to_string();
        let label = filter_expression_label(&filter);
        Ok(NotExpr {
            source_label: source_label.into(),
            label,
            position: start,
            args: Vec::new(),
            filter: Some(Box::new(filter)),
        })
    }

    fn parse_not(&mut self) -> Result<NotExpr, Error> {
        let position = self.pos;
        let _name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args("not", true)?;
        if args.is_empty() {
            return Err(Error::new(
                "too few arguments for 'not' [found=0,expected=1]",
            ));
        }
        let last = labels.last().map(String::as_str).unwrap_or("");
        let label = format!("not({last})");
        Ok(NotExpr {
            source_label: label.clone().into(),
            label,
            position,
            args,
            filter: None,
        })
    }

    fn starts_typeof_call(&self) -> bool {
        self.starts_named_call("typeof")
    }

    fn parse_typeof(&mut self) -> Result<TypeofExpr, Error> {
        let name = self.ident_spelling()?;
        if self.argument_is_make_geohash() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_make_geohash()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: Vec::new(),
                arithmetic: None,
                geohash: Some(inner),
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_to_uuid() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_to_uuid()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: Vec::new(),
                arithmetic: None,
                geohash: None,
                uuid: Some(Box::new(inner)),
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_json_extract() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_json_extract()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: Vec::new(),
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: Some(Box::new(inner)),
                to_char: None,
            });
        }
        if self.argument_is_pg_is_in_recovery() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_is_in_recovery()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            if !inner.args.is_empty() {
                let reported = if inner.label.to_ascii_lowercase().contains("pg_catalog.") {
                    "pg_catalog.pg_is_in_recovery"
                } else {
                    "pg_is_in_recovery"
                };
                return Err(Error::new(format!(
                    "wrong number of arguments for function `{reported}`; expected: 0, provided: {}",
                    inner.args.len()
                )));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: vec![ConcatArg::Literal(Scalar::Boolean(false))],
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_current_database() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_current_database()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            if !inner.args.is_empty() {
                let reported = if inner.label.to_ascii_lowercase().contains("pg_catalog.") {
                    "pg_catalog.current_database"
                } else {
                    "current_database"
                };
                return Err(Error::new(format!(
                    "wrong number of arguments for function `{reported}`; expected: 0, provided: {}",
                    inner.args.len()
                )));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: vec![ConcatArg::Literal(Scalar::Text("database".to_string()))],
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_current_schema() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_current_schema()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            if !inner.args.is_empty() {
                let reported = if inner.label.to_ascii_lowercase().contains("pg_catalog.") {
                    "pg_catalog.current_schema"
                } else {
                    "current_schema"
                };
                return Err(Error::new(format!(
                    "wrong number of arguments for function `{reported}`; expected: 0, provided: {}",
                    inner.args.len()
                )));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: vec![ConcatArg::Literal(Scalar::Text("schema".to_string()))],
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_version() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_version()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            if !inner.args.is_empty() {
                let reported = if inner.label.to_ascii_lowercase().contains("pg_catalog.") {
                    "pg_catalog.version"
                } else {
                    "version"
                };
                return Err(Error::new(format!(
                    "wrong number of arguments for function `{reported}`; expected: 0, provided: {}",
                    inner.args.len()
                )));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: vec![ConcatArg::Literal(Scalar::Text("version".to_string()))],
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_pg_age() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_age()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: inner.args,
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_pg_table_is_visible() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_table_is_visible()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: inner.args,
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_pg_get_expr() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_get_expr()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: inner.args,
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_pg_get_partkeydef() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_get_partkeydef()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: inner.args,
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_pg_get_userbyid() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_pg_get_userbyid()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: inner.args,
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: None,
            });
        }
        if self.argument_is_to_char() {
            self.skip();
            self.eat(b'(');
            self.skip();
            let inner = self.parse_to_char()?;
            self.skip();
            if !self.eat(b')') {
                return Err(Error::new(
                    "unsupported: expected ')' after typeof".to_string(),
                ));
            }
            return Ok(TypeofExpr {
                label: format!("{name}({})", inner.label),
                source_label: Box::default(),
                args: Vec::new(),
                arithmetic: None,
                geohash: None,
                uuid: None,
                json: None,
                to_char: Some(Box::new(inner)),
            });
        }
        if let Some(expr) = self.parse_typeof_arithmetic_argument(&name)? {
            return Ok(expr);
        }
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("exactly one argument expected"));
        }
        Ok(TypeofExpr {
            label: format!("{name}({})", labels.join(", ")),
            source_label: Box::default(),
            args,
            arithmetic: None,
            geohash: None,
            uuid: None,
            json: None,
            to_char: None,
        })
    }

    fn parse_typeof_arithmetic_argument(
        &mut self,
        name: &str,
    ) -> Result<Option<TypeofExpr>, Error> {
        let saved = self.pos;
        self.skip();
        if !self.eat(b'(') {
            self.pos = saved;
            return Ok(None);
        }
        self.skip();
        let start = self.pos;
        let mut depth = 0usize;
        let mut quoted = false;
        let mut pos = start;
        let end = loop {
            let Some(&byte) = self.src.get(pos) else {
                self.pos = saved;
                return Ok(None);
            };
            match byte {
                b'\'' => {
                    if quoted && self.src.get(pos + 1) == Some(&b'\'') {
                        pos += 2;
                        continue;
                    }
                    quoted = !quoted;
                }
                b'(' if !quoted => depth += 1,
                b')' if !quoted && depth == 0 => break pos,
                b')' if !quoted => depth -= 1,
                _ => {}
            }
            pos += 1;
        };
        let source = std::str::from_utf8(&self.src[start..end])
            .map_err(|_| Error::new("invalid UTF-8 in typeof expression"))?
            .trim();
        if top_level_arithmetic(source.as_bytes()).is_none() {
            self.pos = saved;
            return Ok(None);
        }
        let _ = parse_filter_operand(source, 0)?;
        self.pos = end + 1;
        Ok(Some(TypeofExpr {
            label: format!("{name}({})", normalize_select_arithmetic_label(source)),
            source_label: String::new().into(),
            args: Vec::new(),
            arithmetic: Some(source.to_string().into_boxed_str()),
            geohash: None,
            uuid: None,
            json: None,
            to_char: None,
        }))
    }

    fn argument_is_pg_age(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_age_call();
        self.pos = saved;
        nested
    }

    fn argument_is_pg_table_is_visible(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_table_is_visible_call();
        self.pos = saved;
        nested
    }

    fn argument_is_pg_get_expr(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_get_expr_call();
        self.pos = saved;
        nested
    }

    fn argument_is_pg_get_partkeydef(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_get_partkeydef_call();
        self.pos = saved;
        nested
    }

    fn argument_is_pg_get_userbyid(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_get_userbyid_call();
        self.pos = saved;
        nested
    }

    fn argument_is_pg_is_in_recovery(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_pg_is_in_recovery_call();
        self.pos = saved;
        nested
    }

    fn argument_is_current_database(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_current_database_call();
        self.pos = saved;
        nested
    }

    fn argument_is_current_schema(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_current_schema_call();
        self.pos = saved;
        nested
    }

    fn argument_is_version(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_version_call();
        self.pos = saved;
        nested
    }

    fn argument_is_to_char(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_to_char_call();
        self.pos = saved;
        nested
    }

    fn argument_is_json_extract(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_json_extract_call();
        self.pos = saved;
        nested
    }

    fn argument_is_to_uuid(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_to_uuid_call();
        self.pos = saved;
        nested
    }

    fn argument_is_make_geohash(&mut self) -> bool {
        let saved = self.pos;
        self.skip();
        let open = self.eat(b'(');
        self.skip();
        let nested = open && self.starts_make_geohash_call();
        self.pos = saved;
        nested
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
            source_label: format!("{name}({part_src} FROM {arg_label})"),
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(Sha256Expr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(Sha1Expr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(Md5Expr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToDateExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToTimestampExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DayOfWeekSundayFirstExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(IsEndOfMonthExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToStrExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DaysInMonthExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(IsLeapYearExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(TimestampCeilExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(TimestampFloorExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(WeekOfYearExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DayOfWeekExpr {
            source_label: label.clone(),
            label,
            args,
        })
    }

    fn starts_micros_call(&self) -> bool {
        self.starts_named_call("micros")
    }

    fn starts_nanos_call(&self) -> bool {
        self.starts_named_call("nanos")
    }

    fn starts_to_utc_call(&self) -> bool {
        self.starts_named_call("to_utc")
    }

    fn starts_to_timezone_call(&self) -> bool {
        self.starts_named_call("to_timezone")
    }

    fn starts_to_pg_date_call(&self) -> bool {
        self.starts_named_call("to_pg_date")
    }

    fn parse_to_pg_date(&mut self) -> Result<ToPgDateExpr, Error> {
        let name = self.ident_spelling()?;
        let fname = name.to_ascii_lowercase();
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!("function `{fname}` requires arguments")));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToPgDateExpr {
            source_label: label.clone(),
            label,
            args,
        })
    }

    fn parse_to_timezone(&mut self) -> Result<ToTimezoneExpr, Error> {
        let name = self.ident_spelling()?;
        let fname = name.to_ascii_lowercase();
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(TIMESTAMP, STRING)"
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToTimezoneExpr {
            source_label: label.clone(),
            label,
            args,
        })
    }

    fn parse_to_utc(&mut self) -> Result<ToUtcExpr, Error> {
        let name = self.ident_spelling()?;
        let fname = name.to_ascii_lowercase();
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(TIMESTAMP, STRING)"
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(ToUtcExpr {
            source_label: label.clone(),
            label,
            args,
        })
    }

    fn parse_nanos(&mut self) -> Result<NanosExpr, Error> {
        let name = self.ident_spelling()?;
        let fname = name.to_ascii_lowercase();
        self.skip();
        let open = self.pos;
        if self.eat(b'(') {
            self.skip();
            if self.peek() == Some(b'*') {
                return Err(Error::new(
                    "too few arguments for '*' [found=0,expected=2]".to_string(),
                ));
            }
        }
        self.pos = open;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(TIMESTAMP)"
            )));
        }
        Ok(NanosExpr {
            label: format!("{name}({})", labels.join(", ")),
            source_label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_micros(&mut self) -> Result<MicrosExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `micros` requires arguments: micros(TIMESTAMP)".to_string(),
            ));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(MicrosExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(MillisExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(SecondExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(MinuteExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(HourExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DayExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(MonthExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(YearExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DateDiffExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(DateAddExpr {
            source_label: label.clone(),
            label,
            args,
        })
    }

    fn starts_cast_call(&self) -> bool {
        self.starts_named_call("cast")
    }

    fn parse_cast(&mut self) -> Result<CastExpr, Error> {
        let name = self.ident_spelling()?;
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
            source_label: format!("{name}({column} AS {type_name})"),
            column,
            type_name,
            infix: false,
        })
    }

    fn starts_colon_cast(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.src.get(pos) == Some(&b':') && self.src.get(pos + 1) == Some(&b':')
    }

    fn parse_colon_cast(&mut self, name: String) -> Result<CastExpr, Error> {
        self.skip();
        if !(self.eat(b':') && self.eat(b':')) {
            return Err(Error::new(
                "unsupported: expected '::' after column".to_string(),
            ));
        }
        let mut type_name = self.ident_spelling()?;
        if type_name.eq_ignore_ascii_case("float") {
            type_name = "DOUBLE".to_string();
        }
        Ok(CastExpr {
            label: format!("{name}::{type_name}"),
            source_label: format!("{name}::{type_name}"),
            column: name,
            type_name,
            infix: true,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(NullIfExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(CoalesceExpr {
            source_label: label.clone(),
            label,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(SizePrettyExpr {
            source_label: label.clone().into(),
            label,
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

    fn starts_glob_call(&self) -> bool {
        self.starts_named_call("glob")
    }

    fn starts_ilike_call(&self) -> bool {
        self.starts_named_call("ilike")
    }

    fn starts_like_call(&self) -> bool {
        self.starts_named_call("like")
    }

    fn starts_switch_call(&self) -> bool {
        self.starts_named_call("switch")
    }

    fn starts_bar_call(&self) -> bool {
        self.starts_named_call("bar")
    }

    fn starts_timestamp_sequence_call(&self) -> bool {
        self.starts_named_call("timestamp_sequence")
    }

    fn parse_timestamp_sequence(&mut self) -> Result<TimestampSequenceExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `timestamp_sequence` requires arguments: timestamp_sequence(TIMESTAMP, LONG)",
            ));
        }
        Ok(TimestampSequenceExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_bar(&mut self) -> Result<BarExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `bar` requires arguments: bar(DOUBLE, DOUBLE, DOUBLE, INT)",
            ));
        }
        Ok(BarExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn parse_switch(&mut self) -> Result<SwitchExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("switch requires a key and a value"));
        }
        Ok(SwitchExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
    }

    fn starts_not_like(&self) -> bool {
        let Some(after_not) = self.keyword_at(self.pos, "not") else {
            return false;
        };
        self.keyword_at(after_not, "like").is_some()
    }

    fn parse_not_like(&mut self, name: String) -> Result<LikeExpr, Error> {
        if !self.eat_keyword("not") {
            return Err(Error::new(
                "unsupported: expected NOT LIKE after column".to_string(),
            ));
        }
        let mut expr = self.parse_like_op(name)?;
        expr.label = format!("not({})", expr.label);
        expr.negated = true;
        Ok(expr)
    }

    fn starts_like_infix(&self) -> bool {
        self.keyword_at(self.pos, "like").is_some()
    }

    fn parse_like_op(&mut self, name: String) -> Result<LikeExpr, Error> {
        if !self.eat_keyword("like") {
            return Err(Error::new(
                "unsupported: expected LIKE after column".to_string(),
            ));
        }
        let (pattern_label, pattern) = self.parse_call_arg(true)?;
        Ok(LikeExpr {
            label: format!("{name} like {pattern_label}"),
            source_label: format!("{name} like {pattern_label}"),
            args: vec![ConcatArg::Column(name), pattern],
            negated: false,
            regex: false,
        })
    }

    fn parse_like(&mut self) -> Result<LikeExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args("like", true)?;
        if args.is_empty() {
            return Err(Error::new(
                "too few arguments for 'like' [found=0,expected=2]",
            ));
        }
        let source_label = format!("{}({})", name, labels.join(", "));
        let label = if labels.len() == 2 {
            format!("{} like {}", labels[0], labels[1])
        } else {
            format!("like({})", labels.join(", "))
        };
        Ok(LikeExpr {
            label,
            source_label,
            args,
            negated: false,
            regex: false,
        })
    }

    fn starts_not_regex(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.src.get(pos) == Some(&b'!') && self.src.get(pos + 1) == Some(&b'~')
    }

    fn parse_not_regex(&mut self, name: String) -> Result<LikeExpr, Error> {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.src.get(self.pos) != Some(&b'!') || self.src.get(self.pos + 1) != Some(&b'~') {
            return Err(Error::new(
                "unsupported: expected !~ after column".to_string(),
            ));
        }
        self.pos += 2;
        let (pattern_label, pattern) = self.parse_call_arg(true)?;
        Ok(LikeExpr {
            label: format!("{name} !~ {pattern_label}"),
            source_label: format!("{name} !~ {pattern_label}"),
            args: vec![ConcatArg::Column(name), pattern],
            negated: true,
            regex: true,
        })
    }

    fn starts_regex_infix(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.src.get(pos) == Some(&b'~') && self.src.get(pos + 1) != Some(&b'=')
    }

    fn parse_regex_op(&mut self, name: String) -> Result<LikeExpr, Error> {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        if self.pos >= self.src.len() || self.src[self.pos] != b'~' {
            return Err(Error::new(
                "unsupported: expected ~ after column".to_string(),
            ));
        }
        self.pos += 1;
        let (pattern_label, pattern) = self.parse_call_arg(true)?;
        Ok(LikeExpr {
            label: format!("{name} ~ {pattern_label}"),
            source_label: format!("{name} ~ {pattern_label}"),
            args: vec![ConcatArg::Column(name), pattern],
            negated: false,
            regex: true,
        })
    }

    fn starts_not_ilike(&self) -> bool {
        let Some(after_not) = self.keyword_at(self.pos, "not") else {
            return false;
        };
        self.keyword_at(after_not, "ilike").is_some()
    }

    fn parse_not_ilike(&mut self, name: String) -> Result<IlikeExpr, Error> {
        if !self.eat_keyword("not") {
            return Err(Error::new(
                "unsupported: expected NOT ILIKE after column".to_string(),
            ));
        }
        let mut expr = self.parse_ilike_op(name)?;
        expr.label = format!("not({})", expr.label);
        expr.negated = true;
        Ok(expr)
    }

    fn starts_ilike_infix(&self) -> bool {
        self.keyword_at(self.pos, "ilike").is_some()
    }

    fn parse_ilike_op(&mut self, name: String) -> Result<IlikeExpr, Error> {
        if !self.eat_keyword("ilike") {
            return Err(Error::new(
                "unsupported: expected ILIKE after column".to_string(),
            ));
        }
        let (pattern_label, pattern) = self.parse_call_arg(true)?;
        Ok(IlikeExpr {
            label: format!("{name} ilike {pattern_label}"),
            source_label: format!("{name} ilike {pattern_label}"),
            args: vec![ConcatArg::Column(name), pattern],
            negated: false,
        })
    }

    fn parse_ilike(&mut self) -> Result<IlikeExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "too few arguments for 'ilike' [found=0,expected=2]",
            ));
        }
        let source_label = format!("{}({})", name, labels.join(", "));
        let label = if labels.len() == 2 {
            let value = match &args[0] {
                ConcatArg::Column(column) => column.to_ascii_lowercase(),
                _ => labels[0].clone(),
            };
            format!("{value} ilike {}", labels[1])
        } else {
            format!("ilike({})", labels.join(", "))
        };
        Ok(IlikeExpr {
            label,
            source_label,
            args,
            negated: false,
        })
    }

    fn parse_glob(&mut self) -> Result<GlobExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        Ok(GlobExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
        })
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
        let start = self.pos;
        let name = self.ident_spelling()?;
        self.skip();
        if self.eat(b'(') {
            self.skip();
            if self.starts_trim_call() {
                let trim = self.parse_trim()?;
                self.skip();
                if !self.eat(b',') {
                    return Err(Error::new("unsupported: expected ',' after replace value"));
                }
                let (old_label, old) = self.parse_call_arg(false)?;
                self.skip();
                if !self.eat(b',') {
                    return Err(Error::new(
                        "unsupported: expected ',' after replace old value",
                    ));
                }
                let (new_label, new) = self.parse_call_arg(false)?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new("unsupported: expected ')' after replace"));
                }
                return Ok(ReplaceExpr {
                    label: format!("{name}({}, {old_label}, {new_label})", trim.label),
                    args: vec![ConcatArg::Literal(Scalar::Null), old, new],
                    value_trim: Some(Box::new(trim)),
                });
            }
        }
        self.pos = start;
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
            value_trim: None,
        })
    }

    fn starts_regexp_replace_call(&self) -> bool {
        self.starts_named_call("regexp_replace")
    }

    fn parse_regexp_replace(&mut self) -> Result<RegexpReplaceExpr, Error> {
        let position = self.pos;
        let name = self.ident_spelling()?;
        let call_start = self.pos;
        self.skip();
        let nested_value = if self.eat(b'(') {
            self.skip();
            if self.starts_pad_call() {
                let value_position = self.pos;
                let value = self.parse_pad()?;
                self.skip();
                if !self.eat(b',') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {name}"
                    )));
                }
                self.skip();
                let pattern_position = self.pos;
                let (pattern_label, pattern) = self.parse_call_arg(true)?;
                self.skip();
                if !self.eat(b',') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {name}"
                    )));
                }
                self.skip();
                let replacement_position = self.pos;
                let (replacement_label, replacement) = self.parse_call_arg(true)?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {name}"
                    )));
                }
                Some((
                    vec![value.label.clone(), pattern_label, replacement_label],
                    vec![ConcatArg::Literal(Scalar::Null), pattern, replacement],
                    vec![value_position, pattern_position, replacement_position],
                    Box::new(Proj::Pad(value)),
                ))
            } else {
                self.pos = call_start;
                None
            }
        } else {
            self.pos = call_start;
            None
        };
        let (labels, args, arg_positions, value_expr) = match nested_value {
            Some((labels, args, positions, value)) => (labels, args, positions, Some(value)),
            None => {
                let (labels, args, positions) = self.parse_call_args_with_positions(&name, true)?;
                (labels, args, positions, None)
            }
        };
        if args.is_empty() {
            return Err(Error::new(
                "function `regexp_replace` requires arguments".to_string(),
            ));
        }
        Ok(RegexpReplaceExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            value_expr,
            position,
            arg_positions,
        })
    }

    fn starts_trim_call(&self) -> bool {
        self.starts_named_call("ltrim")
            || self.starts_named_call("rtrim")
            || self.starts_named_call("trim")
    }

    fn parse_trim(&mut self) -> Result<TrimExpr, Error> {
        let start = self.pos;
        let name = self.ident_spelling()?;
        let end = if name.eq_ignore_ascii_case("ltrim") {
            TrimEnd::Left
        } else if name.eq_ignore_ascii_case("rtrim") {
            TrimEnd::Right
        } else {
            TrimEnd::Both
        };
        self.skip();
        if self.eat(b'(') {
            self.skip();
            let nested_case = if self.starts_named_call("lower") {
                Some(NestedTextCase::Lower(self.parse_lower()?))
            } else if self.starts_named_call("upper") {
                Some(NestedTextCase::Upper(self.parse_upper()?))
            } else {
                None
            };
            if let Some(nested_case) = nested_case {
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {}",
                        name.to_ascii_lowercase()
                    )));
                }
                let nested_label = match &nested_case {
                    NestedTextCase::Lower(expr) => &expr.label,
                    NestedTextCase::Upper(expr) => &expr.label,
                };
                return Ok(TrimExpr {
                    label: format!("{name}({nested_label})"),
                    args: Vec::new(),
                    end,
                    nested_case: Some(nested_case),
                    nested_replace: None,
                });
            }
            if self.starts_named_call("replace") {
                let replace = self.parse_replace()?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {}",
                        name.to_ascii_lowercase()
                    )));
                }
                let label = format!("{name}({})", replace.label);
                return Ok(TrimExpr {
                    label,
                    args: Vec::new(),
                    end,
                    nested_case: None,
                    nested_replace: Some(Box::new(replace)),
                });
            }
        }
        self.pos = start;
        let name = self.ident_spelling()?;
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
            nested_case: None,
            nested_replace: None,
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
        let start = self.pos;
        let name = self.ident_spelling()?;
        self.skip();
        if self.eat(b'(') {
            self.skip();
            if self.starts_trim_call() {
                let trim = self.parse_trim()?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new("unsupported: expected ')' after upper"));
                }
                let label = format!("{name}({})", trim.label);
                return Ok(UpperExpr {
                    source_label: label.clone().into(),
                    label,
                    args: Vec::new(),
                    nested_trim: Some(Box::new(trim)),
                    nested_replace: None,
                });
            } else if self.starts_named_call("replace") {
                let replace = self.parse_replace()?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new("unsupported: expected ')' after upper"));
                }
                let label = format!("{name}({})", replace.label);
                return Ok(UpperExpr {
                    source_label: label.clone().into(),
                    label,
                    args: Vec::new(),
                    nested_trim: None,
                    nested_replace: Some(Box::new(replace)),
                });
            }
        }
        self.pos = start;
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(UpperExpr {
            source_label: label.clone().into(),
            label,
            args,
            nested_trim: None,
            nested_replace: None,
        })
    }

    fn parse_lower(&mut self) -> Result<LowerExpr, Error> {
        let start = self.pos;
        let name = self.ident_spelling()?;
        self.skip();
        if self.eat(b'(') {
            self.skip();
            if self.starts_trim_call() {
                let trim = self.parse_trim()?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new("unsupported: expected ')' after lower"));
                }
                let label = format!("{name}({})", trim.label);
                return Ok(LowerExpr {
                    source_label: label.clone().into(),
                    label,
                    args: Vec::new(),
                    nested_trim: Some(Box::new(trim)),
                    nested_replace: None,
                });
            } else if self.starts_named_call("replace") {
                let replace = self.parse_replace()?;
                self.skip();
                if !self.eat(b')') {
                    return Err(Error::new("unsupported: expected ')' after lower"));
                }
                let label = format!("{name}({})", replace.label);
                return Ok(LowerExpr {
                    source_label: label.clone().into(),
                    label,
                    args: Vec::new(),
                    nested_trim: None,
                    nested_replace: Some(Box::new(replace)),
                });
            }
        }
        self.pos = start;
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, false)?;
        if args.is_empty() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        let label = format!("{name}({})", labels.join(", "));
        Ok(LowerExpr {
            source_label: label.clone().into(),
            label,
            args,
            nested_trim: None,
            nested_replace: None,
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
        let label = format!("{name}({})", labels.join(", "));
        Ok(LengthExpr {
            source_label: label.clone().into(),
            label,
            args,
            bytes,
        })
    }

    fn parse_call_args(
        &mut self,
        name: &str,
        numbers: bool,
    ) -> Result<(Vec<String>, Vec<ConcatArg>), Error> {
        let (labels, args, _) = self.parse_call_args_with_positions(name, numbers)?;
        Ok((labels, args))
    }

    fn parse_call_args_with_positions(
        &mut self,
        name: &str,
        numbers: bool,
    ) -> Result<ParsedCallArgsWithPositions, Error> {
        self.skip();
        if !self.eat(b'(') {
            return Err(Error::new(format!(
                "unsupported: expected '(' after {name}"
            )));
        }
        self.skip();
        if self.peek() == Some(b')') {
            self.pos += 1;
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }
        let mut args = Vec::new();
        let mut labels = Vec::new();
        let mut arg_positions = Vec::new();
        loop {
            self.skip();
            arg_positions.push(self.pos);
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
        Ok((labels, args, arg_positions))
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
            let mut spelling = self.ident_spelling()?;
            let arg = if spelling.eq_ignore_ascii_case("null") {
                ConcatArg::Literal(Scalar::Null)
            } else if spelling.eq_ignore_ascii_case("true") {
                ConcatArg::Literal(Scalar::Boolean(true))
            } else if spelling.eq_ignore_ascii_case("false") {
                ConcatArg::Literal(Scalar::Boolean(false))
            } else {
                self.skip();
                if self.eat(b'.') {
                    spelling.push('.');
                    spelling.push_str(&self.ident_spelling()?);
                }
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

    fn starts_pipes(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        self.src[pos..].starts_with(b"||")
    }

    fn parse_concat_op(&mut self, name: String) -> Result<ConcatExpr, Error> {
        self.skip();
        if !self.eat(b'|') || !self.eat(b'|') {
            return Err(Error::new(
                "unsupported: expected '||' after column".to_string(),
            ));
        }
        let right = self.ident_spelling()?;
        Ok(ConcatExpr {
            label: format!("concat({name}, {right})"),
            args: vec![ConcatArg::Column(name), ConcatArg::Column(right)],
        })
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
            "count"
                | "min"
                | "max"
                | "sum"
                | "avg"
                | "first"
                | "last"
                | "count_distinct"
                | "stddev"
                | "variance"
                | "var_samp"
                | "var_pop"
                | "stddev_pop"
                | "stddev_samp"
                | "ksum"
                | "nsum"
                | "geomean"
                | "skewness"
                | "skewness_pop"
                | "skewness_samp"
                | "kurtosis_pop"
                | "kurtosis"
                | "kurtosis_samp"
                | "sparkline"
                | "mode"
                | "approx_median"
                | "approx_percentile"
                | "arg_max"
                | "arg_min"
                | "approx_count_distinct"
                | "string_agg"
                | "string_distinct_agg"
                | "covar_pop"
                | "vwap"
                | "twap"
                | "weighted_avg"
                | "weighted_stddev"
                | "weighted_stddev_rel"
                | "weighted_stddev_freq"
                | "covar_samp"
                | "corr"
                | "regr_slope"
                | "regr_intercept"
                | "regr_r2"
                | "bool_and"
                | "bool_or"
                | "isordered"
                | "percentile_disc"
                | "quantile_disc"
                | "percentile_cont"
                | "quantile_cont"
                | "first_not_null"
                | "last_not_null"
                | "bit_and"
                | "bit_or"
                | "bit_xor"
                | "haversine_dist_deg"
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
        let position = self.pos;
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
            Some(self.qualified_ident_spelling()?)
        };
        let func = match name.to_ascii_lowercase().as_str() {
            "count" => AggFunc::Count,
            "min" => AggFunc::Min,
            "max" => AggFunc::Max,
            "sum" => AggFunc::Sum,
            "avg" => AggFunc::Avg,
            "first" => AggFunc::First,
            "last" => AggFunc::Last,
            "count_distinct" => AggFunc::CountDistinct,
            "stddev" => AggFunc::StdDev,
            "variance" => AggFunc::Variance,
            "var_samp" => AggFunc::VarSamp,
            "var_pop" => AggFunc::VarPop,
            "stddev_pop" => AggFunc::StdDevPop,
            "stddev_samp" => AggFunc::StdDevSamp,
            "ksum" => AggFunc::KSum,
            "nsum" => AggFunc::NSum,
            "geomean" => AggFunc::GeoMean,
            "skewness" => AggFunc::Skewness,
            "skewness_pop" => AggFunc::SkewnessPop,
            "skewness_samp" => AggFunc::SkewnessSamp,
            "kurtosis_pop" => AggFunc::KurtosisPop,
            "kurtosis" => AggFunc::Kurtosis,
            "kurtosis_samp" => AggFunc::KurtosisSamp,
            "sparkline" => AggFunc::Sparkline,
            "mode" => AggFunc::Mode,
            "approx_median" => AggFunc::ApproxMedian,
            "approx_percentile" => AggFunc::ApproxPercentile,
            "arg_max" => AggFunc::ArgMax,
            "arg_min" => AggFunc::ArgMin,
            "approx_count_distinct" => AggFunc::ApproxCountDistinct,
            "string_agg" => AggFunc::StringAgg,
            "string_distinct_agg" => AggFunc::StringDistinctAgg,
            "covar_pop" => AggFunc::CovarPop,
            "vwap" => AggFunc::Vwap,
            "twap" => AggFunc::Twap,
            "weighted_avg" => AggFunc::WeightedAvg,
            "weighted_stddev" => AggFunc::WeightedStdDev,
            "weighted_stddev_rel" => AggFunc::WeightedStdDevRel,
            "weighted_stddev_freq" => AggFunc::WeightedStdDevFreq,
            "covar_samp" => AggFunc::CovarSamp,
            "corr" => AggFunc::Corr,
            "regr_slope" => AggFunc::RegrSlope,
            "regr_intercept" => AggFunc::RegrIntercept,
            "regr_r2" => AggFunc::RegrR2,
            "bool_and" => AggFunc::BoolAnd,
            "bool_or" => AggFunc::BoolOr,
            "isordered" => AggFunc::IsOrdered,
            "percentile_disc" => AggFunc::PercentileDisc,
            "quantile_disc" => AggFunc::QuantileDisc,
            "percentile_cont" => AggFunc::PercentileCont,
            "quantile_cont" => AggFunc::QuantileCont,
            "first_not_null" => AggFunc::FirstNotNull,
            "last_not_null" => AggFunc::LastNotNull,
            "bit_and" => AggFunc::BitAnd,
            "bit_or" => AggFunc::BitOr,
            "bit_xor" => AggFunc::BitXor,
            "haversine_dist_deg" => AggFunc::Haversine,
            _ => return Err(Error::new(format!("internal: aggregate `{name}`"))),
        };
        if func == AggFunc::Haversine {
            return self.finish_haversine(name, column, position);
        }
        if func != AggFunc::Count && starred {
            return Err(Error::new("too few arguments for '*' [found=0,expected=2]"));
        }
        self.skip();
        if matches!(
            func,
            AggFunc::StdDev
                | AggFunc::Variance
                | AggFunc::VarSamp
                | AggFunc::VarPop
                | AggFunc::StdDevPop
                | AggFunc::StdDevSamp
                | AggFunc::KSum
                | AggFunc::NSum
                | AggFunc::GeoMean
                | AggFunc::Skewness
                | AggFunc::SkewnessPop
                | AggFunc::SkewnessSamp
                | AggFunc::KurtosisPop
                | AggFunc::KurtosisSamp
                | AggFunc::Kurtosis
                | AggFunc::BoolAnd
                | AggFunc::BoolOr
        ) && self.peek() != Some(b')')
        {
            let mut provided = if column.is_some() { 1 } else { 0 };
            while self.peek().is_some() && self.peek() != Some(b')') {
                if self.eat(b',') {
                    provided += 1;
                } else {
                    self.pos += 1;
                }
            }
            if !self.eat(b')') {
                return Err(Error::new(format!(
                    "unsupported: expected ')' after {name}"
                )));
            }
            let fname = name.to_ascii_lowercase();
            return Err(Error::new(format!(
                "wrong number of arguments for function `{fname}`; expected: 1, provided: {provided}"
            )));
        }
        let mut third_type = None;
        let mut third_value = None;
        let mut third_label = None;
        let mut fourth_type = None;
        let mut precision = None;
        let mut precision_more = false;
        let mut precision_label = None;
        let mut separator = None;
        let mut separator_column = None;
        let mut separator_more = None;
        let mut separator_label = None;
        let mut pair = None;
        let mut extra = None;
        let mut spark_label = None;
        if matches!(func, AggFunc::StringAgg | AggFunc::StringDistinctAgg)
            && self.peek() == Some(b',')
        {
            self.pos += 1;
            self.skip();
            let start = self.pos;
            if self.peek() == Some(b'\'') {
                separator = Some(Scalar::Text(self.parse_string()?));
            } else if self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
                let word = self.ident_spelling()?;
                if word.eq_ignore_ascii_case("null") {
                    separator = Some(Scalar::Null);
                } else {
                    separator_column = Some(word);
                }
            } else {
                return Err(Error::new(format!(
                    "unsupported: {} separator must be a literal",
                    name.to_ascii_lowercase()
                )));
            }
            separator_label = Some(
                std::str::from_utf8(&self.src[start..self.pos])
                    .unwrap_or("")
                    .to_string(),
            );
            self.skip();
            if self.eat(b',') {
                self.skip();
                let more_start = self.pos;
                if self.peek() == Some(b'\'') {
                    separator_more = Some(Scalar::Text(self.parse_string()?));
                } else if self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
                    let word = self.ident_spelling()?;
                    separator_more = if word.eq_ignore_ascii_case("null") {
                        Some(Scalar::Null)
                    } else {
                        Some(Scalar::Text(word))
                    };
                }
                if self.pos == more_start {
                    return Err(Error::new(format!(
                        "unsupported: {} separator must be a literal",
                        name.to_ascii_lowercase()
                    )));
                }
                self.skip();
            }
        }
        if func == AggFunc::ApproxMedian && self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            let start = self.pos;
            if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                let word = self.ident_spelling()?;
                extra = if word.eq_ignore_ascii_case("null") {
                    Some(AggregateExtra::Type("NULL".to_string()))
                } else {
                    Some(AggregateExtra::Column(word))
                };
                precision_label = Some(
                    std::str::from_utf8(&self.src[start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
            } else {
                precision = Some(self.parse_number()?);
                precision_label = Some(
                    std::str::from_utf8(&self.src[start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
            }
            self.skip();
            if self.peek() == Some(b',') {
                self.pos += 1;
                self.skip();
                let third_start = self.pos;
                if self
                    .peek()
                    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                {
                    let word = self.ident_spelling()?;
                    third_type = if word.eq_ignore_ascii_case("null") {
                        Some("NULL".to_string())
                    } else {
                        Some(word)
                    };
                } else if self.starts_limit_int()
                    || self.peek() == Some(b'.')
                    || self.peek() == Some(b'-')
                {
                    let number = self.parse_number()?;
                    third_type = Some(median_literal_type(&number).to_string());
                }
                if self.pos == third_start {
                    return Err(Error::new(
                        "unsupported: approx_median precision must be a literal",
                    ));
                }
                let _ = third_start;
            }
        }
        if func == AggFunc::ApproxPercentile && self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            let start = self.pos;
            if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                let word = self.ident_spelling()?;
                extra = if word.eq_ignore_ascii_case("null") {
                    Some(AggregateExtra::Type("NULL".to_string()))
                } else {
                    Some(AggregateExtra::Column(word))
                };
                precision_label = Some(
                    std::str::from_utf8(&self.src[start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
            } else {
                precision = Some(self.parse_number()?);
                precision_label = Some(
                    std::str::from_utf8(&self.src[start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
            }
            self.skip();
            if self.peek() == Some(b',') {
                self.pos += 1;
                self.skip();
                let third_start = self.pos;
                if self
                    .peek()
                    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                {
                    let word = self.ident_spelling()?;
                    third_type = if word.eq_ignore_ascii_case("null") {
                        Some("NULL".to_string())
                    } else {
                        Some(word)
                    };
                } else if self.starts_limit_int()
                    || self.peek() == Some(b'.')
                    || self.peek() == Some(b'-')
                {
                    let number = self.parse_number()?;
                    third_type = Some(median_literal_type(&number).to_string());
                    third_value = Some(number);
                }
                if self.pos == third_start {
                    return Err(Error::new(
                        "unsupported: approx_percentile precision must be a literal",
                    ));
                }
                third_label = Some(
                    std::str::from_utf8(&self.src[third_start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
                self.skip();
                if self.peek() == Some(b',') {
                    self.pos += 1;
                    self.skip();
                    let fourth_start = self.pos;
                    if self
                        .peek()
                        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                    {
                        let word = self.ident_spelling()?;
                        fourth_type = if word.eq_ignore_ascii_case("null") {
                            Some("NULL".to_string())
                        } else {
                            Some(word)
                        };
                    } else if self.starts_limit_int()
                        || self.peek() == Some(b'.')
                        || self.peek() == Some(b'-')
                    {
                        let number = self.parse_number()?;
                        fourth_type = Some(median_literal_type(&number).to_string());
                    }
                    if self.pos == fourth_start {
                        return Err(Error::new(
                            "unsupported: approx_percentile precision must be a literal",
                        ));
                    }
                }
            }
        }
        if func == AggFunc::ApproxCountDistinct && self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            let start = self.pos;
            if self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
                let word = self.ident_spelling()?;
                if word.eq_ignore_ascii_case("null") {
                    precision = Some(Scalar::Null);
                    precision_label = Some(word);
                } else {
                    return Err(Error::new(format!("Invalid column: {word}")));
                }
            } else {
                precision = Some(self.parse_number()?);
                precision_label = Some(
                    std::str::from_utf8(&self.src[start..self.pos])
                        .unwrap_or("")
                        .to_string(),
                );
            }
            self.skip();
            if self.eat(b',') {
                precision_more = true;
                self.skip();
                if self.peek().is_some_and(|byte| byte.is_ascii_alphabetic()) {
                    let _ = self.ident_spelling()?;
                } else if self.starts_limit_int() || self.peek() == Some(b'.') {
                    let _ = self.parse_number()?;
                }
                self.skip();
            }
        }
        if matches!(
            func,
            AggFunc::CovarPop
                | AggFunc::CovarSamp
                | AggFunc::Corr
                | AggFunc::RegrSlope
                | AggFunc::RegrIntercept
                | AggFunc::RegrR2
                | AggFunc::Vwap
                | AggFunc::Twap
                | AggFunc::WeightedAvg
                | AggFunc::WeightedStdDev
                | AggFunc::WeightedStdDevRel
                | AggFunc::WeightedStdDevFreq
        ) && self.peek() == Some(b',')
        {
            self.pos += 1;
            self.skip();
            if !self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                let message = if func == AggFunc::Vwap {
                    "unsupported: vwap expects a column"
                } else if func == AggFunc::Twap {
                    "unsupported: twap expects a column"
                } else if func == AggFunc::WeightedAvg {
                    "unsupported: weighted_avg expects a column"
                } else if func == AggFunc::WeightedStdDev {
                    "unsupported: weighted_stddev expects a column"
                } else if func == AggFunc::WeightedStdDevRel {
                    "unsupported: weighted_stddev_rel expects a column"
                } else if func == AggFunc::WeightedStdDevFreq {
                    "unsupported: weighted_stddev_freq expects a column"
                } else {
                    "unsupported: covariance expects a column"
                };
                return Err(Error::new(message));
            }
            pair = Some(self.qualified_ident_spelling()?);
            self.skip();
            if self.peek() == Some(b',') {
                let mut provided = 2;
                while self.peek().is_some() && self.peek() != Some(b')') {
                    if self.eat(b',') {
                        provided += 1;
                    } else {
                        self.pos += 1;
                    }
                }
                if !self.eat(b')') {
                    return Err(Error::new(format!(
                        "unsupported: expected ')' after {name}"
                    )));
                }
                return Err(Error::new(format!(
                    "wrong number of arguments for function `{}`; expected: 2, provided: {provided}",
                    name.to_ascii_lowercase()
                )));
            }
        }
        if matches!(func, AggFunc::ArgMax | AggFunc::ArgMin) && self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                pair = Some(self.qualified_ident_spelling()?);
            } else {
                return Err(Error::new(format!(
                    "unsupported: {} expects a column",
                    name.to_ascii_lowercase()
                )));
            }
            self.skip();
            if self.peek() == Some(b',') {
                self.pos += 1;
                self.skip();
                let third_start = self.pos;
                if self
                    .peek()
                    .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                {
                    let word = self.ident_spelling()?;
                    third_type = if word.eq_ignore_ascii_case("null") {
                        Some("NULL".to_string())
                    } else {
                        Some(word)
                    };
                } else if self.starts_limit_int()
                    || self.peek() == Some(b'.')
                    || self.peek() == Some(b'-')
                {
                    let number = self.parse_number()?;
                    third_type = Some(median_literal_type(&number).to_string());
                }
                if self.pos == third_start {
                    return Err(Error::new(format!(
                        "unsupported: {} expects a column",
                        name.to_ascii_lowercase()
                    )));
                }
            }
        }
        if matches!(
            func,
            AggFunc::FirstNotNull
                | AggFunc::LastNotNull
                | AggFunc::BitAnd
                | AggFunc::BitOr
                | AggFunc::BitXor
                | AggFunc::Mode
                | AggFunc::IsOrdered
        ) && self.peek() == Some(b',')
        {
            self.pos += 1;
            self.skip();
            if self.peek() == Some(b'\'') {
                let _ = self.parse_string()?;
                extra = Some(AggregateExtra::Type("STRING".to_string()));
            } else if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                let word = self.ident_spelling()?;
                extra = if word.eq_ignore_ascii_case("null") {
                    Some(AggregateExtra::Type("NULL".to_string()))
                } else {
                    Some(AggregateExtra::Column(word))
                };
            } else if self.peek() == Some(b'-')
                || self.peek() == Some(b'.')
                || self.peek().is_some_and(|byte| byte.is_ascii_digit())
            {
                let number = self.parse_number()?;
                let kind = match number {
                    Scalar::Double(_) => "DOUBLE",
                    Scalar::Long(value) if i32::try_from(value).is_ok() => "INT",
                    Scalar::Long(_) => "LONG",
                    _ => "DOUBLE",
                };
                extra = Some(AggregateExtra::Type(kind.to_string()));
            } else {
                return Err(Error::new(format!(
                    "unsupported: {} extra argument",
                    name.to_ascii_lowercase()
                )));
            }
            self.skip();
        }
        if func == AggFunc::Sparkline && self.peek() == Some(b',') {
            let column_name = column.clone().unwrap_or_default();
            let parsed = self.parse_spark_args()?;
            if parsed.matched {
                precision_more = true;
                precision = parsed.width.map(|width| Scalar::Long(i64::from(width)));
                third_value = parsed
                    .min_bits
                    .map(|bits| Scalar::Double(f64::from_bits(bits)));
                separator = parsed
                    .max_bits
                    .map(|bits| Scalar::Double(f64::from_bits(bits)));
            }
            spark_label = Some(format!("{column_name}, {}", parsed.suffix));
            fourth_type = parsed.mismatch;
        }
        if matches!(
            func,
            AggFunc::PercentileDisc
                | AggFunc::QuantileDisc
                | AggFunc::PercentileCont
                | AggFunc::QuantileCont
        ) && self.peek() == Some(b',')
        {
            let fname = name.to_ascii_lowercase();
            self.pos += 1;
            self.skip();
            let start = self.pos;
            if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                let word = self.ident_spelling()?;
                extra = if word.eq_ignore_ascii_case("null") {
                    Some(AggregateExtra::Type("NULL".to_string()))
                } else {
                    Some(AggregateExtra::Column(word))
                };
            } else if self.starts_limit_int()
                || self.peek() == Some(b'.')
                || self.peek() == Some(b'-')
            {
                precision = Some(self.parse_number()?);
            } else {
                return Err(Error::new(format!(
                    "unsupported: {fname} expects a percentile"
                )));
            }
            precision_label = Some(
                std::str::from_utf8(&self.src[start..self.pos])
                    .unwrap_or("")
                    .to_string(),
            );
            self.skip();
            if self.peek() == Some(b',') {
                let mut extras = Vec::new();
                while self.peek() == Some(b',') {
                    self.pos += 1;
                    self.skip();
                    let token_start = self.pos;
                    if self
                        .peek()
                        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
                    {
                        let word = self.ident_spelling()?;
                        extras.push(if word.eq_ignore_ascii_case("null") {
                            "NULL".to_string()
                        } else {
                            format!("col:{word}")
                        });
                    } else if self.starts_limit_int()
                        || self.peek() == Some(b'.')
                        || self.peek() == Some(b'-')
                    {
                        let number = self.parse_number()?;
                        extras.push(median_literal_type(&number).to_string());
                    } else {
                        return Err(Error::new(format!(
                            "unsupported: {fname} expects a percentile"
                        )));
                    }
                    let _ = token_start;
                    self.skip();
                }
                fourth_type = Some(extras.join(", "));
            }
        }
        if !self.eat(b')') {
            return Err(Error::new(format!(
                "unsupported: expected ')' after {name}"
            )));
        }
        if matches!(
            func,
            AggFunc::StdDev
                | AggFunc::Variance
                | AggFunc::VarSamp
                | AggFunc::VarPop
                | AggFunc::StdDevPop
                | AggFunc::StdDevSamp
                | AggFunc::KSum
                | AggFunc::NSum
                | AggFunc::GeoMean
                | AggFunc::Skewness
                | AggFunc::SkewnessPop
                | AggFunc::SkewnessSamp
                | AggFunc::KurtosisPop
                | AggFunc::KurtosisSamp
                | AggFunc::Kurtosis
        ) && column.is_none()
        {
            let fname = name.to_ascii_lowercase();
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(DOUBLE)"
            )));
        }
        if func == AggFunc::Twap && column.is_none() {
            return Err(Error::new(
                "function `twap` requires arguments: twap(DOUBLE, TIMESTAMP)",
            ));
        }
        if matches!(
            func,
            AggFunc::CovarPop
                | AggFunc::CovarSamp
                | AggFunc::Corr
                | AggFunc::RegrSlope
                | AggFunc::RegrIntercept
                | AggFunc::RegrR2
                | AggFunc::Vwap
                | AggFunc::WeightedAvg
                | AggFunc::WeightedStdDev
                | AggFunc::WeightedStdDevRel
                | AggFunc::WeightedStdDevFreq
        ) && column.is_none()
        {
            let fname = name.to_ascii_lowercase();
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(DOUBLE, DOUBLE)"
            )));
        }
        if matches!(
            func,
            AggFunc::CovarPop
                | AggFunc::CovarSamp
                | AggFunc::Corr
                | AggFunc::RegrSlope
                | AggFunc::RegrIntercept
                | AggFunc::RegrR2
                | AggFunc::Vwap
                | AggFunc::Twap
                | AggFunc::WeightedAvg
                | AggFunc::WeightedStdDev
                | AggFunc::WeightedStdDevRel
                | AggFunc::WeightedStdDevFreq
        ) && pair.is_none()
        {
            return Err(Error::new(format!(
                "wrong number of arguments for function `{}`; expected: 2, provided: 1",
                name.to_ascii_lowercase()
            )));
        }
        if func == AggFunc::IsOrdered && column.is_none() {
            return Err(Error::new("function `isOrdered` requires arguments"));
        }
        if matches!(func, AggFunc::BoolAnd | AggFunc::BoolOr) && column.is_none() {
            let fname = name.to_ascii_lowercase();
            return Err(Error::new(format!(
                "function `{fname}` requires arguments: {fname}(BOOLEAN)"
            )));
        }
        if func != AggFunc::Count && column.is_none() {
            return Err(Error::new(format!(
                "function `{}` requires arguments",
                name.to_ascii_lowercase()
            )));
        }
        let label = if let Some(suffix) = spark_label {
            format!("{name}({suffix})")
        } else if let (Some(column), Some(second), Some(third)) =
            (&column, &precision_label, &third_label)
        {
            format!("{name}({column}, {second}, {third})")
        } else {
            match (&column, &precision_label, &separator_label, &pair) {
                (Some(column), Some(precision), _, _) => format!("{name}({column}, {precision})"),
                (Some(column), None, Some(separator), _) => {
                    format!("{name}({column}, {separator})")
                }
                (Some(column), None, None, Some(pair)) => format!("{name}({column}, {pair})"),
                (Some(column), None, None, None) => format!("{name}({column})"),
                (None, _, _, _) => format!("{name}()"),
            }
        };
        Ok(AggregateExpr {
            func,
            position,
            label,
            column,
            precision,
            precision_more,
            separator,
            separator_column,
            separator_more,
            pair,
            extra,
            third_type,
            third_value,
            fourth_type,
        })
    }

    fn finish_haversine(
        &mut self,
        name: String,
        lat: Option<String>,
        position: usize,
    ) -> Result<AggregateExpr, Error> {
        let fname = "haversine_dist_deg";
        self.skip();
        if lat.is_none() {
            if self.eat(b')') {
                return Err(Error::new(format!(
                    "function `{fname}` requires arguments: {fname}(DOUBLE, DOUBLE, TIMESTAMP)"
                )));
            }
            return Err(Error::new(format!(
                "unsupported: {fname} argument near `{}`",
                self.rest()
            )));
        }
        let mut names = vec![lat.expect("lat")];
        while self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            if self
                .peek()
                .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
            {
                names.push(self.ident_spelling()?);
            } else {
                return Err(Error::new(format!(
                    "unsupported: {fname} argument near `{}`",
                    self.rest()
                )));
            }
            self.skip();
        }
        if !self.eat(b')') {
            return Err(Error::new(format!(
                "unsupported: expected ')' after {name}"
            )));
        }
        if names.len() != 3 {
            return Err(Error::new(format!(
                "wrong number of arguments for function `{fname}`; expected: 3, provided: {}",
                names.len()
            )));
        }
        Ok(AggregateExpr {
            func: AggFunc::Haversine,
            position,
            label: format!("{name}({}, {}, {})", names[0], names[1], names[2]),
            column: Some(names[0].clone()),
            precision: None,
            precision_more: false,
            separator: None,
            separator_column: None,
            separator_more: None,
            pair: Some(names[1].clone()),
            extra: Some(AggregateExtra::Column(names[2].clone())),
            third_type: None,
            third_value: None,
            fourth_type: None,
        })
    }

    fn parse_spark_args(&mut self) -> Result<ParsedSpark, Error> {
        let mut values = Vec::new();
        let mut types = Vec::new();
        let mut labels = Vec::new();
        while self.peek() == Some(b',') {
            self.pos += 1;
            self.skip();
            let start = self.pos;
            let (value, kind) = self.parse_spark_arg()?;
            let spelling = std::str::from_utf8(&self.src[start..self.pos])
                .unwrap_or("")
                .to_string();
            labels.push(spelling);
            types.push(kind);
            values.push(value);
            self.skip();
        }
        let suffix = labels.join(", ");
        let mismatch = Some(types.join(", "));
        let reject = ParsedSpark {
            matched: false,
            min_bits: None,
            max_bits: None,
            width: None,
            suffix: suffix.clone(),
            mismatch: mismatch.clone(),
        };
        if values.len() != 3 || types[2].starts_with("col:") {
            return Ok(reject);
        }
        let width = match &values[2] {
            Scalar::Null => None,
            Scalar::Long(value) => match i32::try_from(*value) {
                Ok(value) => Some(value),
                Err(_) => return Ok(reject),
            },
            _ => return Ok(reject),
        };
        let min_bits = match spark_bound(&values[0], &types[0])? {
            Some(SparkBound::Number(number)) => Some(number.to_bits()),
            Some(SparkBound::Auto) => None,
            None => return Ok(reject),
        };
        let max_bits = match spark_bound(&values[1], &types[1])? {
            Some(SparkBound::Number(number)) => Some(number.to_bits()),
            Some(SparkBound::Auto) => None,
            None => return Ok(reject),
        };
        Ok(ParsedSpark {
            matched: true,
            min_bits,
            max_bits,
            width,
            suffix,
            mismatch: None,
        })
    }

    fn parse_spark_arg(&mut self) -> Result<(Scalar, String), Error> {
        if self.peek() == Some(b'\'') {
            let text = self.parse_string()?;
            let kind = if text.chars().count() <= 1 {
                "CHAR"
            } else {
                "STRING"
            };
            return Ok((Scalar::Text(text), kind.to_string()));
        }
        if matches!(self.peek(), Some(byte) if byte.is_ascii_digit() || byte == b'-') {
            let number = self.parse_number()?;
            let kind = median_literal_type(&number).to_string();
            return Ok((number, kind));
        }
        let word = self.ident_spelling()?;
        if word.eq_ignore_ascii_case("null") {
            return Ok((Scalar::Null, "NULL".to_string()));
        }
        Ok((Scalar::Null, format!("col:{word}")))
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
        self.parse_filter_or()
    }

    fn parse_filter_or(&mut self) -> Result<Filter, Error> {
        let filter = self.parse_filter_and()?;
        self.parse_filter_or_tail(filter)
    }

    fn parse_filter_or_tail(&mut self, mut filter: Filter) -> Result<Filter, Error> {
        while self.eat_keyword("or") {
            let right = self.parse_filter_and()?;
            filter = Filter::Or(Box::new(filter), Box::new(right));
        }
        Ok(filter)
    }

    fn parse_filter_and(&mut self) -> Result<Filter, Error> {
        let filter = self.parse_filter_atom()?;
        self.parse_filter_and_tail(filter)
    }

    fn parse_filter_and_tail(&mut self, mut filter: Filter) -> Result<Filter, Error> {
        while self.eat_keyword("and") {
            let right = self.parse_filter_atom()?;
            filter = Filter::And(Box::new(filter), Box::new(right));
        }
        Ok(filter)
    }

    fn filter_function_ahead(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
            pos += 1;
        }
        loop {
            if !matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_')
            {
                return false;
            }
            pos += 1;
            while matches!(self.src.get(pos), Some(byte) if byte.is_ascii_alphanumeric() || *byte == b'_')
            {
                pos += 1;
            }
            while pos < self.src.len() && self.src[pos].is_ascii_whitespace() {
                pos += 1;
            }
            if self.src.get(pos) == Some(&b'.') {
                pos += 1;
                continue;
            }
            return self.src.get(pos) == Some(&b'(');
        }
    }

    fn parse_filter_function_comparison(&mut self) -> Result<Filter, Error> {
        let start = self.pos;
        if let Some((operator_start, negated, operator_len)) =
            top_level_regex_operator(self.src, start)
        {
            let label = std::str::from_utf8(&self.src[start..operator_start])
                .map_err(|_| Error::new("unsupported expression in WHERE regex"))?
                .trim()
                .to_string();
            let value_position = start
                + self.src[start..operator_start]
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
            let value = parse_filter_projection(&label)?;
            let right_start = operator_start + operator_len;
            let right_end = filter_expr_end(self.src, right_start);
            let mut pattern_parser = Parser {
                src: &self.src[right_start..right_end],
                pos: 0,
            };
            let (_, pattern) = pattern_parser.parse_call_arg(true)?;
            pattern_parser.skip();
            if pattern_parser.peek().is_some() {
                return Err(Error::new("unsupported expression in WHERE regex pattern"));
            }
            let position =
                filter_regex_pattern_position(self.src, operator_start, right_end, &pattern);
            self.pos = right_end;
            return Ok(Filter::RegexExpression {
                label,
                value: Box::new(value),
                pattern,
                negated,
                position,
                operator_position: operator_start,
                value_position,
            });
        }
        if let Some((operator_start, negated, case_insensitive, operator_len)) =
            top_level_like_operator(self.src, start)
        {
            let label = std::str::from_utf8(&self.src[start..operator_start])
                .map_err(|_| Error::new("unsupported expression in WHERE LIKE"))?
                .trim()
                .to_string();
            let value_position = start
                + self.src[start..operator_start]
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
            let value = parse_filter_projection(&label)?;
            let right_start = operator_start + operator_len;
            let right_end = filter_expr_end(self.src, right_start);
            let mut pattern_parser = Parser {
                src: &self.src[right_start..right_end],
                pos: 0,
            };
            let (_, pattern) = pattern_parser.parse_call_arg(true)?;
            pattern_parser.skip();
            if pattern_parser.peek().is_some() {
                return Err(Error::new("unsupported expression in WHERE LIKE pattern"));
            }
            self.pos = right_end;
            return Ok(Filter::LikeExpression {
                label,
                value: Box::new(value),
                pattern,
                case_insensitive,
                negated,
                operator_position: operator_start,
                value_position,
            });
        }
        let (op_pos, _) = top_level_cmp(self.src, start)
            .ok_or_else(|| Error::new("unsupported: expected comparison after function"))?;
        let left_text = std::str::from_utf8(&self.src[start..op_pos])
            .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
            .trim();
        let left = FilterOperand::Projection(Box::new(parse_filter_projection(left_text)?));
        self.pos = op_pos;
        let op = self.parse_op()?;
        self.skip();
        let right_start = self.pos;
        let right_end = filter_expr_end(self.src, right_start);
        let right_text = std::str::from_utf8(&self.src[right_start..right_end])
            .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
            .trim();
        let first = right_text.as_bytes().first().copied();
        let is_literal = matches!(first, Some(b'\'' | b'-' | b'0'..=b'9'))
            || ["null", "true", "false"]
                .iter()
                .any(|keyword| right_text.eq_ignore_ascii_case(keyword));
        let right = if is_literal {
            let mut literal_parser = Parser {
                src: right_text.as_bytes(),
                pos: 0,
            };
            FilterOperand::Literal(literal_parser.parse_literal()?)
        } else {
            FilterOperand::Projection(Box::new(parse_filter_projection(right_text)?))
        };
        self.pos = right_end;
        Ok(Filter::CompareExpressions { left, op, right })
    }

    fn parse_filter_atom(&mut self) -> Result<Filter, Error> {
        self.skip();
        if self.eat_keyword("not") {
            return Ok(Filter::Not(Box::new(self.parse_filter_atom()?)));
        }
        if let Some(filter) = self.parse_extended_filter_comparison()? {
            return Ok(filter);
        }
        if self.eat(b'(') {
            let filter = self.parse_filter_or()?;
            if !self.eat(b')') {
                return Err(Error::new(format!(
                    "unsupported: expected ')' in WHERE clause near `{}`",
                    self.rest()
                )));
            }
            return Ok(filter);
        }
        if self.eat_keyword("true") {
            return Ok(Filter::BooleanLiteral(Some(true)));
        }
        if self.eat_keyword("false") {
            return Ok(Filter::BooleanLiteral(Some(false)));
        }
        if self.eat_keyword("null") {
            return Ok(Filter::BooleanLiteral(None));
        }
        if matches!(self.peek(), Some(b'\'' | b'-' | b'0'..=b'9')) {
            let position = self.pos;
            let value = self.parse_literal()?;
            return Ok(Filter::NonBooleanLiteral { value, position });
        }
        if self.filter_function_ahead() {
            return self.parse_filter_function_comparison();
        }
        let column = self.qualified_ident()?;
        let checkpoint = self.pos;
        let negated = self.eat_keyword("not");
        if self.eat_keyword("is") {
            let negated = self.eat_keyword("not");
            self.expect_keyword("null")?;
            return Ok(Filter::Compare {
                column,
                op: if negated { CmpOp::NotEq } else { CmpOp::Eq },
                literal: Scalar::Null,
            });
        }
        if self.eat_keyword("between") {
            let low = self.parse_literal()?;
            self.expect_keyword("and")?;
            let high = self.parse_literal()?;
            return Ok(Filter::Between {
                column,
                low,
                high,
                negated,
            });
        }
        let in_position = self.pos;
        if self.eat_keyword("in") {
            if !self.eat(b'(') {
                return Err(Error::new("unsupported: expected '(' after in"));
            }
            self.skip();
            if self.eat(b')') {
                return Err(Error::at("too few arguments for 'in'", in_position));
            }
            let mut items = Vec::new();
            let mut item_positions = Vec::new();
            loop {
                self.skip();
                item_positions.push(self.pos);
                items.push(self.parse_literal()?);
                self.skip();
                if self.eat(b',') {
                    continue;
                }
                if self.eat(b')') {
                    break;
                }
                return Err(Error::new("unsupported: expected ')' after in"));
            }
            return Ok(Filter::InList {
                column,
                items,
                item_positions,
                negated,
                position: in_position,
            });
        }
        if self.eat_keyword("like") {
            let (_, pattern) = self.parse_call_arg(true)?;
            return Ok(Filter::Like {
                column,
                pattern,
                case_insensitive: false,
                negated,
            });
        }
        if self.eat_keyword("ilike") {
            let (_, pattern) = self.parse_call_arg(true)?;
            return Ok(Filter::Like {
                column,
                pattern,
                case_insensitive: true,
                negated,
            });
        }
        if self.starts_not_regex() {
            let operator_start = self.pos;
            let expr = self.parse_not_regex(column.clone())?;
            let pattern = expr.args[1].clone();
            let position =
                filter_regex_pattern_position(self.src, operator_start, self.pos, &pattern);
            return Ok(Filter::Regex {
                column,
                pattern,
                negated: true,
                position,
            });
        }
        if self.starts_regex_infix() {
            let operator_start = self.pos;
            let expr = self.parse_regex_op(column.clone())?;
            let pattern = expr.args[1].clone();
            let position =
                filter_regex_pattern_position(self.src, operator_start, self.pos, &pattern);
            return Ok(Filter::Regex {
                column,
                pattern,
                negated: false,
                position,
            });
        }
        if negated {
            self.pos = checkpoint;
        }
        self.skip();
        if self.peek().is_none()
            || matches!(self.peek(), Some(b')' | b',' | b';'))
            || self.keyword_at(self.pos, "and").is_some()
            || self.keyword_at(self.pos, "or").is_some()
            || self.keyword_at(self.pos, "as").is_some()
            || self.keyword_at(self.pos, "from").is_some()
            || self.keyword_at(self.pos, "group").is_some()
            || self.keyword_at(self.pos, "order").is_some()
            || self.keyword_at(self.pos, "limit").is_some()
        {
            return Ok(Filter::BooleanColumn { column });
        }
        if self.starts_add() {
            let left =
                FilterOperand::Projection(Box::new(Proj::Add(self.parse_filter_add(column)?)));
            let op = self.parse_op()?;
            self.skip();
            if self.filter_function_ahead() {
                let right_end = filter_expr_end(self.src, self.pos);
                let right_text = std::str::from_utf8(&self.src[self.pos..right_end])
                    .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
                    .trim();
                let right =
                    FilterOperand::Projection(Box::new(parse_filter_projection(right_text)?));
                self.pos = right_end;
                return Ok(Filter::CompareExpressions { left, op, right });
            }
            let right_column = matches!(self.peek(), Some(byte) if byte.is_ascii_alphabetic() || byte == b'_')
                && self.keyword_at(self.pos, "true").is_none()
                && self.keyword_at(self.pos, "false").is_none()
                && self.keyword_at(self.pos, "null").is_none();
            let right = if right_column {
                let name = self.qualified_ident()?;
                if self.starts_add() {
                    FilterOperand::Projection(Box::new(Proj::Add(self.parse_filter_add(name)?)))
                } else {
                    FilterOperand::Projection(Box::new(Proj::Column(name)))
                }
            } else {
                FilterOperand::Literal(self.parse_literal()?)
            };
            return Ok(Filter::CompareExpressions { left, op, right });
        }
        let op = self.parse_op()?;
        self.skip();
        let right_start = self.pos;
        if self.filter_function_ahead() {
            let right_end = filter_expr_end(self.src, right_start);
            let right_text = std::str::from_utf8(&self.src[right_start..right_end])
                .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
                .trim();
            let left = FilterOperand::Projection(Box::new(Proj::Column(column)));
            let right = FilterOperand::Projection(Box::new(parse_filter_projection(right_text)?));
            self.pos = right_end;
            return Ok(Filter::CompareExpressions { left, op, right });
        }
        let right_is_column = matches!(self.peek(), Some(byte) if byte.is_ascii_alphabetic() || byte == b'_')
            && self.keyword_at(self.pos, "true").is_none()
            && self.keyword_at(self.pos, "false").is_none()
            && self.keyword_at(self.pos, "null").is_none();
        if right_is_column {
            let right = self.qualified_ident()?;
            if self.starts_add() {
                let left = FilterOperand::Projection(Box::new(Proj::Column(column)));
                let right =
                    FilterOperand::Projection(Box::new(Proj::Add(self.parse_filter_add(right)?)));
                return Ok(Filter::CompareExpressions { left, op, right });
            }
            self.skip();
            if self.peek().is_none()
                || matches!(self.peek(), Some(b')' | b';'))
                || self.keyword_at(self.pos, "and").is_some()
                || self.keyword_at(self.pos, "or").is_some()
                || self.keyword_at(self.pos, "order").is_some()
                || self.keyword_at(self.pos, "limit").is_some()
            {
                return Ok(Filter::CompareColumns {
                    left: column,
                    op,
                    right,
                });
            }
            self.pos = right_start;
        }
        let literal = self.parse_literal()?;
        Ok(Filter::Compare {
            column,
            op,
            literal,
        })
    }

    fn parse_extended_filter_comparison(&mut self) -> Result<Option<Filter>, Error> {
        let start = self.pos;
        if let Some((operator_start, in_position, negated, items_start)) =
            top_level_in_operator(self.src, start)
        {
            let left_text = std::str::from_utf8(&self.src[start..operator_start])
                .map_err(|_| Error::new("unsupported expression in WHERE IN"))?
                .trim();
            if needs_extended_filter_expr(left_text) || unwrapped_filter_expr(left_text).is_some() {
                let left_leading = self.src[start..operator_start]
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
                let value = parse_filter_operand(left_text, start + left_leading)?;
                let items_end = filter_expr_end(self.src, items_start);
                let mut parser = Parser {
                    src: &self.src[items_start..items_end],
                    pos: 0,
                };
                if !parser.eat(b'(') {
                    return Err(Error::at("unsupported: expected '(' after in", in_position));
                }
                parser.skip();
                let mut items = Vec::new();
                let mut item_positions = Vec::new();
                if parser.eat(b')') {
                    return Err(Error::at("too few arguments for 'in'", in_position));
                }
                loop {
                    parser.skip();
                    item_positions.push(items_start + parser.pos);
                    items.push(parser.parse_literal()?);
                    parser.skip();
                    if parser.eat(b',') {
                        continue;
                    }
                    if parser.eat(b')') {
                        break;
                    }
                    return Err(Error::new("unsupported: expected ')' after in list"));
                }
                parser.skip();
                if parser.peek().is_some() {
                    return Err(Error::new("unsupported: trailing input after in list"));
                }
                self.pos = items_end;
                return Ok(Some(Filter::InListExpression {
                    value,
                    items,
                    item_positions,
                    negated,
                    position: in_position,
                }));
            }
        }
        if let Some((null_pos, negated, end)) = top_level_is_null(self.src, start) {
            let left_text = std::str::from_utf8(&self.src[start..null_pos])
                .map_err(|_| Error::new("unsupported expression in WHERE NULL check"))?
                .trim();
            if needs_extended_filter_expr(left_text) || unwrapped_filter_expr(left_text).is_some() {
                let left_leading = self.src[start..null_pos]
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
                let left = parse_filter_operand(left_text, start + left_leading)?;
                self.pos = end;
                return Ok(Some(Filter::CompareExpressions {
                    left,
                    op: if negated { CmpOp::NotEq } else { CmpOp::Eq },
                    right: FilterOperand::Literal(Scalar::Null),
                }));
            }
        }
        let Some((cmp_pos, op)) = top_level_cmp(self.src, start) else {
            return Ok(None);
        };
        if has_top_level_boolean_operator(&self.src[start..cmp_pos]) {
            return Ok(None);
        }
        let left_text = std::str::from_utf8(&self.src[start..cmp_pos])
            .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
            .trim();
        let cmp_width = match op {
            CmpOp::NotEq | CmpOp::LtEq | CmpOp::GtEq => 2,
            _ => 1,
        };
        let right_start = cmp_pos + cmp_width;
        let right_end = filter_expr_end(self.src, right_start);
        let right_text = std::str::from_utf8(&self.src[right_start..right_end])
            .map_err(|_| Error::new("unsupported expression in WHERE comparison"))?
            .trim();
        if !needs_extended_filter_expr(left_text) && !needs_extended_filter_expr(right_text) {
            return Ok(None);
        }
        let left_leading = self.src[start..cmp_pos]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        let right_leading = self.src[right_start..right_end]
            .iter()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        let left = parse_filter_operand(left_text, start + left_leading)?;
        let right = parse_filter_operand(right_text, right_start + right_leading)?;
        self.pos = cmp_pos;
        let op = self.parse_op()?;
        self.pos = right_end;
        Ok(Some(Filter::CompareExpressions { left, op, right }))
    }

    fn parse_filter_add(&mut self, name: String) -> Result<AddExpr, Error> {
        self.skip();
        let op = if self.eat(b'-') {
            ArithOp::Sub
        } else if self.eat(b'+') {
            ArithOp::Add
        } else if self.eat(b'*') {
            ArithOp::Mul
        } else if self.eat(b'/') {
            ArithOp::Div
        } else if self.eat(b'%') {
            ArithOp::Mod
        } else {
            return Err(Error::new("unsupported arithmetic expression in WHERE"));
        };
        self.skip();
        let (literal, other) = if self.starts_add_column() {
            (Scalar::Null, Some(self.qualified_ident()?))
        } else {
            (self.parse_literal()?, None)
        };
        let label = match &other {
            Some(column) => format!("{name} {} {column}", op.spelling()),
            None => format!("{name} {} {}", op.spelling(), eq_literal_label(&literal)),
        };
        Ok(AddExpr {
            label,
            value: name,
            literal,
            other,
            op,
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
        if matches!(self.peek(), Some(b'e' | b'E')) {
            float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
                return Err(Error::new("unsupported: invalid scientific number"));
            }
            while matches!(self.peek(), Some(byte) if byte.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        if float {
            let value: f64 = text
                .parse()
                .map_err(|_| Error::new(format!("invalid number `{text}`")))?;
            let mantissa = text.split(['e', 'E']).next().unwrap_or(text);
            if !value.is_finite()
                || (value == 0.0 && mantissa.bytes().any(|byte| matches!(byte, b'1'..=b'9')))
            {
                return Err(Error::new(format!("invalid number `{text}`")));
            }
            Ok(Scalar::Double(value))
        } else {
            match text.parse::<i64>() {
                Ok(i64::MIN) | Err(_) => text
                    .parse::<f64>()
                    .map(Scalar::Double)
                    .map_err(|_| Error::new(format!("invalid number `{text}`"))),
                Ok(value) => Ok(Scalar::Long(value)),
            }
        }
    }
}

fn filter_regex_pattern_position(
    source: &[u8],
    operator_start: usize,
    end: usize,
    pattern: &ConcatArg,
) -> usize {
    let ConcatArg::Literal(Scalar::Text(pattern)) = pattern else {
        return operator_start;
    };
    if pattern.is_empty() {
        return operator_start;
    }
    let end = end.min(source.len());
    let start = operator_start.min(end);
    source[start..end]
        .windows(pattern.len())
        .position(|window| window == pattern.as_bytes())
        .map_or(operator_start, |offset| start + offset)
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
                position: 51,
            }]
        );
        let descending = parse_query("SELECT id FROM sensors ORDER BY location DESC, id").unwrap();
        assert_eq!(
            descending.order_by,
            vec![
                OrderBy {
                    column: "location".to_string(),
                    descending: true,
                    position: 32,
                },
                OrderBy {
                    column: "id".to_string(),
                    descending: false,
                    position: 47,
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
