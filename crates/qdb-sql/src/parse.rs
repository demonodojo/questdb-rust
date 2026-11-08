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

/// `regexp_replace(value, pattern, replacement)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexpReplaceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
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
    pub args: Vec<ConcatArg>,
    pub negated: bool,
}

/// `like(text, pattern)` is a case-sensitive `LIKE` and returns a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LikeExpr {
    pub label: String,
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
    /// `true` for `column::type`. `false` for `cast(column AS type)`.
    pub infix: bool,
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

/// `nanos(timestamp)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NanosExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_utc(timestamp, timezone)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToUtcExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_timezone(timestamp, timezone)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToTimezoneExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_pg_date(string)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToPgDateExpr {
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

/// `make_geohash(lon, lat, bits)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakeGeohashExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `to_uuid(lo, hi)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToUuidExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `json_extract(doc, path)` and the typed `json_extract(doc, path, type)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonExtractExpr {
    pub label: String,
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
    pub args: Vec<ConcatArg>,
    pub base64: Option<FromBase64Expr>,
}

/// `typeof(value)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeofExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub geohash: Option<MakeGeohashExpr>,
    pub uuid: Option<ToUuidExpr>,
    pub json: Option<JsonExtractExpr>,
    pub to_char: Option<ToCharExpr>,
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
    pub args: Vec<ConcatArg>,
    pub negated: bool,
}

/// `value in (list)` reports whether `value` equals one listed constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InListExpr {
    pub label: String,
    pub value: String,
    pub args: Vec<ConcatArg>,
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

/// `value = other` compares a column with one constant or another column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EqExpr {
    pub label: String,
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
    pub args: Vec<ConcatArg>,
}

/// `current_setting(name)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentSettingExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `pg_get_partkeydef(oid)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetPartKeyDefExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `pg_catalog.pg_get_userbyid(id)` returns the fixed role name `public`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetUserByIdExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_catalog.pg_table_is_visible(oid)` returns the fixed boolean `true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgTableIsVisibleExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_catalog.age(xid)` returns the fixed long `0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgAgeExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
    pub qualified: bool,
}

/// `pg_get_expr(expr, relation [, pretty])`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgGetExprExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `netmask(text)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetmaskExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `geo_distance_meters(lat1, lon1, lat2, lon2)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoDistanceExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `geo_within_radius_latlon(lat, lon, center_lat, center_lon, radius)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeoWithinExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `within_radius(x, y, center_x, center_y, radius)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinRadiusExpr {
    pub label: String,
    pub args: Vec<ConcatArg>,
}

/// `within_box(x, y, min_x, min_y, max_x, max_y)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithinBoxExpr {
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
    group_by: &mut Option<String>,
    alias: &str,
) {
    match columns {
        SelectList::Projection(items) => {
            for item in items {
                if let Proj::Column(name) = item {
                    *name = strip_table_alias(name, alias);
                }
            }
        }
        SelectList::Columns(names) => {
            for name in names {
                *name = strip_table_alias(name, alias);
            }
        }
        SelectList::Grouped(grouped) => {
            grouped.key = strip_table_alias(&grouped.key, alias);
        }
        SelectList::Star | SelectList::Aggregate(_) => {}
    }
    if let Some(filter) = filter {
        filter.column = strip_table_alias(&filter.column, alias);
    }
    for order in order_by.iter_mut() {
        order.column = strip_table_alias(&order.column, alias);
    }
    if let Some(name) = group_by {
        *name = strip_table_alias(name, alias);
    }
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
        let mut column = parser.ident()?;
        parser.skip();
        if parser.eat(b'.') {
            column.push('.');
            column.push_str(&parser.ident()?);
            parser.skip();
        }
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

    fn ident(&mut self) -> Result<String, Error> {
        self.ident_raw()
            .ok_or_else(|| Error::new(format!("expected identifier near `{}`", self.rest())))
    }

    fn parse_select_list(&mut self) -> Result<SelectList, Error> {
        self.skip();
        if self.eat(b'*') {
            return Ok(SelectList::Star);
        }
        if !self.starts_value() && !self.starts_unary_minus() && !self.starts_tilde() {
            return Err(Error::new(
                "unsupported: SELECT list must name columns or `*`",
            ));
        }
        let mut pieces = Vec::new();
        loop {
            self.skip();
            if self.starts_named_call("generate_series") {
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
                } else {
                    pieces.push(SelectPiece::Column(name));
                }
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
        Ok(CurrentSettingExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(PgGetUserByIdExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(PgTableIsVisibleExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(PgAgeExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(PgGetPartKeyDefExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(PgGetExprExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(NetmaskExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(GeoDistanceExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(GeoWithinExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(WithinRadiusExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(WithinBoxExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(MakeGeohashExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(ToUuidExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(JsonExtractExpr {
            label: format!("{name}({})", labels.join(", ")),
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
            return Ok(ToCharExpr {
                label: format!("{name}({})", inner.label),
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
        Ok(ToCharExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(FormatTypeExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(BetweenExpr {
            label: format!("{name} between ({low_label}, {high_label})"),
            args: vec![ConcatArg::Column(name), low, high],
            negated: false,
        })
    }

    fn parse_between(&mut self) -> Result<BetweenExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        Ok(BetweenExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(InListExpr {
            label: format!("{name} in ({})", labels.join(", ")),
            value: name,
            args,
        })
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
        let spelling = self.ident_spelling()?;
        let right = match spelling.to_ascii_lowercase().as_str() {
            "null" => AndRight::Literal(Scalar::Null),
            "true" => AndRight::Literal(Scalar::Boolean(true)),
            "false" => AndRight::Literal(Scalar::Boolean(false)),
            _ => AndRight::Column(spelling),
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
        let name = self.ident_spelling()?;
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
        let name = self.ident_spelling()?;
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
        matches!(
            self.src.get(pos),
            Some(byte) if byte.is_ascii_alphabetic() || *byte == b'_'
        )
    }

    fn parse_not_prefix(&mut self) -> Result<NotExpr, Error> {
        let _keyword = self.ident_spelling()?;
        let value = self.ident_spelling()?;
        Ok(NotExpr {
            label: format!("not({value})"),
            args: vec![ConcatArg::Column(value)],
        })
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
                args: Vec::new(),
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
                args: Vec::new(),
                geohash: None,
                uuid: Some(inner),
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
                args: Vec::new(),
                geohash: None,
                uuid: None,
                json: Some(inner),
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
                args: vec![ConcatArg::Literal(Scalar::Boolean(false))],
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
                args: vec![ConcatArg::Literal(Scalar::Text("database".to_string()))],
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
                args: vec![ConcatArg::Literal(Scalar::Text("schema".to_string()))],
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
                args: vec![ConcatArg::Literal(Scalar::Text("version".to_string()))],
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
                args: inner.args,
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
                args: inner.args,
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
                args: inner.args,
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
                args: inner.args,
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
                args: inner.args,
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
                args: Vec::new(),
                geohash: None,
                uuid: None,
                json: None,
                to_char: Some(inner),
            });
        }
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new("exactly one argument expected"));
        }
        Ok(TypeofExpr {
            label: format!("{name}({})", labels.join(", ")),
            args,
            geohash: None,
            uuid: None,
            json: None,
            to_char: None,
        })
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
        Ok(ToPgDateExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(ToTimezoneExpr {
            label: format!("{name}({})", labels.join(", ")),
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
        Ok(ToUtcExpr {
            label: format!("{name}({})", labels.join(", ")),
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
            args: vec![ConcatArg::Column(name), pattern],
            negated: false,
            regex: false,
        })
    }

    fn parse_like(&mut self) -> Result<LikeExpr, Error> {
        let _name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args("like", true)?;
        if args.is_empty() {
            return Err(Error::new(
                "too few arguments for 'like' [found=0,expected=2]",
            ));
        }
        let label = if labels.len() == 2 {
            format!("{} like {}", labels[0], labels[1])
        } else {
            format!("like({})", labels.join(", "))
        };
        Ok(LikeExpr {
            label,
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
        Ok(IlikeExpr {
            label: format!("{name}({})", labels.join(", ")),
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

    fn starts_regexp_replace_call(&self) -> bool {
        self.starts_named_call("regexp_replace")
    }

    fn parse_regexp_replace(&mut self) -> Result<RegexpReplaceExpr, Error> {
        let name = self.ident_spelling()?;
        let (labels, args) = self.parse_call_args(&name, true)?;
        if args.is_empty() {
            return Err(Error::new(
                "function `regexp_replace` requires arguments".to_string(),
            ));
        }
        Ok(RegexpReplaceExpr {
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
            } else if spelling.eq_ignore_ascii_case("true") {
                ConcatArg::Literal(Scalar::Boolean(true))
            } else if spelling.eq_ignore_ascii_case("false") {
                ConcatArg::Literal(Scalar::Boolean(false))
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
            return self.finish_haversine(name, column);
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
            pair = Some(self.ident_spelling()?);
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
                pair = Some(self.ident_spelling()?);
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
        let column = self.qualified_ident()?;
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
