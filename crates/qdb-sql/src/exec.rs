use crate::parse::{
    date_from_literal, parse_query, timestamp_from_literal, AbsExpr, AggFunc, AggregateExpr,
    CeilExpr, CmpOp, ConcatArg, ExpExpr, Filter, FloorExpr, GroupedSelect, LengthExpr, LnExpr,
    LogExpr, LowerExpr, PadExpr, PiExpr, PowerExpr, Proj, Query, QuoteIdentExpr, ReplaceExpr,
    RoundExpr, SelectList, SideExpr, SignExpr, SizePrettyExpr, SplitPartExpr, SqrtExpr,
    StartsWithExpr, StrPosExpr, SubstrExpr, TrimEnd, TrimExpr, UpperExpr,
};
use qdb_storage::{open_table, Table};
use qdb_types::{format_f64, format_timestamp_micros, Error, Scalar};
use std::cmp::Ordering;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Scalar>>,
}

pub fn execute(data_dir: &Path, sql: &str) -> Result<QueryResult, Error> {
    let query = parse_query(sql)?;
    let table = open_table(data_dir, &query.table)?;
    execute_on(&table, &query)
}

pub fn execute_on(table: &Table, query: &Query) -> Result<QueryResult, Error> {
    if !table.name.eq_ignore_ascii_case(&query.table) {
        return Err(Error::new(format!(
            "query table `{}` does not match opened table `{}`",
            query.table, table.name
        )));
    }
    if query.group_by.is_some()
        && !matches!(
            &query.columns,
            SelectList::Aggregate(_) | SelectList::Grouped(_)
        )
    {
        return Err(Error::new("unsupported: GROUP BY without an aggregate"));
    }
    let projection = match &query.columns {
        SelectList::Aggregate(aggregate) if query.group_by.is_none() => {
            return execute_aggregate(table, query, aggregate);
        }
        SelectList::Aggregate(aggregate) => return execute_group(table, query, None, aggregate),
        SelectList::Grouped(grouped) => {
            return execute_group(table, query, Some(grouped), &grouped.aggregate);
        }
        SelectList::Projection(items) => return execute_projection(table, query, items),
        SelectList::Star => (0..table.meta.columns.len()).collect::<Vec<_>>(),
        SelectList::Columns(names) => {
            let mut indexes = Vec::with_capacity(names.len());
            for name in names {
                let index = table
                    .column_index(name)
                    .ok_or_else(|| Error::new(format!("unknown column `{name}`")))?;
                indexes.push(index);
            }
            indexes
        }
    };
    let mut row_ids = Vec::new();
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        row_ids.push(row);
    }
    if !query.order_by.is_empty() {
        let mut keys = Vec::with_capacity(query.order_by.len());
        for order in &query.order_by {
            let index = table
                .column_index(&order.column)
                .ok_or_else(|| Error::new(format!("unknown ORDER BY column `{}`", order.column)))?;
            keys.push((index, table.meta.columns[index].tag, order.descending));
        }
        row_ids.sort_by(|a, b| {
            for (index, tag, descending) in &keys {
                let mut ordering =
                    compare_asc(&table.columns[*index][*a], &table.columns[*index][*b], *tag);
                if *descending {
                    ordering = ordering.reverse();
                }
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            Ordering::Equal
        });
    }
    if let Some(limit) = query.limit {
        if limit.lo < 0 && limit.hi > 0 && limit_follows_timestamp_order(table, query)? {
            return Err(Error::new("LIMIT <negative>, <positive> is not allowed"));
        }
        apply_limit(&mut row_ids, limit.lo, limit.hi);
    }
    let columns = projection
        .iter()
        .map(|index| table.meta.columns[*index].name.clone())
        .collect();
    let mut rows = Vec::with_capacity(row_ids.len());
    for row in row_ids {
        let values = projection
            .iter()
            .map(|index| table.columns[*index][row].clone())
            .collect();
        rows.push(values);
    }
    Ok(QueryResult { columns, rows })
}

fn execute_projection(table: &Table, query: &Query, items: &[Proj]) -> Result<QueryResult, Error> {
    let resolved = items
        .iter()
        .map(|item| resolve_proj(table, item))
        .collect::<Result<Vec<_>, _>>()?;
    let mut row_ids = Vec::new();
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        row_ids.push(row);
    }
    for row in &row_ids {
        for item in &resolved {
            reject_negative_substring(table, *row, item)?;
        }
    }
    if !query.order_by.is_empty() {
        let mut keys = Vec::with_capacity(query.order_by.len());
        for order in &query.order_by {
            if let Some(index) = resolved.iter().position(|item| {
                item.expr_label()
                    .is_some_and(|label| label.eq_ignore_ascii_case(&order.column))
            }) {
                keys.push(OrderKey::Expr(index, order.descending));
                continue;
            }
            let index = table
                .column_index(&order.column)
                .ok_or_else(|| Error::new(format!("unknown ORDER BY column `{}`", order.column)))?;
            keys.push(OrderKey::Column(
                index,
                table.meta.columns[index].tag,
                order.descending,
            ));
        }
        row_ids.sort_by(|a, b| {
            for key in &keys {
                let mut ordering = match key {
                    OrderKey::Column(index, tag, _) => {
                        compare_asc(&table.columns[*index][*a], &table.columns[*index][*b], *tag)
                    }
                    OrderKey::Expr(index, _) => compare_asc(
                        &eval_resolved(table, *a, &resolved[*index]),
                        &eval_resolved(table, *b, &resolved[*index]),
                        resolved[*index].expr_tag(),
                    ),
                };
                if key.descending() {
                    ordering = ordering.reverse();
                }
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            Ordering::Equal
        });
    }
    if let Some(limit) = query.limit {
        if limit.lo < 0 && limit.hi > 0 && limit_follows_timestamp_order(table, query)? {
            return Err(Error::new("LIMIT <negative>, <positive> is not allowed"));
        }
        apply_limit(&mut row_ids, limit.lo, limit.hi);
    }
    let columns = resolved
        .iter()
        .map(|item| match item {
            ResolvedProj::Column { index, .. } => table.meta.columns[*index].name.clone(),
            ResolvedProj::Concat { label, .. }
            | ResolvedProj::Length { label, .. }
            | ResolvedProj::Lower { label, .. }
            | ResolvedProj::Upper { label, .. }
            | ResolvedProj::Side { label, .. }
            | ResolvedProj::Substr { label, .. }
            | ResolvedProj::Trim { label, .. }
            | ResolvedProj::Replace { label, .. }
            | ResolvedProj::StartsWith { label, .. }
            | ResolvedProj::StrPos { label, .. }
            | ResolvedProj::SplitPart { label, .. }
            | ResolvedProj::Pad { label, .. }
            | ResolvedProj::QuoteIdent { label, .. }
            | ResolvedProj::SizePretty { label, .. }
            | ResolvedProj::Abs { label, .. }
            | ResolvedProj::Round { label, .. }
            | ResolvedProj::Floor { label, .. }
            | ResolvedProj::Ceil { label, .. }
            | ResolvedProj::Sign { label, .. }
            | ResolvedProj::Sqrt { label, .. }
            | ResolvedProj::Power { label, .. }
            | ResolvedProj::Ln { label, .. }
            | ResolvedProj::Log { label, .. }
            | ResolvedProj::Exp { label, .. }
            | ResolvedProj::Pi { label, .. } => label.clone(),
        })
        .collect();
    let rows = row_ids
        .into_iter()
        .map(|row| {
            resolved
                .iter()
                .map(|item| eval_resolved(table, row, item))
                .collect()
        })
        .collect();
    Ok(QueryResult { columns, rows })
}

enum OrderKey {
    Column(usize, i32, bool),
    Expr(usize, bool),
}

impl OrderKey {
    fn descending(&self) -> bool {
        match self {
            Self::Column(_, _, descending) | Self::Expr(_, descending) => *descending,
        }
    }
}

enum ResolvedProj {
    Column {
        index: usize,
    },
    Concat {
        label: String,
        args: Vec<ResolvedArg>,
    },
    Length {
        label: String,
        arg: ResolvedArg,
        bytes: bool,
    },
    Lower {
        label: String,
        arg: ResolvedArg,
    },
    Upper {
        label: String,
        arg: ResolvedArg,
    },
    Side {
        label: String,
        from_right: bool,
        text: ResolvedArg,
        count: CountArg,
    },
    Substr {
        label: String,
        mode: SubstrMode,
    },
    Trim {
        label: String,
        arg: ResolvedArg,
        end: TrimEnd,
    },
    Replace {
        label: String,
        mode: ReplaceMode,
    },
    StartsWith {
        label: String,
        value: ResolvedArg,
        prefix: ResolvedArg,
    },
    StrPos {
        label: String,
        value: ResolvedArg,
        needle: ResolvedArg,
        needle_null: bool,
    },
    SplitPart {
        label: String,
        value: ResolvedArg,
        delimiter: ResolvedArg,
        index: Option<i32>,
    },
    Pad {
        label: String,
        value: ResolvedArg,
        width: CountArg,
        fill: Option<ResolvedArg>,
        from_right: bool,
    },
    QuoteIdent {
        label: String,
        arg: ResolvedArg,
    },
    SizePretty {
        label: String,
        size: CountArg,
    },
    Abs {
        label: String,
        input: AbsInput,
        result_tag: i32,
    },
    Round {
        label: String,
        input: RoundInput,
        scale: RoundScale,
    },
    Floor {
        label: String,
        input: FloorInput,
        wide: bool,
    },
    Ceil {
        label: String,
        input: FloorInput,
        wide: bool,
    },
    Sign {
        label: String,
        input: SignInput,
        result_tag: i32,
    },
    Sqrt {
        label: String,
        input: SqrtInput,
    },
    Power {
        label: String,
        base: SqrtInput,
        exponent: SqrtInput,
    },
    Ln {
        label: String,
        input: SqrtInput,
    },
    Log {
        label: String,
        input: SqrtInput,
    },
    Exp {
        label: String,
        input: SqrtInput,
    },
    Pi {
        label: String,
    },
}

enum SqrtInput {
    Null,
    Long(i64),
    Double(f64),
    Column { index: usize },
}

enum SignInput {
    Byte(i8),
    Int(i32),
    Long(i64),
    Double(f64),
    Column { index: usize },
}

enum FloorInput {
    Null,
    Long(i64),
    Double(f64),
    Column { index: usize },
}

enum RoundInput {
    Null,
    Long(i64),
    Double(f64),
    Column { index: usize },
}

enum RoundScale {
    Zero,
    Fixed(i32),
    Column(usize),
    Null,
}

enum AbsInput {
    Null,
    Long(i64),
    Double(f64),
    Column { index: usize },
}

enum ReplaceMode {
    Null,
    Empty,
    Value(ResolvedArg),
    Run {
        value: ResolvedArg,
        old: ResolvedArg,
        new: ResolvedArg,
    },
}

enum SubstrMode {
    Null,
    Empty,
    Slice {
        text: ResolvedArg,
        start: CountArg,
        len: CountArg,
    },
}

impl ResolvedProj {
    fn expr_label(&self) -> Option<&str> {
        match self {
            Self::Concat { label, .. }
            | Self::Length { label, .. }
            | Self::Lower { label, .. }
            | Self::Upper { label, .. }
            | Self::Side { label, .. }
            | Self::Substr { label, .. }
            | Self::Trim { label, .. }
            | Self::Replace { label, .. }
            | Self::StartsWith { label, .. }
            | Self::StrPos { label, .. }
            | Self::SplitPart { label, .. }
            | Self::Pad { label, .. }
            | Self::QuoteIdent { label, .. }
            | Self::SizePretty { label, .. }
            | Self::Abs { label, .. }
            | Self::Round { label, .. }
            | Self::Floor { label, .. }
            | Self::Ceil { label, .. }
            | Self::Sign { label, .. }
            | Self::Sqrt { label, .. }
            | Self::Power { label, .. }
            | Self::Ln { label, .. }
            | Self::Log { label, .. }
            | Self::Exp { label, .. }
            | Self::Pi { label, .. } => Some(label),
            Self::Column { .. } => None,
        }
    }

    fn expr_tag(&self) -> i32 {
        match self {
            Self::Length { .. } | Self::StrPos { .. } => qdb_types::TAG_INT,
            Self::Abs { result_tag, .. } => *result_tag,
            Self::Round { .. } => qdb_types::TAG_DOUBLE,
            Self::Sign { result_tag, .. } => *result_tag,
            Self::Sqrt { .. }
            | Self::Power { .. }
            | Self::Ln { .. }
            | Self::Log { .. }
            | Self::Exp { .. }
            | Self::Pi { .. } => qdb_types::TAG_DOUBLE,
            Self::Floor { wide, .. } | Self::Ceil { wide, .. } => {
                if *wide {
                    qdb_types::TAG_DOUBLE
                } else {
                    qdb_types::TAG_FLOAT
                }
            }
            Self::StartsWith { .. } => qdb_types::TAG_BOOLEAN,
            _ => qdb_types::TAG_STRING,
        }
    }
}

enum ResolvedArg {
    Text(String),
    Null,
    Column { index: usize, tag: i32 },
}

enum CountArg {
    Null,
    Fixed(i64),
    ColumnInt { index: usize },
    ColumnSymbol { index: usize, keys: Vec<Scalar> },
}

fn resolve_proj(table: &Table, item: &Proj) -> Result<ResolvedProj, Error> {
    match item {
        Proj::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("unknown column `{name}`")))?;
            Ok(ResolvedProj::Column { index })
        }
        Proj::Concat(expr) => Ok(ResolvedProj::Concat {
            label: expr.label.clone(),
            args: expr
                .args
                .iter()
                .map(|arg| resolve_concat_arg(table, arg))
                .collect::<Result<Vec<_>, _>>()?,
        }),
        Proj::Length(expr) => resolve_length(table, expr),
        Proj::Lower(expr) => resolve_lower(table, expr),
        Proj::Upper(expr) => resolve_upper(table, expr),
        Proj::Side(expr) => resolve_side(table, expr),
        Proj::Substr(expr) => resolve_substr(table, expr),
        Proj::Trim(expr) => resolve_trim(table, expr),
        Proj::Replace(expr) => resolve_replace(table, expr),
        Proj::StartsWith(expr) => resolve_starts_with(table, expr),
        Proj::StrPos(expr) => resolve_strpos(table, expr),
        Proj::SplitPart(expr) => resolve_split_part(table, expr),
        Proj::Pad(expr) => resolve_pad(table, expr),
        Proj::QuoteIdent(expr) => resolve_quote_ident(table, expr),
        Proj::SizePretty(expr) => resolve_size_pretty(table, expr),
        Proj::Abs(expr) => resolve_abs(table, expr),
        Proj::Round(expr) => resolve_round(table, expr),
        Proj::Floor(expr) => resolve_floor(table, expr),
        Proj::Ceil(expr) => resolve_ceil(table, expr),
        Proj::Sign(expr) => resolve_sign(table, expr),
        Proj::Sqrt(expr) => resolve_sqrt(table, expr),
        Proj::Power(expr) => resolve_power(table, expr),
        Proj::Ln(expr) => resolve_ln(table, expr),
        Proj::Log(expr) => resolve_log(table, expr),
        Proj::Exp(expr) => resolve_exp(table, expr),
        Proj::Pi(expr) => resolve_pi(table, expr),
    }
}

fn resolve_pi(table: &Table, expr: &PiExpr) -> Result<ResolvedProj, Error> {
    for arg in &expr.args {
        if let ConcatArg::Column(name) = arg {
            if table.column_index(name).is_none() {
                return Err(Error::new(format!("Invalid column: {name}")));
            }
        }
    }
    if !expr.args.is_empty() {
        return Err(Error::new(format!(
            "wrong number of arguments for function `pi`; expected: 0, provided: {}",
            expr.args.len()
        )));
    }
    Ok(ResolvedProj::Pi {
        label: expr.label.clone(),
    })
}

fn resolve_exp(table: &Table, expr: &ExpExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `exp`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    let input = exp_input(table, &expr.args[0])?;
    Ok(ResolvedProj::Exp {
        label: expr.label.clone(),
        input,
    })
}

fn exp_input(table: &Table, arg: &ConcatArg) -> Result<SqrtInput, Error> {
    match log_input(table, arg) {
        Ok(input) => Ok(input),
        Err(err) => Err(Error::new(err.message.replace("`log`", "`exp`"))),
    }
}

fn resolve_log(table: &Table, expr: &LogExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `log`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    let input = log_input(table, &expr.args[0])?;
    Ok(ResolvedProj::Log {
        label: expr.label.clone(),
        input,
    })
}

fn log_input(table: &Table, arg: &ConcatArg) -> Result<SqrtInput, Error> {
    match ln_input(table, arg) {
        Ok(input) => Ok(input),
        Err(err) => Err(Error::new(err.message.replace("`ln`", "`log`"))),
    }
}

fn resolve_ln(table: &Table, expr: &LnExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `ln`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    let input = ln_input(table, &expr.args[0])?;
    Ok(ResolvedProj::Ln {
        label: expr.label.clone(),
        input,
    })
}

fn ln_input(table: &Table, arg: &ConcatArg) -> Result<SqrtInput, Error> {
    match power_input(table, arg) {
        Ok(input) => Ok(input),
        Err(err) if err.message.starts_with("expression type mismatch") => {
            Err(match concat_arg_type(table, arg) {
                Ok(actual) => Error::new(format!(
                "argument type mismatch for function `ln` at #1 expected: DOUBLE, actual: {actual}"
            )),
                Err(column) => column,
            })
        }
        Err(err) => Err(err),
    }
}

fn resolve_power(table: &Table, expr: &PowerExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 2 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `power`; expected: 2, provided: {}",
            expr.args.len()
        )));
    }
    let base = power_input(table, &expr.args[0])?;
    let exponent = power_input(table, &expr.args[1])?;
    Ok(ResolvedProj::Power {
        label: expr.label.clone(),
        base,
        exponent,
    })
}

fn power_input(table: &Table, arg: &ConcatArg) -> Result<SqrtInput, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok(SqrtInput::Null),
        ConcatArg::Literal(Scalar::Long(number)) => Ok(SqrtInput::Long(*number)),
        ConcatArg::Literal(Scalar::Double(number)) => Ok(SqrtInput::Double(*number)),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
                    | qdb_types::TAG_FLOAT
                    | qdb_types::TAG_DOUBLE
            ) {
                Ok(SqrtInput::Column { index })
            } else {
                Err(power_type_error(table, arg))
            }
        }
        ConcatArg::Literal(_) => Err(power_type_error(table, arg)),
    }
}

fn power_type_error(table: &Table, arg: &ConcatArg) -> Error {
    match concat_arg_type(table, arg) {
        Ok(actual) => Error::new(format!(
            "expression type mismatch, expected: DOUBLE, actual: {actual}"
        )),
        Err(err) => err,
    }
}

fn resolve_sqrt(table: &Table, expr: &SqrtExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `sqrt`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    let input = sqrt_input(table, &expr.args[0])?;
    Ok(ResolvedProj::Sqrt {
        label: expr.label.clone(),
        input,
    })
}

fn sqrt_input(table: &Table, arg: &ConcatArg) -> Result<SqrtInput, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok(SqrtInput::Null),
        ConcatArg::Literal(Scalar::Long(number)) => Ok(SqrtInput::Long(*number)),
        ConcatArg::Literal(Scalar::Double(number)) => Ok(SqrtInput::Double(*number)),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
                    | qdb_types::TAG_FLOAT
                    | qdb_types::TAG_DOUBLE
            ) {
                Ok(SqrtInput::Column { index })
            } else {
                Err(sqrt_type_error(table, arg))
            }
        }
        ConcatArg::Literal(_) => Err(sqrt_type_error(table, arg)),
    }
}

fn sqrt_type_error(table: &Table, arg: &ConcatArg) -> Error {
    match concat_arg_type(table, arg) {
        Ok(actual) => Error::new(format!(
            "argument type mismatch for function `sqrt` at #1 expected: DOUBLE, actual: {actual}"
        )),
        Err(err) => err,
    }
}

fn resolve_sign(table: &Table, expr: &SignExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(named_type_error(table, "sign", &expr.args));
    }
    let (input, result_tag) = sign_input(table, &expr.args[0])
        .map_err(|_| named_type_error(table, "sign", &expr.args))?;
    Ok(ResolvedProj::Sign {
        label: expr.label.clone(),
        input,
        result_tag,
    })
}

fn sign_input(table: &Table, arg: &ConcatArg) -> Result<(SignInput, i32), ()> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok((SignInput::Byte(0), qdb_types::TAG_BYTE)),
        ConcatArg::Literal(Scalar::Long(number)) => {
            if let Ok(number) = i32::try_from(*number) {
                Ok((SignInput::Int(number), qdb_types::TAG_INT))
            } else {
                Ok((SignInput::Long(*number), qdb_types::TAG_LONG))
            }
        }
        ConcatArg::Literal(Scalar::Double(number)) => {
            Ok((SignInput::Double(*number), qdb_types::TAG_DOUBLE))
        }
        ConcatArg::Column(name) => {
            let index = table.column_index(name).ok_or(())?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
                    | qdb_types::TAG_FLOAT
                    | qdb_types::TAG_DOUBLE
            ) {
                Ok((SignInput::Column { index }, tag))
            } else {
                Err(())
            }
        }
        ConcatArg::Literal(_) => Err(()),
    }
}

fn resolve_ceil(table: &Table, expr: &CeilExpr) -> Result<ResolvedProj, Error> {
    let name = function_name(&expr.label);
    if expr.args.len() != 1 {
        if decimal_floor(table, &expr.args) {
            return Err(Error::new(format!("unsupported: decimal {name}")));
        }
        return Err(named_type_error(table, &name, &expr.args));
    }
    let (input, wide) = floor_input(table, &expr.args[0])
        .map_err(|_| named_type_error(table, &name, &expr.args))?;
    Ok(ResolvedProj::Ceil {
        label: expr.label.clone(),
        input,
        wide,
    })
}

fn resolve_floor(table: &Table, expr: &FloorExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        if decimal_floor(table, &expr.args) {
            return Err(Error::new("unsupported: decimal floor"));
        }
        return Err(named_type_error(table, "floor", &expr.args));
    }
    let (input, wide) = floor_input(table, &expr.args[0])
        .map_err(|_| named_type_error(table, "floor", &expr.args))?;
    Ok(ResolvedProj::Floor {
        label: expr.label.clone(),
        input,
        wide,
    })
}

fn decimal_floor(table: &Table, args: &[ConcatArg]) -> bool {
    args.len() == 2 && args.iter().all(|arg| integer_floor_arg(table, arg))
}

fn integer_floor_arg(table: &Table, arg: &ConcatArg) -> bool {
    match arg {
        ConcatArg::Literal(Scalar::Long(_)) => true,
        ConcatArg::Column(name) => table.column_index(name).is_some_and(|index| {
            matches!(
                table.meta.columns[index].tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
            )
        }),
        _ => false,
    }
}

fn floor_input(table: &Table, arg: &ConcatArg) -> Result<(FloorInput, bool), ()> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok((FloorInput::Null, true)),
        ConcatArg::Literal(Scalar::Long(number)) => {
            Ok((FloorInput::Long(*number), i32::try_from(*number).is_err()))
        }
        ConcatArg::Literal(Scalar::Double(number)) => Ok((FloorInput::Double(*number), true)),
        ConcatArg::Column(name) => {
            let index = table.column_index(name).ok_or(())?;
            let tag = table.meta.columns[index].tag;
            let wide = matches!(tag, qdb_types::TAG_LONG | qdb_types::TAG_DOUBLE);
            if wide
                || matches!(
                    tag,
                    qdb_types::TAG_BYTE
                        | qdb_types::TAG_SHORT
                        | qdb_types::TAG_INT
                        | qdb_types::TAG_FLOAT
                )
            {
                Ok((FloorInput::Column { index }, wide))
            } else {
                Err(())
            }
        }
        ConcatArg::Literal(_) => Err(()),
    }
}

fn resolve_round(table: &Table, expr: &RoundExpr) -> Result<ResolvedProj, Error> {
    if expr.args.is_empty() || expr.args.len() > 2 {
        return Err(named_type_error(table, "round", &expr.args));
    }
    let input = round_input(table, &expr.args[0])
        .map_err(|_| named_type_error(table, "round", &expr.args))?;
    let scale = if expr.args.len() == 1 {
        RoundScale::Zero
    } else {
        match resolve_i32_count(table, &expr.args[1]) {
            Ok(CountArg::Null) => RoundScale::Null,
            Ok(CountArg::Fixed(0)) => RoundScale::Zero,
            Ok(CountArg::Fixed(scale)) => {
                let scale = i32::try_from(scale).unwrap_or(i32::MAX);
                if round_scale_in_range(scale) {
                    RoundScale::Fixed(scale)
                } else {
                    RoundScale::Null
                }
            }
            Ok(CountArg::ColumnInt { index })
                if table.meta.columns[index].tag != qdb_types::TAG_LONG =>
            {
                RoundScale::Column(index)
            }
            Ok(_) | Err(()) => return Err(named_type_error(table, "round", &expr.args)),
        }
    };
    Ok(ResolvedProj::Round {
        label: expr.label.clone(),
        input,
        scale,
    })
}

fn round_input(table: &Table, arg: &ConcatArg) -> Result<RoundInput, ()> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok(RoundInput::Null),
        ConcatArg::Literal(Scalar::Long(number)) => Ok(RoundInput::Long(*number)),
        ConcatArg::Literal(Scalar::Double(number)) => Ok(RoundInput::Double(*number)),
        ConcatArg::Column(name) => {
            let index = table.column_index(name).ok_or(())?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
                    | qdb_types::TAG_FLOAT
                    | qdb_types::TAG_DOUBLE
            ) {
                Ok(RoundInput::Column { index })
            } else {
                Err(())
            }
        }
        ConcatArg::Literal(_) => Err(()),
    }
}

fn named_type_error(table: &Table, name: &str, args: &[ConcatArg]) -> Error {
    let mut types = Vec::new();
    for arg in args {
        match concat_arg_type(table, arg) {
            Ok(type_name) => types.push(type_name),
            Err(err) => return err,
        }
    }
    Error::new(format!(
        "there is no matching function `{name}` with the argument types: ({})",
        types.join(", ")
    ))
}

fn resolve_abs(table: &Table, expr: &AbsExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(abs_type_error(table, &expr.args));
    }
    let (input, result_tag) = match &expr.args[0] {
        ConcatArg::Literal(Scalar::Null) => (AbsInput::Null, qdb_types::TAG_INT),
        ConcatArg::Literal(Scalar::Long(number)) => (
            AbsInput::Long(*number),
            if i32::try_from(*number).is_ok() {
                qdb_types::TAG_INT
            } else {
                qdb_types::TAG_LONG
            },
        ),
        ConcatArg::Literal(Scalar::Double(number)) => {
            (AbsInput::Double(*number), qdb_types::TAG_DOUBLE)
        }
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            let tag = table.meta.columns[index].tag;
            let result_tag = match tag {
                qdb_types::TAG_BYTE | qdb_types::TAG_SHORT => qdb_types::TAG_SHORT,
                qdb_types::TAG_INT => qdb_types::TAG_INT,
                qdb_types::TAG_LONG => qdb_types::TAG_LONG,
                qdb_types::TAG_FLOAT | qdb_types::TAG_DOUBLE => qdb_types::TAG_DOUBLE,
                _ => return Err(abs_type_error(table, &expr.args)),
            };
            (AbsInput::Column { index }, result_tag)
        }
        ConcatArg::Literal(_) => return Err(abs_type_error(table, &expr.args)),
    };
    Ok(ResolvedProj::Abs {
        label: expr.label.clone(),
        input,
        result_tag,
    })
}

fn abs_type_error(table: &Table, args: &[ConcatArg]) -> Error {
    let mut types = Vec::new();
    for arg in args {
        match concat_arg_type(table, arg) {
            Ok(type_name) => types.push(type_name),
            Err(err) => return err,
        }
    }
    Error::new(format!(
        "there is no matching function `abs` with the argument types: ({})",
        types.join(", ")
    ))
}

fn resolve_size_pretty(table: &Table, expr: &SizePrettyExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `size_pretty`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    let size = match resolve_count(table, &expr.args[0]) {
        Ok(CountArg::ColumnSymbol { .. }) | Err(()) => {
            let actual = concat_arg_type(table, &expr.args[0])?;
            return Err(Error::new(format!(
                "argument type mismatch for function `size_pretty` at #1 expected: LONG, actual: {actual}"
            )));
        }
        Ok(size) => size,
    };
    Ok(ResolvedProj::SizePretty {
        label: expr.label.clone(),
        size,
    })
}

fn resolve_quote_ident(table: &Table, expr: &QuoteIdentExpr) -> Result<ResolvedProj, Error> {
    Ok(ResolvedProj::QuoteIdent {
        label: expr.label.clone(),
        arg: resolve_one_text(table, &function_name(&expr.label), &expr.args)?,
    })
}

fn resolve_pad(table: &Table, expr: &PadExpr) -> Result<ResolvedProj, Error> {
    let name = function_name(&expr.label);
    let width = expr.args.get(1).and_then(|arg| pad_width(table, arg).ok());
    let fill_ok = expr.args.len() != 3
        || expr
            .args
            .get(2)
            .is_some_and(|arg| replace_text_ok(table, arg));
    if !matches!(expr.args.len(), 2 | 3)
        || expr
            .args
            .first()
            .is_none_or(|arg| !replace_text_ok(table, arg))
        || width.is_none()
        || !fill_ok
    {
        return Err(text_type_error(table, &name, &expr.args));
    }
    let fill = if expr.args.len() == 3 {
        Some(resolve_concat_arg(table, &expr.args[2])?)
    } else {
        None
    };
    Ok(ResolvedProj::Pad {
        label: expr.label.clone(),
        value: resolve_concat_arg(table, &expr.args[0])?,
        width: width.expect("width already checked"),
        fill,
        from_right: expr.from_right,
    })
}

fn pad_width(table: &Table, arg: &ConcatArg) -> Result<CountArg, ()> {
    match resolve_i32_count(table, arg)? {
        CountArg::ColumnSymbol { .. } => Err(()),
        width => Ok(width),
    }
}

fn resolve_split_part(table: &Table, expr: &SplitPartExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 3
        || expr
            .args
            .first()
            .is_none_or(|arg| !replace_text_ok(table, arg))
        || expr
            .args
            .get(1)
            .is_none_or(|arg| !replace_text_ok(table, arg))
        || split_index(&expr.args[2], table).is_err()
    {
        return Err(text_type_error(table, "split_part", &expr.args));
    }
    let index = match split_index(&expr.args[2], table).expect("index type already checked") {
        SplitIndex::Null => None,
        SplitIndex::Fixed(0) => {
            return Err(Error::new("field position must not be zero"));
        }
        SplitIndex::Fixed(index) => Some(index),
        SplitIndex::Column => {
            return Err(Error::new(
                "index must be either a constant expression or a placeholder",
            ));
        }
    };
    Ok(ResolvedProj::SplitPart {
        label: expr.label.clone(),
        value: resolve_concat_arg(table, &expr.args[0])?,
        delimiter: resolve_concat_arg(table, &expr.args[1])?,
        index,
    })
}

enum SplitIndex {
    Null,
    Fixed(i32),
    Column,
}

fn split_index(arg: &ConcatArg, table: &Table) -> Result<SplitIndex, ()> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok(SplitIndex::Null),
        ConcatArg::Literal(Scalar::Long(number)) => i32::try_from(*number)
            .map(SplitIndex::Fixed)
            .map_err(|_| ()),
        ConcatArg::Literal(_) => Err(()),
        ConcatArg::Column(name) => {
            let index = table.column_index(name).ok_or(())?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE | qdb_types::TAG_SHORT | qdb_types::TAG_INT
            ) {
                Ok(SplitIndex::Column)
            } else {
                Err(())
            }
        }
    }
}

fn resolve_strpos(table: &Table, expr: &StrPosExpr) -> Result<ResolvedProj, Error> {
    let name = expr
        .label
        .split_once('(')
        .map(|(name, _)| name.to_ascii_lowercase())
        .unwrap_or_else(|| "strpos".to_string());
    if expr.args.len() != 2 || expr.args.iter().any(|arg| !replace_text_ok(table, arg)) {
        return Err(text_type_error(table, &name, &expr.args));
    }
    Ok(ResolvedProj::StrPos {
        label: expr.label.clone(),
        value: resolve_concat_arg(table, &expr.args[0])?,
        needle: resolve_concat_arg(table, &expr.args[1])?,
        needle_null: matches!(expr.args[1], ConcatArg::Literal(Scalar::Null)),
    })
}

fn resolve_starts_with(table: &Table, expr: &StartsWithExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 2 || expr.args.iter().any(|arg| !replace_text_ok(table, arg)) {
        return Err(text_type_error(table, "starts_with", &expr.args));
    }
    Ok(ResolvedProj::StartsWith {
        label: expr.label.clone(),
        value: resolve_concat_arg(table, &expr.args[0])?,
        prefix: resolve_concat_arg(table, &expr.args[1])?,
    })
}

fn resolve_replace(table: &Table, expr: &ReplaceExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 3 || expr.args.iter().any(|arg| !replace_text_ok(table, arg)) {
        return Err(text_type_error(table, "replace", &expr.args));
    }
    let mode = if matches!(expr.args[2], ConcatArg::Literal(Scalar::Null))
        || matches!(expr.args[1], ConcatArg::Literal(Scalar::Null))
    {
        ReplaceMode::Null
    } else if matches!(&expr.args[1], ConcatArg::Literal(Scalar::Text(text)) if text.is_empty()) {
        ReplaceMode::Value(resolve_concat_arg(table, &expr.args[0])?)
    } else if matches!(expr.args[0], ConcatArg::Literal(Scalar::Null)) {
        ReplaceMode::Null
    } else if matches!(&expr.args[0], ConcatArg::Literal(Scalar::Text(text)) if text.is_empty()) {
        ReplaceMode::Empty
    } else {
        ReplaceMode::Run {
            value: resolve_concat_arg(table, &expr.args[0])?,
            old: resolve_concat_arg(table, &expr.args[1])?,
            new: resolve_concat_arg(table, &expr.args[2])?,
        }
    };
    Ok(ResolvedProj::Replace {
        label: expr.label.clone(),
        mode,
    })
}

fn replace_text_ok(table: &Table, arg: &ConcatArg) -> bool {
    match arg {
        ConcatArg::Literal(Scalar::Text(_)) | ConcatArg::Literal(Scalar::Null) => true,
        ConcatArg::Literal(_) => false,
        ConcatArg::Column(name) => table.column_index(name).is_some_and(|index| {
            matches!(
                table.meta.columns[index].tag,
                qdb_types::TAG_STRING
                    | qdb_types::TAG_SYMBOL
                    | qdb_types::TAG_VARCHAR
                    | qdb_types::TAG_CHAR
            )
        }),
    }
}

fn text_type_error(table: &Table, name: &str, args: &[ConcatArg]) -> Error {
    let mut types = Vec::new();
    for arg in args {
        match replace_arg_type(table, arg) {
            Ok(type_name) => types.push(type_name),
            Err(err) => return err,
        }
    }
    Error::new(format!(
        "there is no matching function `{name}` with the argument types: ({})",
        types.join(", ")
    ))
}

fn replace_arg_type(table: &Table, arg: &ConcatArg) -> Result<String, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok("NULL".to_string()),
        ConcatArg::Literal(Scalar::Text(text)) if text.encode_utf16().count() == 1 => {
            Ok("CHAR".to_string())
        }
        ConcatArg::Literal(Scalar::Text(_)) => Ok("STRING".to_string()),
        ConcatArg::Literal(Scalar::Long(number)) => {
            if i32::try_from(*number).is_ok() {
                Ok("INT".to_string())
            } else {
                Ok("LONG".to_string())
            }
        }
        ConcatArg::Literal(Scalar::Double(_)) => Ok("DOUBLE".to_string()),
        ConcatArg::Literal(_) => Ok("STRING".to_string()),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            Ok(qdb_types::type_name(table.meta.columns[index].tag).to_string())
        }
    }
}

fn resolve_trim(table: &Table, expr: &TrimExpr) -> Result<ResolvedProj, Error> {
    let name = match expr.end {
        TrimEnd::Left => "ltrim",
        TrimEnd::Right => "rtrim",
        TrimEnd::Both => "trim",
    };
    Ok(ResolvedProj::Trim {
        label: expr.label.clone(),
        arg: resolve_one_text(table, name, &expr.args)?,
        end: expr.end,
    })
}

fn resolve_substr(table: &Table, expr: &SubstrExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 3 {
        return Err(side_type_error(table, "substring", &expr.args));
    }
    let text = resolve_concat_arg(table, &expr.args[0])?;
    if !side_text_arg(&expr.args[0], &text) {
        return Err(side_type_error(table, "substring", &expr.args));
    }
    let start = match resolve_i32_count(table, &expr.args[1]) {
        Ok(count) => count,
        Err(()) => return Err(side_type_error(table, "substring", &expr.args)),
    };
    let len = match resolve_i32_count(table, &expr.args[2]) {
        Ok(count) => count,
        Err(()) => return Err(side_type_error(table, "substring", &expr.args)),
    };
    let mode = if matches!(expr.args[0], ConcatArg::Literal(Scalar::Null))
        || matches!(start, CountArg::Null)
        || matches!(len, CountArg::Null)
    {
        SubstrMode::Null
    } else if let CountArg::Fixed(len_value) = len {
        if len_value < 0 {
            return Err(Error::new("negative substring length is not allowed"));
        }
        if len_value == 0 {
            SubstrMode::Empty
        } else if let CountArg::Fixed(start_value) = start {
            let start_i = i32::try_from(start_value).unwrap_or(i32::MAX);
            let len_i = i32::try_from(len_value).unwrap_or(i32::MAX);
            if start_i.wrapping_add(len_i) < 1 {
                SubstrMode::Null
            } else {
                SubstrMode::Slice { text, start, len }
            }
        } else {
            SubstrMode::Slice { text, start, len }
        }
    } else {
        SubstrMode::Slice { text, start, len }
    };
    Ok(ResolvedProj::Substr {
        label: expr.label.clone(),
        mode,
    })
}

fn resolve_i32_count(table: &Table, arg: &ConcatArg) -> Result<CountArg, ()> {
    match resolve_count(table, arg)? {
        CountArg::Fixed(number) if i32::try_from(number).is_err() => Err(()),
        CountArg::ColumnInt { index } if table.meta.columns[index].tag == qdb_types::TAG_LONG => {
            Err(())
        }
        count => Ok(count),
    }
}

fn reject_negative_substring(table: &Table, row: usize, item: &ResolvedProj) -> Result<(), Error> {
    if let ResolvedProj::Substr {
        mode: SubstrMode::Slice { len, .. },
        ..
    } = item
    {
        if count_of(table, row, len).is_some_and(|len| len < 0) {
            return Err(Error::new("negative substring length is not allowed"));
        }
    }
    Ok(())
}

fn resolve_side(table: &Table, expr: &SideExpr) -> Result<ResolvedProj, Error> {
    let name = if expr.from_right { "right" } else { "left" };
    if expr.args.len() != 2 {
        return Err(side_type_error(table, name, &expr.args));
    }
    let text = resolve_concat_arg(table, &expr.args[0])?;
    if !side_text_arg(&expr.args[0], &text) {
        return Err(side_type_error(table, name, &expr.args));
    }
    let count = match resolve_count(table, &expr.args[1]) {
        Ok(count) => count,
        Err(()) => return Err(side_type_error(table, name, &expr.args)),
    };
    Ok(ResolvedProj::Side {
        label: expr.label.clone(),
        from_right: expr.from_right,
        text,
        count,
    })
}

fn side_text_arg(arg: &ConcatArg, resolved: &ResolvedArg) -> bool {
    match arg {
        ConcatArg::Literal(Scalar::Text(_)) | ConcatArg::Literal(Scalar::Null) => true,
        ConcatArg::Literal(_) => false,
        ConcatArg::Column(_) => side_text_ok(resolved),
    }
}

fn side_text_ok(arg: &ResolvedArg) -> bool {
    match arg {
        ResolvedArg::Text(_) | ResolvedArg::Null => true,
        ResolvedArg::Column { tag, .. } => matches!(
            *tag,
            qdb_types::TAG_STRING
                | qdb_types::TAG_SYMBOL
                | qdb_types::TAG_VARCHAR
                | qdb_types::TAG_CHAR
        ),
    }
}

fn resolve_count(table: &Table, arg: &ConcatArg) -> Result<CountArg, ()> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok(CountArg::Null),
        ConcatArg::Literal(Scalar::Long(number)) => Ok(CountArg::Fixed(*number)),
        ConcatArg::Literal(_) => Err(()),
        ConcatArg::Column(name) => {
            let index = table.column_index(name).ok_or(())?;
            let tag = table.meta.columns[index].tag;
            if matches!(
                tag,
                qdb_types::TAG_BYTE
                    | qdb_types::TAG_SHORT
                    | qdb_types::TAG_INT
                    | qdb_types::TAG_LONG
            ) {
                return Ok(CountArg::ColumnInt { index });
            }
            if tag == qdb_types::TAG_SYMBOL {
                let mut keys = Vec::new();
                for value in &table.columns[index] {
                    if value.is_null() || keys.contains(value) {
                        continue;
                    }
                    keys.push(value.clone());
                }
                return Ok(CountArg::ColumnSymbol { index, keys });
            }
            Err(())
        }
    }
}

fn side_type_error(table: &Table, name: &str, args: &[ConcatArg]) -> Error {
    let mut types = Vec::new();
    for arg in args {
        match side_arg_type(table, arg) {
            Ok(type_name) => types.push(type_name),
            Err(err) => return err,
        }
    }
    Error::new(format!(
        "there is no matching function `{name}` with the argument types: ({})",
        types.join(", ")
    ))
}

fn side_arg_type(table: &Table, arg: &ConcatArg) -> Result<String, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok("NULL".to_string()),
        ConcatArg::Literal(Scalar::Long(number)) => {
            if i32::try_from(*number).is_ok() {
                Ok("INT".to_string())
            } else {
                Ok("LONG".to_string())
            }
        }
        ConcatArg::Literal(Scalar::Double(_)) => Ok("DOUBLE".to_string()),
        ConcatArg::Literal(_) => Ok("STRING".to_string()),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            Ok(qdb_types::type_name(table.meta.columns[index].tag).to_string())
        }
    }
}

fn resolve_upper(table: &Table, expr: &UpperExpr) -> Result<ResolvedProj, Error> {
    Ok(ResolvedProj::Upper {
        label: expr.label.clone(),
        arg: resolve_one_text(table, &function_name(&expr.label), &expr.args)?,
    })
}

fn resolve_lower(table: &Table, expr: &LowerExpr) -> Result<ResolvedProj, Error> {
    Ok(ResolvedProj::Lower {
        label: expr.label.clone(),
        arg: resolve_one_text(table, &function_name(&expr.label), &expr.args)?,
    })
}

fn function_name(label: &str) -> String {
    label
        .split_once('(')
        .map(|(name, _)| name)
        .unwrap_or(label)
        .to_ascii_lowercase()
}

fn resolve_one_text(table: &Table, name: &str, args: &[ConcatArg]) -> Result<ResolvedArg, Error> {
    if args.len() != 1 {
        let types = args
            .iter()
            .map(|arg| concat_arg_type(table, arg))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        return Err(Error::new(format!(
            "there is no matching function `{name}` with the argument types: ({types})"
        )));
    }
    let arg = resolve_concat_arg(table, &args[0])?;
    if let ResolvedArg::Column { tag, .. } = &arg {
        if !matches!(
            *tag,
            qdb_types::TAG_STRING
                | qdb_types::TAG_SYMBOL
                | qdb_types::TAG_VARCHAR
                | qdb_types::TAG_CHAR
        ) {
            return Err(Error::new(format!(
                "there is no matching function `{name}` with the argument types: ({})",
                qdb_types::type_name(*tag)
            )));
        }
    }
    Ok(arg)
}

fn resolve_length(table: &Table, expr: &LengthExpr) -> Result<ResolvedProj, Error> {
    if expr.bytes {
        return resolve_length_bytes(table, expr);
    }
    if expr.args.len() != 1 {
        let types = expr
            .args
            .iter()
            .map(|arg| concat_arg_type(table, arg))
            .collect::<Result<Vec<_>, _>>()?
            .join(", ");
        return Err(Error::new(format!(
            "there is no matching function `length` with the argument types: ({types})"
        )));
    }
    let arg = resolve_concat_arg(table, &expr.args[0])?;
    if let ResolvedArg::Column { tag, .. } = &arg {
        if !matches!(
            *tag,
            qdb_types::TAG_STRING
                | qdb_types::TAG_SYMBOL
                | qdb_types::TAG_VARCHAR
                | qdb_types::TAG_CHAR
        ) {
            return Err(Error::new(format!(
                "there is no matching function `length` with the argument types: ({})",
                qdb_types::type_name(*tag)
            )));
        }
    }
    Ok(ResolvedProj::Length {
        label: expr.label.clone(),
        arg,
        bytes: false,
    })
}

fn resolve_length_bytes(table: &Table, expr: &LengthExpr) -> Result<ResolvedProj, Error> {
    if expr.args.len() != 1 {
        return Err(Error::new(format!(
            "wrong number of arguments for function `length_bytes`; expected: 1, provided: {}",
            expr.args.len()
        )));
    }
    length_bytes_arg(table, &expr.args[0])?;
    Ok(ResolvedProj::Length {
        label: expr.label.clone(),
        arg: resolve_concat_arg(table, &expr.args[0])?,
        bytes: true,
    })
}

fn length_bytes_arg(table: &Table, arg: &ConcatArg) -> Result<(), Error> {
    let accepted = match arg {
        ConcatArg::Literal(Scalar::Null | Scalar::Text(_)) => true,
        ConcatArg::Literal(_) => false,
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            matches!(
                table.meta.columns[index].tag,
                qdb_types::TAG_STRING | qdb_types::TAG_VARCHAR | qdb_types::TAG_CHAR
            )
        }
    };
    if accepted {
        return Ok(());
    }
    let actual = concat_arg_type(table, arg)?;
    Err(Error::new(format!(
        "argument type mismatch for function `length_bytes` at #1 expected: VARCHAR, actual: {actual}"
    )))
}

fn concat_arg_type(table: &Table, arg: &ConcatArg) -> Result<String, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Null) => Ok("NULL".to_string()),
        ConcatArg::Literal(Scalar::Text(text)) if text.encode_utf16().count() == 1 => {
            Ok("CHAR".to_string())
        }
        ConcatArg::Literal(Scalar::Long(number)) => {
            if i32::try_from(*number).is_ok() {
                Ok("INT".to_string())
            } else {
                Ok("LONG".to_string())
            }
        }
        ConcatArg::Literal(Scalar::Double(_)) => Ok("DOUBLE".to_string()),
        ConcatArg::Literal(_) => Ok("STRING".to_string()),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            Ok(qdb_types::type_name(table.meta.columns[index].tag).to_string())
        }
    }
}

fn resolve_concat_arg(table: &Table, arg: &ConcatArg) -> Result<ResolvedArg, Error> {
    match arg {
        ConcatArg::Literal(Scalar::Text(text)) => Ok(ResolvedArg::Text(text.clone())),
        ConcatArg::Literal(_) => Ok(ResolvedArg::Null),
        ConcatArg::Column(name) => {
            let index = table
                .column_index(name)
                .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
            Ok(ResolvedArg::Column {
                index,
                tag: table.meta.columns[index].tag,
            })
        }
    }
}

fn eval_resolved(table: &Table, row: usize, item: &ResolvedProj) -> Scalar {
    match item {
        ResolvedProj::Column { index } => table.columns[*index][row].clone(),
        ResolvedProj::Concat { args, .. } => Scalar::Text(
            args.iter()
                .map(|arg| match arg {
                    ResolvedArg::Text(text) => text.clone(),
                    ResolvedArg::Null => String::new(),
                    ResolvedArg::Column { index, tag } => {
                        cast_concat(&table.columns[*index][row], *tag)
                    }
                })
                .collect(),
        ),
        ResolvedProj::Length { arg, bytes, .. } => {
            Scalar::Int(text_length(table, row, arg, *bytes))
        }
        ResolvedProj::Lower { arg, .. } => match text_of(table, row, arg) {
            Some(text) => Scalar::Text(text.to_lowercase()),
            None => Scalar::Null,
        },
        ResolvedProj::Upper { arg, .. } => match text_of(table, row, arg) {
            Some(text) => Scalar::Text(text.to_uppercase()),
            None => Scalar::Null,
        },
        ResolvedProj::Side {
            from_right,
            text,
            count,
            ..
        } => match (text_of(table, row, text), count_of(table, row, count)) {
            (Some(text), Some(count)) => Scalar::Text(take_chars(&text, count, *from_right)),
            _ => Scalar::Null,
        },
        ResolvedProj::Substr { mode, .. } => match mode {
            SubstrMode::Null => Scalar::Null,
            SubstrMode::Empty => Scalar::Text(String::new()),
            SubstrMode::Slice { text, start, len } => {
                match (
                    text_of(table, row, text),
                    count_of(table, row, start),
                    count_of(table, row, len),
                ) {
                    (Some(text), Some(start), Some(len)) => {
                        Scalar::Text(substring_utf16(&text, start, len))
                    }
                    _ => Scalar::Null,
                }
            }
        },
        ResolvedProj::Floor { input, wide, .. } => floor_value(table, row, input, *wide),
        ResolvedProj::Ceil { input, wide, .. } => ceil_value(table, row, input, *wide),
        ResolvedProj::Sign { input, .. } => sign_value(table, row, input),
        ResolvedProj::Power { base, exponent, .. } => power_value(table, row, base, exponent),
        ResolvedProj::Log { input, .. } => log_value(table, row, input, f64::log10),
        ResolvedProj::Exp { input, .. } => log_value(table, row, input, f64::exp),
        ResolvedProj::Pi { .. } => Scalar::Double(std::f64::consts::PI),
        ResolvedProj::Ln { input, .. } => log_value(table, row, input, f64::ln),
        ResolvedProj::Sqrt { input, .. } => match sqrt_number(table, row, input) {
            Some(value) => {
                let result = value.sqrt();
                if result.is_nan() {
                    Scalar::Null
                } else {
                    Scalar::Double(result)
                }
            }
            None => Scalar::Null,
        },
        ResolvedProj::Round { input, scale, .. } => match round_row(table, row, input, scale) {
            Some(value) => Scalar::Double(value),
            None => Scalar::Null,
        },
        ResolvedProj::Abs { input, .. } => abs_value(table, row, input),
        ResolvedProj::SizePretty { size, .. } => match count_of(table, row, size) {
            Some(size) if size != i64::MIN => Scalar::Text(size_pretty(size)),
            _ => Scalar::Null,
        },
        ResolvedProj::QuoteIdent { arg, .. } => match text_of(table, row, arg) {
            Some(text) => Scalar::Text(quote_ident(&text)),
            None => Scalar::Null,
        },
        ResolvedProj::Pad {
            value,
            width,
            fill,
            from_right,
            ..
        } => {
            let Some(text) = text_of(table, row, value) else {
                return Scalar::Null;
            };
            let Some(width) = count_of(table, row, width) else {
                return Scalar::Null;
            };
            let Ok(width) = i32::try_from(width) else {
                return Scalar::Null;
            };
            if width < 0 {
                return Scalar::Null;
            }
            let fill = match fill {
                None => " ".to_string(),
                Some(fill) => match text_of(table, row, fill) {
                    Some(fill) if !fill.is_empty() => fill,
                    _ => return Scalar::Null,
                },
            };
            Scalar::Text(pad_utf16(&text, width, &fill, *from_right))
        }
        ResolvedProj::SplitPart {
            value,
            delimiter,
            index,
            ..
        } => {
            let Some(index) = *index else {
                return Scalar::Null;
            };
            match (text_of(table, row, value), text_of(table, row, delimiter)) {
                (Some(value), Some(delimiter)) => {
                    Scalar::Text(split_part_utf16(&value, &delimiter, index))
                }
                _ => Scalar::Null,
            }
        }
        ResolvedProj::StrPos {
            value,
            needle,
            needle_null,
            ..
        } => {
            if *needle_null {
                Scalar::Null
            } else {
                match text_of(table, row, value) {
                    None => Scalar::Null,
                    Some(value) => match text_of(table, row, needle) {
                        None => Scalar::Null,
                        Some(needle) => Scalar::Int(strpos_utf16(&value, &needle)),
                    },
                }
            }
        }
        ResolvedProj::StartsWith { value, prefix, .. } => {
            let matched = match (text_of(table, row, value), text_of(table, row, prefix)) {
                (Some(value), Some(prefix)) => value.starts_with(&prefix),
                _ => false,
            };
            Scalar::Boolean(matched)
        }
        ResolvedProj::Replace { mode, .. } => match mode {
            ReplaceMode::Null => Scalar::Null,
            ReplaceMode::Empty => Scalar::Text(String::new()),
            ReplaceMode::Value(arg) => match text_of(table, row, arg) {
                Some(text) => Scalar::Text(text),
                None => Scalar::Null,
            },
            ReplaceMode::Run { value, old, new } => match text_of(table, row, value) {
                None => Scalar::Null,
                Some(value) if value.is_empty() => Scalar::Text(String::new()),
                Some(value) => match (text_of(table, row, old), text_of(table, row, new)) {
                    (Some(old), Some(new)) => Scalar::Text(replace_utf16(&value, &old, &new)),
                    _ => Scalar::Null,
                },
            },
        },
        ResolvedProj::Trim { arg, end, .. } => match text_of(table, row, arg) {
            Some(text) => Scalar::Text(trim_ascii_space(&text, *end)),
            None => Scalar::Null,
        },
    }
}

fn power_value(table: &Table, row: usize, base: &SqrtInput, exponent: &SqrtInput) -> Scalar {
    let Some(exponent) = sqrt_number(table, row, exponent) else {
        return Scalar::Null;
    };
    let base = match sqrt_number(table, row, base) {
        Some(value) => value,
        None => {
            if exponent == 0.0 {
                return Scalar::Double(1.0);
            }
            return Scalar::Null;
        }
    };
    let result = base.powf(exponent);
    if result.is_finite() {
        Scalar::Double(result)
    } else {
        Scalar::Null
    }
}

fn log_value(table: &Table, row: usize, input: &SqrtInput, function: fn(f64) -> f64) -> Scalar {
    match sqrt_number(table, row, input) {
        Some(value) => {
            let result = function(value);
            if result.is_nan() {
                Scalar::Null
            } else {
                Scalar::Double(result)
            }
        }
        None => Scalar::Null,
    }
}

fn sqrt_number(table: &Table, row: usize, input: &SqrtInput) -> Option<f64> {
    match input {
        SqrtInput::Null => None,
        SqrtInput::Long(number) => Some(*number as f64),
        SqrtInput::Double(number) => Some(*number),
        SqrtInput::Column { index } => match &table.columns[*index][row] {
            Scalar::Null => None,
            Scalar::Byte(number) => Some(f64::from(*number)),
            Scalar::Short(number) => Some(f64::from(*number)),
            Scalar::Int(number) => Some(f64::from(*number)),
            Scalar::Long(number) => Some(*number as f64),
            Scalar::Float(number) => Some(f64::from(*number)),
            Scalar::Double(number) => Some(*number),
            _ => None,
        },
    }
}

fn sign_value(table: &Table, row: usize, input: &SignInput) -> Scalar {
    match input {
        SignInput::Byte(number) => Scalar::Byte(number.signum()),
        SignInput::Int(number) => Scalar::Int(number.signum()),
        SignInput::Long(number) => Scalar::Long(number.signum()),
        SignInput::Double(number) => Scalar::Double(sign_f64(*number)),
        SignInput::Column { index } => match &table.columns[*index][row] {
            Scalar::Null => Scalar::Null,
            Scalar::Byte(number) => Scalar::Byte(number.signum()),
            Scalar::Short(number) => Scalar::Short(number.signum()),
            Scalar::Int(number) => Scalar::Int(number.signum()),
            Scalar::Long(number) => Scalar::Long(number.signum()),
            Scalar::Float(number) => Scalar::Float(sign_f32(*number)),
            Scalar::Double(number) => Scalar::Double(sign_f64(*number)),
            _ => Scalar::Null,
        },
    }
}

fn sign_f64(value: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        0.0
    } else if value.is_sign_negative() {
        -1.0
    } else {
        1.0
    }
}

fn sign_f32(value: f32) -> f32 {
    if value.is_nan() || value == 0.0 {
        value
    } else if value.is_sign_negative() {
        -1.0
    } else {
        1.0
    }
}

fn ceil_value(table: &Table, row: usize, input: &FloorInput, wide: bool) -> Scalar {
    if wide {
        return match floor_f64(table, row, input) {
            Some(value) => Scalar::Double(value.ceil()),
            None => Scalar::Null,
        };
    }
    match floor_f32(table, row, input) {
        Some(value) => Scalar::Float(value.ceil()),
        None => Scalar::Null,
    }
}

fn floor_value(table: &Table, row: usize, input: &FloorInput, wide: bool) -> Scalar {
    if wide {
        return match floor_f64(table, row, input) {
            Some(value) => Scalar::Double(value.floor()),
            None => Scalar::Null,
        };
    }
    match floor_f32(table, row, input) {
        Some(value) => Scalar::Float(value.floor()),
        None => Scalar::Null,
    }
}

fn floor_f64(table: &Table, row: usize, input: &FloorInput) -> Option<f64> {
    match input {
        FloorInput::Null => None,
        FloorInput::Long(number) => Some(*number as f64),
        FloorInput::Double(number) => Some(*number),
        FloorInput::Column { index } => match &table.columns[*index][row] {
            Scalar::Null => None,
            Scalar::Long(number) => Some(*number as f64),
            Scalar::Double(number) => Some(*number),
            _ => None,
        },
    }
}

fn floor_f32(table: &Table, row: usize, input: &FloorInput) -> Option<f32> {
    match input {
        FloorInput::Null => None,
        FloorInput::Long(number) => Some(*number as f32),
        FloorInput::Double(_) => None,
        FloorInput::Column { index } => match &table.columns[*index][row] {
            Scalar::Null => None,
            Scalar::Byte(number) => Some(f32::from(*number)),
            Scalar::Short(number) => Some(f32::from(*number)),
            Scalar::Int(number) => Some(*number as f32),
            Scalar::Float(number) => Some(*number),
            _ => None,
        },
    }
}

fn round_row(table: &Table, row: usize, input: &RoundInput, scale: &RoundScale) -> Option<f64> {
    let scale = match scale {
        RoundScale::Null => return None,
        RoundScale::Zero => 0,
        RoundScale::Fixed(scale) => *scale,
        RoundScale::Column(index) => match &table.columns[*index][row] {
            Scalar::Null => return None,
            Scalar::Byte(number) => i32::from(*number),
            Scalar::Short(number) => i32::from(*number),
            Scalar::Int(number) => *number,
            _ => return None,
        },
    };
    let value = round_number(table, row, input)?;
    round_half_up(value, scale)
}

fn round_number(table: &Table, row: usize, input: &RoundInput) -> Option<f64> {
    match input {
        RoundInput::Null => None,
        RoundInput::Long(number) => Some(*number as f64),
        RoundInput::Double(number) => Some(*number),
        RoundInput::Column { index, .. } => match &table.columns[*index][row] {
            Scalar::Null => None,
            Scalar::Byte(number) => Some(f64::from(*number)),
            Scalar::Short(number) => Some(f64::from(*number)),
            Scalar::Int(number) => Some(f64::from(*number)),
            Scalar::Long(number) => Some(*number as f64),
            Scalar::Float(number) => Some(f64::from(*number)),
            Scalar::Double(number) => Some(*number),
            _ => None,
        },
    }
}

fn round_scale_in_range(scale: i32) -> bool {
    scale > -18 && scale + 2 < 18
}

fn round_half_up(value: f64, scale: i32) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    if scale == 0 {
        return Some((value + 0.5).floor());
    }
    if !round_scale_in_range(scale) {
        return None;
    }
    let sign = value.to_bits() & (1_u64 << 63);
    let magnitude = if scale > 0 {
        round_half_up_positive(value.abs(), scale)
    } else {
        round_half_up_negative(value.abs(), -scale)
    };
    Some(f64::from_bits(magnitude.to_bits() | sign))
}

fn round_half_up_positive(value: f64, scale: i32) -> f64 {
    let factor = pow10(scale + 2) as f64;
    let digits = ((value + 1e-15) * factor) as i64;
    if digits % 100 < 50 {
        round_down_positive(value, scale)
    } else {
        round_up_positive(value, scale)
    }
}

fn round_half_up_negative(value: f64, scale: i32) -> f64 {
    let digits = (value * pow10_neg(scale) * pow10(2) as f64 + 1e-15) as i64;
    if digits % 100 < 50 {
        round_down_negative(value, scale)
    } else {
        round_up_negative(value, scale)
    }
}

fn round_down_positive(value: f64, scale: i32) -> f64 {
    ((value + 1e-15) * pow10(scale) as f64) as i64 as f64 * pow10_neg(scale)
}

fn round_up_positive(value: f64, scale: i32) -> f64 {
    (value * pow10(scale) as f64 + 1.0 - 1e-15) as i64 as f64 * pow10_neg(scale)
}

fn round_down_negative(value: f64, scale: i32) -> f64 {
    ((value + 1e-15) * pow10_neg(scale)) as i64 as f64 * pow10(scale) as f64
}

fn round_up_negative(value: f64, scale: i32) -> f64 {
    (value * pow10_neg(scale) + 1.0 - 1e-15) as i64 as f64 * pow10(scale) as f64
}

fn pow10(scale: i32) -> i64 {
    10i64.pow(scale as u32)
}

fn pow10_neg(scale: i32) -> f64 {
    10f64.powi(-scale)
}

fn abs_value(table: &Table, row: usize, input: &AbsInput) -> Scalar {
    match input {
        AbsInput::Null => Scalar::Null,
        AbsInput::Long(number) => match i32::try_from(*number) {
            Ok(number) => Scalar::Int(number.wrapping_abs()),
            Err(_) => Scalar::Long(number.wrapping_abs()),
        },
        AbsInput::Double(number) => Scalar::Double(number.abs()),
        AbsInput::Column { index, .. } => match &table.columns[*index][row] {
            Scalar::Null => Scalar::Null,
            Scalar::Byte(number) => Scalar::Short(i16::from(*number).wrapping_abs()),
            Scalar::Short(number) => Scalar::Short(number.wrapping_abs()),
            Scalar::Int(number) => Scalar::Int(number.wrapping_abs()),
            Scalar::Long(number) => Scalar::Long(number.wrapping_abs()),
            Scalar::Float(number) => Scalar::Double(f64::from(number.abs())),
            Scalar::Double(number) => Scalar::Double(number.abs()),
            _ => Scalar::Null,
        },
    }
}

fn size_pretty(size: i64) -> String {
    const SCALE: [char; 8] = ['B', 'K', 'M', 'G', 'T', 'P', 'E', 'Z'];
    let mut band = (63 - size.leading_zeros() as i32) / 10;
    if band < 0 {
        band = 0;
    }
    let scale = 1i64 << (band * 10);
    let value = (size as f32) / (scale as f32);
    let mut text = append_scaled_f32(value, 1);
    text.push(' ');
    text.push(SCALE[band as usize]);
    if band > 0 {
        text.push_str("iB");
    }
    text
}

fn append_scaled_f32(value: f32, scale: i32) -> String {
    if value.is_infinite() && value.is_sign_positive() {
        return "Infinity".to_string();
    }
    if value.is_infinite() {
        return "-Infinity".to_string();
    }
    if value.is_nan() {
        return "NaN".to_string();
    }
    let mut out = String::new();
    let mut magnitude = value;
    if magnitude.is_sign_negative() {
        out.push('-');
        magnitude = -magnitude;
    }
    let mut factor = 10i32.pow(scale as u32);
    let scaled = (magnitude * factor as f32 + 0.5) as i32;
    let mut target = scale + 1;
    while target < 11 {
        let Some(next) = factor.checked_mul(10) else {
            break;
        };
        if next > scaled {
            break;
        }
        factor = next;
        target += 1;
    }
    while target > 0 {
        if target == scale {
            out.push('.');
        }
        target -= 1;
        let digit = scaled / factor % 10;
        out.push(char::from(b'0' + digit as u8));
        factor /= 10;
    }
    out
}

fn quote_ident(text: &str) -> String {
    let plain = text
        .chars()
        .all(|ch| ch.is_alphabetic() || ch.is_ascii_digit() || ch == '_' || ch == '$');
    if plain {
        return text.to_string();
    }
    let mut quoted = String::from("\"");
    for ch in text.chars() {
        if ch == '"' {
            quoted.push('"');
        }
        quoted.push(ch);
    }
    quoted.push('"');
    quoted
}

fn pad_utf16(text: &str, width: i32, fill: &str, from_right: bool) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let width = width as usize;
    if width <= units.len() {
        let slice = if from_right {
            &units[units.len() - width..]
        } else {
            &units[..width]
        };
        return String::from_utf16(slice).unwrap_or_else(|_| String::from_utf16_lossy(slice));
    }
    let fill_units: Vec<u16> = fill.encode_utf16().collect();
    let need = width - units.len();
    let mut out = Vec::with_capacity(width);
    let copies = need / fill_units.len();
    let rest = need % fill_units.len();
    if from_right {
        out.extend_from_slice(&units);
    }
    for _ in 0..copies {
        out.extend_from_slice(&fill_units);
    }
    out.extend_from_slice(&fill_units[..rest]);
    if !from_right {
        out.extend_from_slice(&units);
    }
    String::from_utf16(&out).unwrap_or_else(|_| String::from_utf16_lossy(&out))
}

fn split_part_utf16(value: &str, delimiter: &str, index: i32) -> String {
    if index == 0 {
        return String::new();
    }
    let seq: Vec<u16> = value.encode_utf16().collect();
    let term: Vec<u16> = delimiter.encode_utf16().collect();
    let len = i32::try_from(seq.len()).unwrap_or(i32::MAX);
    let (start, end) = if index > 0 {
        let start = if index == 1 {
            0
        } else {
            let found = index_of_nth(&seq, 0, len, &term, index - 1);
            if found < 0 {
                return String::new();
            }
            found + term.len() as i32
        };
        let end = index_of_first(&seq, start, len, &term);
        (start, if end < 0 { len } else { end })
    } else {
        let end = if index == -1 {
            len
        } else {
            let found = index_of_nth(&seq, 0, len, &term, index + 1);
            if found < 0 {
                return String::new();
            }
            found
        };
        let found = index_of_nth(&seq, 0, end, &term, -1);
        let start = if found < 0 {
            0
        } else {
            found + term.len() as i32
        };
        (start, end)
    };
    utf16_slice(&seq, start, end)
}

fn index_of_first(seq: &[u16], lo: i32, hi: i32, term: &[u16]) -> i32 {
    let term_len = term.len() as i32;
    if term_len == 0 {
        return 0;
    }
    let first = term[0];
    let max = hi - term_len;
    let mut index = lo;
    while index <= max {
        if seq[index as usize] != first {
            loop {
                index += 1;
                if index > max || seq[index as usize] == first {
                    break;
                }
            }
        }
        if index <= max {
            let mut cursor = index + 1;
            let end = cursor + term_len - 1;
            let mut term_index = 1i32;
            while cursor < end && seq[cursor as usize] == term[term_index as usize] {
                term_index += 1;
                cursor += 1;
            }
            if cursor == end {
                return index;
            }
        }
        index += 1;
    }
    -1
}

fn index_of_nth(seq: &[u16], lo: i32, hi: i32, term: &[u16], occurrence: i32) -> i32 {
    let term_len = term.len() as i32;
    if term_len == 0 {
        return 0;
    }
    if occurrence == 0 {
        return -1;
    }
    let mut found = -1i32;
    let mut count = 0i32;
    if occurrence > 0 {
        let mut index = lo;
        while index < hi {
            if found == -1 {
                if hi - index < term_len {
                    return -1;
                }
                if seq[index as usize] == term[0] {
                    found = index;
                }
            } else if seq[index as usize] != term[(index - found) as usize] {
                index = found;
                found = -1;
            }
            if found != -1 && index - found == term_len - 1 {
                count += 1;
                if count == occurrence {
                    return found;
                }
                found = -1;
            }
            index += 1;
        }
    } else {
        let mut index = hi - 1;
        while index >= lo {
            if found == -1 {
                if index - lo + 1 < term_len {
                    return -1;
                }
                if seq[index as usize] == term[(term_len - 1) as usize] {
                    found = index;
                }
            } else if seq[index as usize] != term[(term_len - 1 + index - found) as usize] {
                index = found;
                found = -1;
            }
            if found != -1 && found - index == term_len - 1 {
                count -= 1;
                if count == occurrence {
                    return found + 1 - term_len;
                }
                found = -1;
            }
            index -= 1;
        }
    }
    -1
}

fn utf16_slice(units: &[u16], start: i32, end: i32) -> String {
    if start < 0 || end < start {
        return String::new();
    }
    let start = start as usize;
    let end = (end as usize).min(units.len());
    if start >= end || start > units.len() {
        return String::new();
    }
    let slice = &units[start..end];
    String::from_utf16(slice).unwrap_or_else(|_| String::from_utf16_lossy(slice))
}

fn strpos_utf16(value: &str, needle: &str) -> i32 {
    let value_units: Vec<u16> = value.encode_utf16().collect();
    let needle_units: Vec<u16> = needle.encode_utf16().collect();
    if needle_units.is_empty() {
        return 1;
    }
    if value_units.len() < needle_units.len() {
        return 0;
    }
    let last = value_units.len() - needle_units.len();
    for index in 0..=last {
        if value_units[index..index + needle_units.len()] == needle_units[..] {
            return i32::try_from(index + 1).unwrap_or(i32::MAX);
        }
    }
    0
}

fn replace_utf16(value: &str, term: &str, with: &str) -> String {
    if term.is_empty() {
        return value.to_string();
    }
    let value_units: Vec<u16> = value.encode_utf16().collect();
    let term_units: Vec<u16> = term.encode_utf16().collect();
    let with_units: Vec<u16> = with.encode_utf16().collect();
    if value_units.is_empty() || term_units.is_empty() {
        return value.to_string();
    }
    let mut out = Vec::new();
    let mut index = 0;
    while index < value_units.len() {
        let rest = &value_units[index..];
        if rest.starts_with(&term_units) {
            out.extend_from_slice(&with_units);
            index += term_units.len();
        } else if rest.len() < term_units.len() && rest[0] == term_units[0] {
            out.extend_from_slice(rest);
            break;
        } else {
            out.push(value_units[index]);
            index += 1;
        }
    }
    String::from_utf16(&out).unwrap_or_else(|_| String::from_utf16_lossy(&out))
}

fn trim_ascii_space(text: &str, end: TrimEnd) -> String {
    match end {
        TrimEnd::Both => text.trim_matches(' ').to_string(),
        TrimEnd::Left => text.trim_start_matches(' ').to_string(),
        TrimEnd::Right => text.trim_end_matches(' ').to_string(),
    }
}

fn substring_utf16(text: &str, raw_start: i64, len: i64) -> String {
    let Ok(raw_start) = i32::try_from(raw_start) else {
        return String::new();
    };
    let Ok(len) = i32::try_from(len) else {
        return String::new();
    };
    if len <= 0 {
        return String::new();
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let start = raw_start.wrapping_sub(1);
    let start = if start < 0 { 0 } else { start };
    let end = raw_start.wrapping_add(len).wrapping_sub(1);
    let str_len = i32::try_from(units.len()).unwrap_or(i32::MAX);
    let end = if end > str_len { str_len } else { end };
    if start >= end {
        return String::new();
    }
    let slice = &units[start as usize..end as usize];
    String::from_utf16(slice).unwrap_or_else(|_| String::from_utf16_lossy(slice))
}

fn count_of(table: &Table, row: usize, count: &CountArg) -> Option<i64> {
    match count {
        CountArg::Null => None,
        CountArg::Fixed(number) => Some(*number),
        CountArg::ColumnInt { index } => match &table.columns[*index][row] {
            Scalar::Null => None,
            value => as_i64(value).ok(),
        },
        CountArg::ColumnSymbol { index, keys } => {
            let value = &table.columns[*index][row];
            if value.is_null() {
                None
            } else {
                keys.iter()
                    .position(|key| key == value)
                    .map(|index| index as i64)
            }
        }
    }
}

fn take_chars(text: &str, count: i64, from_right: bool) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len() as i64;
    let mut count = count;
    if count < 0 {
        count += len;
    }
    if count < 0 {
        count = 0;
    }
    let count = usize::try_from(count)
        .unwrap_or(usize::MAX)
        .min(chars.len());
    let slice = if from_right {
        &chars[chars.len() - count..]
    } else {
        &chars[..count]
    };
    slice.iter().collect()
}

fn text_of(table: &Table, row: usize, arg: &ResolvedArg) -> Option<String> {
    match arg {
        ResolvedArg::Null => None,
        ResolvedArg::Text(text) => Some(text.clone()),
        ResolvedArg::Column { index, .. } => match &table.columns[*index][row] {
            Scalar::Text(text) => Some(text.clone()),
            Scalar::Char(0) => None,
            Scalar::Char(unit) => char::from_u32(u32::from(*unit)).map(|unit| unit.to_string()),
            _ => None,
        },
    }
}

fn text_length(table: &Table, row: usize, arg: &ResolvedArg, bytes: bool) -> i32 {
    let text = match arg {
        ResolvedArg::Null => None,
        ResolvedArg::Text(text) => Some(text.as_str()),
        ResolvedArg::Column { index, tag } => match &table.columns[*index][row] {
            Scalar::Text(text) => Some(text.as_str()),
            Scalar::Char(0) => None,
            Scalar::Char(unit) => {
                let Some(ch) = char::from_u32(u32::from(*unit)) else {
                    return -1;
                };
                return if bytes {
                    i32::try_from(ch.len_utf8()).unwrap_or(i32::MAX)
                } else {
                    1
                };
            }
            _ if *tag == qdb_types::TAG_CHAR => None,
            _ => None,
        },
    };
    match text {
        Some(text) if bytes => i32::try_from(text.len()).unwrap_or(i32::MAX),
        Some(text) => i32::try_from(text.chars().count()).unwrap_or(i32::MAX),
        None => -1,
    }
}

/// QuestDB `concat` stringifies each argument and skips a SQL `NULL` literal or a text null.
fn cast_concat(value: &Scalar, tag: i32) -> String {
    match tag {
        qdb_types::TAG_STRING | qdb_types::TAG_SYMBOL | qdb_types::TAG_VARCHAR => match value {
            Scalar::Text(text) => text.clone(),
            _ => String::new(),
        },
        qdb_types::TAG_CHAR => match value {
            Scalar::Char(unit) => char::from_u32(u32::from(*unit))
                .map(|unit| unit.to_string())
                .unwrap_or_default(),
            _ => String::new(),
        },
        qdb_types::TAG_BOOLEAN => match value {
            Scalar::Boolean(bit) => {
                if *bit {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => String::new(),
        },
        qdb_types::TAG_BYTE
        | qdb_types::TAG_SHORT
        | qdb_types::TAG_INT
        | qdb_types::TAG_LONG
        | qdb_types::TAG_DATE => match value {
            Scalar::Null => "null".to_string(),
            Scalar::Byte(number) => number.to_string(),
            Scalar::Short(number) => number.to_string(),
            Scalar::Int(number) => number.to_string(),
            Scalar::Long(number) => number.to_string(),
            Scalar::Date(number) => number.to_string(),
            _ => "null".to_string(),
        },
        qdb_types::TAG_FLOAT | qdb_types::TAG_DOUBLE => match value {
            Scalar::Float(number) => format_f64(f64::from(*number)),
            Scalar::Double(number) => format_f64(*number),
            _ => "NaN".to_string(),
        },
        qdb_types::TAG_TIMESTAMP => match value {
            Scalar::Timestamp(micros) => format_timestamp_micros(*micros),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

pub fn render_tsv(result: &QueryResult) -> String {
    let mut out = result.columns.join("\t");
    out.push('\n');
    for row in &result.rows {
        for (index, value) in row.iter().enumerate() {
            if index > 0 {
                out.push('\t');
            }
            out.push_str(&value.to_string());
        }
        out.push('\n');
    }
    out
}

fn execute_aggregate(
    table: &Table,
    query: &Query,
    aggregate: &AggregateExpr,
) -> Result<QueryResult, Error> {
    for order in &query.order_by {
        if !order.column.eq_ignore_ascii_case(&aggregate.label) {
            return Err(Error::new(format!(
                "ORDER BY expressions must appear in select list. Invalid column: {}",
                order.column
            )));
        }
    }
    let input = aggregate_input(table, aggregate)?;
    let mut accumulator = Accumulator::start(aggregate.func);
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        absorb(
            &mut accumulator,
            contribution(&table.columns, row, &input)?,
            aggregate.func,
        );
    }
    let mut row_ids = vec![0];
    if let Some(limit) = query.limit {
        if limit.lo < 0 && limit.hi > 0 && limit_follows_timestamp_order(table, query)? {
            return Err(Error::new("LIMIT <negative>, <positive> is not allowed"));
        }
        apply_limit(&mut row_ids, limit.lo, limit.hi);
    }
    let rows = if row_ids.is_empty() {
        Vec::new()
    } else {
        vec![vec![accumulator.finish()]]
    };
    Ok(QueryResult {
        columns: vec![aggregate.label.clone()],
        rows,
    })
}

fn execute_group(
    table: &Table,
    query: &Query,
    grouped: Option<&GroupedSelect>,
    aggregate: &AggregateExpr,
) -> Result<QueryResult, Error> {
    let group_name = query
        .group_by
        .as_deref()
        .ok_or_else(|| Error::new("unsupported: aggregate mixed with other select expressions"))?;
    let key_index = if let Some(grouped) = grouped {
        let select_index = table
            .column_index(&grouped.key)
            .ok_or_else(|| Error::new(format!("Invalid column: {}", grouped.key)))?;
        let group_index = table
            .column_index(group_name)
            .ok_or_else(|| Error::new(format!("Invalid column: {group_name}")))?;
        if select_index != group_index {
            return Err(Error::new(
                "column must appear in GROUP BY clause or aggregate function",
            ));
        }
        select_index
    } else {
        table
            .column_index(group_name)
            .ok_or_else(|| Error::new(format!("Invalid column: {group_name}")))?
    };
    for order in &query.order_by {
        let by_aggregate = order.column.eq_ignore_ascii_case(&aggregate.label);
        let by_key = order.column.eq_ignore_ascii_case(group_name);
        if !by_aggregate && !by_key {
            return Err(Error::new(format!(
                "ORDER BY expressions must appear in select list. Invalid column: {}",
                order.column
            )));
        }
    }
    let input = aggregate_input(table, aggregate)?;
    if aggregate.func == AggFunc::CountDistinct
        && matches!(
            input.tag,
            qdb_types::TAG_BOOLEAN | qdb_types::TAG_FLOAT | qdb_types::TAG_DOUBLE
        )
    {
        return Err(Error::new(format!(
            "there is no matching function `count_distinct` with the argument types: ({})",
            qdb_types::type_name(input.tag)
        )));
    }
    let value_tag = aggregate_order_tag(&input);
    let mut groups: Vec<(Scalar, Accumulator)> = Vec::new();
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        let key = table.columns[key_index][row].clone();
        let next = contribution(&table.columns, row, &input)?;
        if let Some(group) = groups.iter_mut().find(|(found, _)| found == &key) {
            absorb(&mut group.1, next, aggregate.func);
        } else {
            let mut accumulator = Accumulator::start(aggregate.func);
            absorb(&mut accumulator, next, aggregate.func);
            groups.push((key, accumulator));
        }
    }
    let tag = table.meta.columns[key_index].tag;
    groups.sort_by(|left, right| {
        for order in &query.order_by {
            let mut ordering = if order.column.eq_ignore_ascii_case(&aggregate.label) {
                compare_asc(&left.1.finish(), &right.1.finish(), value_tag)
            } else {
                compare_asc(&left.0, &right.0, tag)
            };
            if order.descending {
                ordering = ordering.reverse();
            }
            if ordering != Ordering::Equal {
                return ordering;
            }
        }
        Ordering::Equal
    });
    let mut row_ids: Vec<usize> = (0..groups.len()).collect();
    if let Some(limit) = query.limit {
        if limit.lo < 0 && limit.hi > 0 && limit_follows_timestamp_order(table, query)? {
            return Err(Error::new("LIMIT <negative>, <positive> is not allowed"));
        }
        apply_limit(&mut row_ids, limit.lo, limit.hi);
    }
    let columns = match grouped {
        Some(grouped) if grouped.key_at == 0 => {
            vec![grouped.key.clone(), aggregate.label.clone()]
        }
        Some(grouped) => vec![aggregate.label.clone(), grouped.key.clone()],
        None => vec![aggregate.label.clone()],
    };
    let rows = row_ids
        .into_iter()
        .map(|index| {
            let (key, accumulator) = &groups[index];
            let value = accumulator.finish();
            match grouped {
                Some(grouped) if grouped.key_at == 0 => vec![key.clone(), value],
                Some(_) => vec![value, key.clone()],
                None => vec![value],
            }
        })
        .collect();
    Ok(QueryResult { columns, rows })
}

struct AggColumn {
    index: Option<usize>,
    tag: i32,
    func: AggFunc,
    /// Symbol texts in key order. `sum` of a symbol adds that key.
    symbol_keys: Vec<Scalar>,
}

enum Accumulator {
    Count(i64),
    Value(Option<Scalar>),
    Average { total: Option<Scalar>, count: i64 },
    Distinct(Vec<Scalar>),
}

impl Accumulator {
    fn start(func: AggFunc) -> Self {
        match func {
            AggFunc::Count => Self::Count(0),
            AggFunc::Min | AggFunc::Max | AggFunc::Sum | AggFunc::First | AggFunc::Last => {
                Self::Value(None)
            }
            AggFunc::Avg => Self::Average {
                total: None,
                count: 0,
            },
            AggFunc::CountDistinct => Self::Distinct(Vec::new()),
        }
    }

    fn finish(&self) -> Scalar {
        match self {
            Self::Count(total) => Scalar::Long(*total),
            Self::Distinct(seen) => Scalar::Long(seen.len() as i64),
            Self::Value(Some(value)) => value.clone(),
            Self::Value(None) | Self::Average { count: 0, .. } => Scalar::Null,
            Self::Average {
                total: Some(total),
                count,
            } => average_of(total, *count),
            Self::Average { total: None, .. } => Scalar::Null,
        }
    }
}

fn aggregate_input(table: &Table, aggregate: &AggregateExpr) -> Result<AggColumn, Error> {
    let Some(name) = &aggregate.column else {
        return Ok(AggColumn {
            index: None,
            tag: qdb_types::TAG_LONG,
            func: aggregate.func,
            symbol_keys: Vec::new(),
        });
    };
    let index = table
        .column_index(name)
        .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
    let tag = table.meta.columns[index].tag;
    if matches!(aggregate.func, AggFunc::Count | AggFunc::Sum) && tag == qdb_types::TAG_BOOLEAN {
        let name = match aggregate.func {
            AggFunc::Sum => "sum",
            _ => "count",
        };
        return Err(Error::new(format!(
            "there is no matching function `{name}` with the argument types: (BOOLEAN)"
        )));
    }
    let symbol_keys =
        if matches!(aggregate.func, AggFunc::Sum | AggFunc::Avg) && tag == qdb_types::TAG_SYMBOL {
            let mut keys = Vec::new();
            for value in &table.columns[index] {
                if value.is_null() || keys.contains(value) {
                    continue;
                }
                keys.push(value.clone());
            }
            keys
        } else {
            Vec::new()
        };
    Ok(AggColumn {
        index: Some(index),
        tag,
        func: aggregate.func,
        symbol_keys,
    })
}

fn aggregate_order_tag(input: &AggColumn) -> i32 {
    match input.func {
        AggFunc::Count | AggFunc::CountDistinct => qdb_types::TAG_LONG,
        AggFunc::Sum if input.tag == qdb_types::TAG_FLOAT => qdb_types::TAG_FLOAT,
        AggFunc::Sum if input.tag == qdb_types::TAG_DOUBLE => qdb_types::TAG_DOUBLE,
        AggFunc::Sum => qdb_types::TAG_LONG,
        AggFunc::Avg => qdb_types::TAG_DOUBLE,
        AggFunc::Min | AggFunc::Max if input.tag == qdb_types::TAG_BOOLEAN => qdb_types::TAG_DOUBLE,
        AggFunc::Min | AggFunc::Max | AggFunc::First | AggFunc::Last => input.tag,
    }
}

fn contribution(
    columns: &[Vec<Scalar>],
    row: usize,
    input: &AggColumn,
) -> Result<Option<Scalar>, Error> {
    let Some(index) = input.index else {
        return Ok(Some(Scalar::Long(1)));
    };
    let value = &columns[index][row];
    let skipped =
        value.is_null() || (input.tag == qdb_types::TAG_CHAR && matches!(value, Scalar::Char(0)));
    match input.func {
        AggFunc::First | AggFunc::Last => Ok(Some(value.clone())),
        _ if skipped => Ok(None),
        AggFunc::Count => Ok(Some(Scalar::Long(1))),
        AggFunc::CountDistinct => Ok(Some(value.clone())),
        AggFunc::Min | AggFunc::Max if input.tag == qdb_types::TAG_BOOLEAN => {
            let Scalar::Boolean(bit) = value else {
                return Ok(None);
            };
            Ok(Some(Scalar::Double(if *bit { 1.0 } else { 0.0 })))
        }
        AggFunc::Min | AggFunc::Max => Ok(Some(value.clone())),
        AggFunc::Sum => sum_addend(value, input),
        AggFunc::Avg => avg_addend(value, input),
    }
}

fn sum_addend(value: &Scalar, input: &AggColumn) -> Result<Option<Scalar>, Error> {
    if value.is_null() || (input.tag == qdb_types::TAG_CHAR && matches!(value, Scalar::Char(0))) {
        return Ok(None);
    }
    match input.tag {
        qdb_types::TAG_BYTE | qdb_types::TAG_SHORT | qdb_types::TAG_INT | qdb_types::TAG_LONG => {
            Ok(Some(Scalar::Long(as_i64(value)?)))
        }
        qdb_types::TAG_DATE => match value {
            Scalar::Date(millis) => Ok(Some(Scalar::Long(*millis))),
            _ => Err(Error::new("internal: date column value")),
        },
        qdb_types::TAG_TIMESTAMP => match value {
            Scalar::Timestamp(micros) => Ok(Some(Scalar::Long(*micros))),
            _ => Err(Error::new("internal: timestamp column value")),
        },
        qdb_types::TAG_FLOAT => match value {
            Scalar::Float(number) => Ok(Some(Scalar::Float(*number))),
            _ => Err(Error::new("internal: float column value")),
        },
        qdb_types::TAG_DOUBLE => match value {
            Scalar::Double(number) => Ok(Some(Scalar::Double(*number))),
            _ => Err(Error::new("internal: double column value")),
        },
        qdb_types::TAG_SYMBOL => {
            let key = input
                .symbol_keys
                .iter()
                .position(|found| found == value)
                .ok_or_else(|| Error::new("internal: symbol key"))?;
            Ok(Some(Scalar::Long(i64::try_from(key).unwrap_or(i64::MAX))))
        }
        qdb_types::TAG_CHAR => Err(Error::new(format!(
            "inconvertible value: {} [CHAR -> SHORT]",
            scalar_text(value)
        ))),
        qdb_types::TAG_STRING => Err(Error::new(format!(
            "inconvertible value: `{}` [STRING -> DOUBLE]",
            scalar_text(value)
        ))),
        qdb_types::TAG_VARCHAR => Err(Error::new(format!(
            "inconvertible value: `{}` [VARCHAR -> DOUBLE]",
            scalar_text(value)
        ))),
        _ => Err(Error::new(format!(
            "unsupported: sum of column type {}",
            input.tag
        ))),
    }
}

fn scalar_text(value: &Scalar) -> String {
    match value {
        Scalar::Text(text) => text.clone(),
        Scalar::Char(unit) => char::from_u32(u32::from(*unit))
            .map(|unit| unit.to_string())
            .unwrap_or_default(),
        _ => value.to_string(),
    }
}

fn avg_addend(value: &Scalar, input: &AggColumn) -> Result<Option<Scalar>, Error> {
    if input.tag == qdb_types::TAG_BOOLEAN {
        let Scalar::Boolean(bit) = value else {
            return Ok(None);
        };
        return Ok(Some(Scalar::Double(if *bit { 1.0 } else { 0.0 })));
    }
    sum_addend(value, input)
}

fn average_of(total: &Scalar, count: i64) -> Scalar {
    let sum = match total {
        Scalar::Long(value) => *value as f64,
        Scalar::Float(value) => f64::from(*value),
        Scalar::Double(value) => *value,
        _ => return Scalar::Null,
    };
    Scalar::Double(sum / count as f64)
}

fn add_sum(left: &Scalar, right: &Scalar) -> Scalar {
    match (left, right) {
        (Scalar::Long(left), Scalar::Long(right)) => Scalar::Long(left.wrapping_add(*right)),
        (Scalar::Float(left), Scalar::Float(right)) => Scalar::Float(*left + *right),
        (Scalar::Double(left), Scalar::Double(right)) => Scalar::Double(*left + *right),
        _ => left.clone(),
    }
}

fn absorb(accumulator: &mut Accumulator, next: Option<Scalar>, func: AggFunc) {
    let Some(next) = next else {
        return;
    };
    match accumulator {
        Accumulator::Count(total) => *total += 1,
        Accumulator::Distinct(seen) => {
            if !seen.contains(&next) {
                seen.push(next);
            }
        }
        Accumulator::Value(current) if func == AggFunc::First => {
            if current.is_none() {
                *current = Some(next);
            }
        }
        Accumulator::Value(current) if func == AggFunc::Last => {
            *current = Some(next);
        }
        Accumulator::Value(current) if func == AggFunc::Sum => {
            *current = Some(match current.take() {
                None => next,
                Some(kept) => add_sum(&kept, &next),
            });
        }
        Accumulator::Average { total, count } => {
            *count += 1;
            *total = Some(match total.take() {
                None => next,
                Some(kept) => add_sum(&kept, &next),
            });
        }
        Accumulator::Value(current) => match current {
            None => *current = Some(next),
            Some(kept) => {
                let replace = matches!(
                    (func, agg_cmp(kept, &next)),
                    (AggFunc::Min, Ordering::Greater) | (AggFunc::Max, Ordering::Less)
                );
                if replace {
                    *current = Some(next);
                }
            }
        },
    }
}

/// `min` and `max` treat `-0.0` and `0.0` as equal and keep the value already chosen.
fn agg_cmp(left: &Scalar, right: &Scalar) -> Ordering {
    match (left, right) {
        (Scalar::Float(left), Scalar::Float(right)) => {
            left.partial_cmp(right).unwrap_or(Ordering::Equal)
        }
        (Scalar::Double(left), Scalar::Double(right)) => {
            left.partial_cmp(right).unwrap_or(Ordering::Equal)
        }
        _ => compare_values(left, right),
    }
}

fn limit_follows_timestamp_order(table: &Table, query: &Query) -> Result<bool, Error> {
    // A single designated-timestamp key uses the ordered scan. Any other key
    // list, including `ts` plus another column, uses the sort path.
    let [order] = query.order_by.as_slice() else {
        return Ok(query.order_by.is_empty());
    };
    let Some(index) = table.column_index(&order.column) else {
        return Ok(false);
    };
    Ok(index as i32 == table.meta.timestamp_index)
}

fn apply_limit(row_ids: &mut Vec<usize>, mut lo: i64, mut hi: i64) {
    // `ORDER BY` a non-timestamp column keeps the last `|lo|` rows when `hi` is positive.
    if lo < 0 && hi > 0 {
        hi = 0;
    }
    if lo > hi && (lo ^ hi) >= 0 {
        std::mem::swap(&mut lo, &mut hi);
    }
    let len = i64::try_from(row_ids.len()).unwrap_or(i64::MAX);
    let (start, end) = if lo >= 0 && hi >= 0 {
        (lo, hi)
    } else if lo < 0 {
        (len.saturating_add(lo).max(0), len.saturating_add(hi))
    } else {
        (lo, len.saturating_add(hi))
    };
    let start = start.max(0);
    let end = end.min(len);
    if start >= end {
        row_ids.clear();
        return;
    }
    let start = usize::try_from(start).unwrap_or(0);
    let end = usize::try_from(end).unwrap_or(row_ids.len());
    row_ids.drain(end..);
    row_ids.drain(..start);
}

fn row_matches(table: &Table, row: usize, filter: &Filter) -> Result<bool, Error> {
    let index = table
        .column_index(&filter.column)
        .ok_or_else(|| Error::new(format!("unknown column `{}`", filter.column)))?;
    let left = &table.columns[index][row];
    let tag = table.meta.columns[index].tag;
    sql_cmp(left, tag, filter.op, &filter.literal)
}

fn sql_cmp(left: &Scalar, tag: i32, op: CmpOp, literal: &Scalar) -> Result<bool, Error> {
    if matches!(literal, Scalar::Null) {
        return Ok(match op {
            CmpOp::Eq => left.is_null(),
            CmpOp::NotEq => !left.is_null(),
            CmpOp::Lt | CmpOp::Gt | CmpOp::LtEq | CmpOp::GtEq => false,
        });
    }
    if left.is_null() {
        return Ok(false);
    }
    let ordering = match tag {
        qdb_types::TAG_BOOLEAN => cmp_bool(left, literal)?,
        qdb_types::TAG_BYTE | qdb_types::TAG_SHORT | qdb_types::TAG_INT | qdb_types::TAG_LONG => {
            cmp_i64(as_i64(left)?, as_i64(literal)?)?
        }
        qdb_types::TAG_FLOAT | qdb_types::TAG_DOUBLE => cmp_f64(as_f64(left)?, as_f64(literal)?)?,
        qdb_types::TAG_CHAR => cmp_text(&char_text(left)?, &expect_text(literal)?)?,
        qdb_types::TAG_STRING | qdb_types::TAG_SYMBOL | qdb_types::TAG_VARCHAR => {
            cmp_text(&expect_text(left)?, &expect_text(literal)?)?
        }
        qdb_types::TAG_TIMESTAMP => {
            let right = timestamp_from_literal(literal)?
                .ok_or_else(|| Error::new("NULL timestamp already handled"))?;
            let Scalar::Timestamp(left) = left else {
                return Err(Error::new("internal: timestamp column value"));
            };
            left.cmp(&right)
        }
        qdb_types::TAG_DATE => {
            let right = date_from_literal(literal)?
                .ok_or_else(|| Error::new("NULL date already handled"))?;
            let Scalar::Date(left) = left else {
                return Err(Error::new("internal: date column value"));
            };
            left.cmp(&right)
        }
        other => {
            return Err(Error::new(format!(
                "unsupported comparison on type tag {other}"
            )))
        }
    };
    Ok(match op {
        CmpOp::Eq => ordering == Ordering::Equal,
        CmpOp::NotEq => ordering != Ordering::Equal,
        CmpOp::Lt => ordering == Ordering::Less,
        CmpOp::Gt => ordering == Ordering::Greater,
        CmpOp::LtEq => ordering != Ordering::Greater,
        CmpOp::GtEq => ordering != Ordering::Less,
    })
}

fn cmp_bool(left: &Scalar, literal: &Scalar) -> Result<Ordering, Error> {
    let Scalar::Boolean(left) = left else {
        return Err(Error::new("internal: boolean column value"));
    };
    let Scalar::Boolean(right) = literal else {
        return Err(Error::new("unsupported boolean comparison"));
    };
    Ok(left.cmp(right))
}

fn cmp_i64(left: i64, right: i64) -> Result<Ordering, Error> {
    Ok(left.cmp(&right))
}

fn cmp_f64(left: f64, right: f64) -> Result<Ordering, Error> {
    left.partial_cmp(&right)
        .ok_or_else(|| Error::new("unsupported numeric comparison"))
}

fn cmp_text(left: &str, right: &str) -> Result<Ordering, Error> {
    Ok(left.cmp(right))
}

fn as_i64(value: &Scalar) -> Result<i64, Error> {
    match value {
        Scalar::Byte(v) => Ok(i64::from(*v)),
        Scalar::Short(v) => Ok(i64::from(*v)),
        Scalar::Int(v) => Ok(i64::from(*v)),
        Scalar::Long(v) => Ok(*v),
        _ => Err(Error::new(
            "unsupported numeric literal for an integer column",
        )),
    }
}

fn as_f64(value: &Scalar) -> Result<f64, Error> {
    match value {
        Scalar::Byte(v) => Ok(f64::from(*v)),
        Scalar::Short(v) => Ok(f64::from(*v)),
        Scalar::Int(v) => Ok(f64::from(*v)),
        Scalar::Long(v) => Ok(*v as f64),
        Scalar::Float(v) => Ok(f64::from(*v)),
        Scalar::Double(v) => Ok(*v),
        _ => Err(Error::new(
            "unsupported numeric literal for a floating column",
        )),
    }
}

fn expect_text(value: &Scalar) -> Result<String, Error> {
    match value {
        Scalar::Text(text) => Ok(text.clone()),
        Scalar::Char(unit) => char_from_unit(*unit).map(|c| c.to_string()),
        _ => Err(Error::new("unsupported text comparison")),
    }
}

fn char_text(value: &Scalar) -> Result<String, Error> {
    match value {
        Scalar::Char(0) => Ok(String::new()),
        Scalar::Char(unit) => char_from_unit(*unit).map(|c| c.to_string()),
        _ => Err(Error::new("internal: char column value")),
    }
}

fn char_from_unit(unit: u16) -> Result<char, Error> {
    char::from_u32(u32::from(unit)).ok_or_else(|| Error::new(format!("invalid UTF-16 char {unit}")))
}

/// Ascending order from QuestDB 10.1.0. Integer, string, symbol, date and
/// timestamp nulls sort first. Float and double nulls sort last, above every
/// finite value, and `-0.0` sorts before `0.0`.
fn compare_asc(left: &Scalar, right: &Scalar, tag: i32) -> Ordering {
    let nulls_last = matches!(tag, qdb_types::TAG_FLOAT | qdb_types::TAG_DOUBLE);
    match (left.is_null(), right.is_null()) {
        (true, true) => Ordering::Equal,
        (true, false) => {
            if nulls_last {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (false, true) => {
            if nulls_last {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (false, false) => compare_values(left, right),
    }
}

fn compare_values(left: &Scalar, right: &Scalar) -> Ordering {
    match (left, right) {
        (Scalar::Boolean(a), Scalar::Boolean(b)) => a.cmp(b),
        (Scalar::Byte(a), Scalar::Byte(b)) => a.cmp(b),
        (Scalar::Short(a), Scalar::Short(b)) => a.cmp(b),
        (Scalar::Char(a), Scalar::Char(b)) => a.cmp(b),
        (Scalar::Int(a), Scalar::Int(b)) => a.cmp(b),
        (Scalar::Long(a), Scalar::Long(b)) => a.cmp(b),
        (Scalar::Float(a), Scalar::Float(b)) => a.total_cmp(b),
        (Scalar::Double(a), Scalar::Double(b)) => a.total_cmp(b),
        (Scalar::Date(a), Scalar::Date(b)) => a.cmp(b),
        (Scalar::Timestamp(a), Scalar::Timestamp(b)) => a.cmp(b),
        (Scalar::Text(a), Scalar::Text(b)) => a.cmp(b),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1")
    }

    #[test]
    fn select_star_and_filters_match_reference_rows() {
        let all = execute(&root(), "SELECT * FROM sensors ORDER BY ts").unwrap();
        assert_eq!(all.rows.len(), 4);
        assert_eq!(all.columns[0], "ts");
        assert_eq!(all.rows[0][1], Scalar::Long(1));
        assert_eq!(all.rows[1][9], Scalar::Null);
        assert_eq!(all.rows[2][9], Scalar::Text(String::new()));
        assert_eq!(all.rows[3][10], Scalar::Text("東京".to_string()));
        assert_eq!(all.rows[3][3], Scalar::Double(-0.0));

        let filtered = execute(
            &root(),
            "SELECT id, note, location, tag FROM sensors WHERE id > 1",
        )
        .unwrap();
        assert_eq!(filtered.columns, vec!["id", "note", "location", "tag"]);
        assert_eq!(filtered.rows.len(), 1);
        assert_eq!(filtered.rows[0][0], Scalar::Long(2));
        assert!(filtered.rows[0][1].is_null());

        let madrid = execute(
            &root(),
            "SELECT tag FROM sensors WHERE location = 'madrid' ORDER BY ts",
        )
        .unwrap();
        assert_eq!(
            madrid.rows,
            vec![
                vec![Scalar::Text("v1".to_string())],
                vec![Scalar::Text("héllo".to_string())],
            ]
        );

        let trues = execute(
            &root(),
            "SELECT id FROM sensors WHERE ok = true ORDER BY ts",
        )
        .unwrap();
        assert_eq!(trues.rows, vec![vec![Scalar::Long(1)], vec![Scalar::Null]]);

        let alpha = execute(&root(), "SELECT note FROM sensors WHERE note = 'alpha'").unwrap();
        assert_eq!(alpha.rows, vec![vec![Scalar::Text("alpha".to_string())]]);

        let null_id = execute(&root(), "SELECT id FROM sensors WHERE id = NULL").unwrap();
        assert_eq!(null_id.rows, vec![vec![Scalar::Null]]);

        let not_null = execute(
            &root(),
            "SELECT id FROM sensors WHERE id != NULL ORDER BY id",
        )
        .unwrap();
        assert_eq!(
            not_null.rows,
            vec![
                vec![Scalar::Long(-1)],
                vec![Scalar::Long(1)],
                vec![Scalar::Long(2)]
            ]
        );

        let beta = execute(&root(), "SELECT ch FROM sensors WHERE ch = 'β'").unwrap();
        assert_eq!(
            beta.rows,
            vec![vec![Scalar::Char(u16::try_from(u32::from('β')).unwrap())]]
        );

        let negative = execute(
            &root(),
            "SELECT score, reading FROM sensors WHERE score < 0",
        )
        .unwrap();
        assert_eq!(
            negative.rows,
            vec![vec![Scalar::Float(-1.5), Scalar::Double(-0.0)]]
        );

        let ordered = execute(&root(), "SELECT id FROM sensors ORDER BY id").unwrap();
        assert_eq!(
            ordered.rows,
            vec![
                vec![Scalar::Null],
                vec![Scalar::Long(-1)],
                vec![Scalar::Long(1)],
                vec![Scalar::Long(2)]
            ]
        );
    }

    #[test]
    fn day_partitions_match_captured_exec() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            ("SELECT * FROM days ORDER BY ts", "expected/select_star.tsv"),
            (
                "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts",
                "expected/madrid.tsv",
            ),
            (
                "SELECT id FROM days WHERE id = NULL",
                "expected/id_null.tsv",
            ),
            ("SELECT note FROM days ORDER BY ts", "expected/notes.tsv"),
        ];
        for (sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
    }

    #[test]
    fn limit_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 2",
                "expected/limit_order_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY ts LIMIT 1",
                "expected/limit_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors LIMIT 2",
                "expected/limit_plain.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors WHERE id != NULL ORDER BY id LIMIT 1",
                "expected/limit_filtered.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT -1",
                "expected/limit_neg.tsv",
            ),
            (
                sensors,
                "SELECT id FROM sensors ORDER BY id LIMIT 0",
                "expected/limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY ts LIMIT 2",
                "expected/limit_ts.tsv",
            ),
            (
                days.clone(),
                "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 1",
                "expected/limit_madrid.tsv",
            ),
            (
                days.clone(),
                "SELECT note FROM days ORDER BY ts LIMIT 3",
                "expected/limit_notes.tsv",
            ),
            (
                days,
                "SELECT id FROM days ORDER BY ts LIMIT -2",
                "expected/limit_neg.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
    }

    #[test]
    fn limit_range_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 1, 3",
                "expected/limit_range.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 3, 1",
                "expected/limit_range.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT -3, -1",
                "expected/limit_range.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 1, -1",
                "expected/limit_range.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 2, 10",
                "expected/limit_plain.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 2, 2",
                "expected/limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT ,2",
                "expected/limit_order_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT 2,",
                "expected/limit_order_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT -2,",
                "expected/limit_plain.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT ,-2",
                "expected/limit_order_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id LIMIT -2, 1",
                "expected/limit_plain.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY ts LIMIT 1, 3",
                "expected/limit_range.tsv",
            ),
            (
                days.clone(),
                "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts LIMIT 0, 1",
                "expected/limit_madrid.tsv",
            ),
            (
                days,
                "SELECT id FROM days ORDER BY ts LIMIT -4, -2",
                "expected/limit_ts.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let rejected = [
            (sensors, "SELECT id FROM sensors LIMIT -2, 1"),
            (
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1"),
                "SELECT id FROM sensors ORDER BY ts LIMIT -2, 1",
            ),
            (
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2"),
                "SELECT id FROM days ORDER BY ts LIMIT -2, 1",
            ),
        ];
        for (root, sql) in rejected {
            let err = execute(&root, sql).unwrap_err();
            assert!(
                err.message.contains("LIMIT <negative>, <positive>"),
                "{sql}: {}",
                err.message
            );
        }
    }

    #[test]
    fn descending_order_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id DESC",
                "expected/desc_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id DESC LIMIT 2",
                "expected/desc_id_limit.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY id DESC LIMIT -1",
                "expected/desc_id_last.tsv",
            ),
            (
                sensors.clone(),
                "SELECT reading FROM sensors ORDER BY reading",
                "expected/asc_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT reading FROM sensors ORDER BY reading DESC",
                "expected/desc_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT note FROM sensors ORDER BY note DESC",
                "expected/desc_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location FROM sensors ORDER BY location DESC",
                "expected/desc_location.tsv",
            ),
            (
                sensors,
                "SELECT score FROM sensors ORDER BY score DESC",
                "expected/desc_score.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY ts DESC",
                "expected/desc_ts.tsv",
            ),
            (
                days.clone(),
                "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY ts DESC",
                "expected/desc_madrid.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY id DESC",
                "expected/desc_id.tsv",
            ),
            (
                days,
                "SELECT note FROM days ORDER BY note DESC",
                "expected/desc_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
    }

    #[test]
    fn multi_column_order_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location, id",
                "expected/order_location_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location, id DESC",
                "expected/order_location_id_desc.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location DESC, id",
                "expected/order_location_desc_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location DESC, id DESC",
                "expected/order_location_desc_id_desc.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY reading, id",
                "expected/order_reading_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY reading DESC, id",
                "expected/order_reading_desc_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location",
                "expected/order_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id FROM sensors ORDER BY location, id LIMIT 2",
                "expected/order_location_id_limit.tsv",
            ),
            (
                sensors,
                "SELECT id FROM sensors ORDER BY ts, id DESC LIMIT -2, 1",
                "expected/order_ts_id_mixed.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY location, ts DESC",
                "expected/order_location_ts_desc.tsv",
            ),
            (
                days.clone(),
                "SELECT id FROM days ORDER BY location DESC, id",
                "expected/order_location_desc_id.tsv",
            ),
            (
                days,
                "SELECT id, note FROM days WHERE location = 'madrid' ORDER BY note DESC, id",
                "expected/order_madrid_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
    }

    #[test]
    fn count_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT count() FROM sensors",
                "expected/count_all.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(*) FROM sensors",
                "expected/count_all.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(id) FROM sensors",
                "expected/count_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(note) FROM sensors",
                "expected/count_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(reading) FROM sensors",
                "expected/count_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(ch) FROM sensors",
                "expected/count_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count(b) FROM sensors",
                "expected/count_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count() FROM sensors WHERE id = NULL",
                "expected/count_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count() FROM sensors WHERE location = 'madrid'",
                "expected/count_two.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count() FROM sensors WHERE id > 100",
                "expected/count_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count() FROM sensors LIMIT 0",
                "expected/count_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT count() FROM days",
                "expected/count_all.tsv",
            ),
            (
                days.clone(),
                "SELECT count(id) FROM days",
                "expected/count_id.tsv",
            ),
            (
                days.clone(),
                "SELECT count(note) FROM days",
                "expected/count_note.tsv",
            ),
            (
                days,
                "SELECT count(location) FROM days WHERE location = 'madrid'",
                "expected/count_madrid.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT count() FROM sensors ORDER BY id").unwrap_err();
        assert!(err.message.contains("ORDER BY"), "{}", err.message);
    }

    fn sorted_body(text: &str) -> (String, Vec<String>) {
        let mut lines = text.lines();
        let header = lines.next().unwrap_or("").to_string();
        let mut body: Vec<String> = lines.map(ToOwned::to_owned).collect();
        body.sort();
        (header, body)
    }

    #[test]
    fn group_by_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let unordered = [
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id",
                "expected/group_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors WHERE id > 100 GROUP BY id",
                "expected/group_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count() FROM sensors WHERE location = 'madrid' GROUP BY location",
                "expected/group_madrid.tsv",
            ),
        ];
        for (root, sql, file) in unordered {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(
                sorted_body(&render_tsv(&result)),
                sorted_body(&expected),
                "{sql}"
            );
        }
        let ordered = [
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id ORDER BY id",
                "expected/group_id_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id ORDER BY id DESC",
                "expected/group_id_desc.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 2",
                "expected/group_id_limit.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT -2, 1",
                "expected/group_id_tail.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count() FROM sensors GROUP BY id ORDER BY id LIMIT 0",
                "expected/group_id_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count(note) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_id_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count(ch) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_id_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count(b) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_id_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count() FROM sensors GROUP BY location ORDER BY location",
                "expected/group_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT note, count() FROM sensors GROUP BY note ORDER BY note",
                "expected/group_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT reading, count() FROM sensors GROUP BY reading ORDER BY reading",
                "expected/group_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ch, count() FROM sensors GROUP BY ch ORDER BY ch",
                "expected/group_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ID, COUNT() FROM sensors GROUP BY ID ORDER BY ID",
                "expected/group_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count() FROM sensors GROUP BY location ORDER BY location",
                "expected/group_count_location.tsv",
            ),
            (
                days.clone(),
                "SELECT id, count() FROM days GROUP BY id ORDER BY id",
                "expected/group_id.tsv",
            ),
            (
                days.clone(),
                "SELECT location, count() FROM days GROUP BY location ORDER BY location",
                "expected/group_location.tsv",
            ),
            (
                days.clone(),
                "SELECT location, count(id) FROM days WHERE location = 'madrid' GROUP BY location",
                "expected/group_madrid.tsv",
            ),
            (
                days,
                "SELECT id, count() FROM days WHERE id = NULL GROUP BY id",
                "expected/group_null_id.tsv",
            ),
        ];
        for (root, sql, file) in ordered {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let by_count = execute(
            &sensors,
            "SELECT location, count() FROM sensors GROUP BY location ORDER BY count() DESC",
        )
        .unwrap();
        assert_eq!(by_count.rows[0][0], Scalar::Text("madrid".to_string()));
        assert_eq!(by_count.rows[0][1], Scalar::Long(2));
        let swapped = execute(
            &sensors,
            "SELECT count(), id FROM sensors GROUP BY id ORDER BY id",
        )
        .unwrap();
        assert_eq!(
            swapped.columns,
            vec!["count()".to_string(), "id".to_string()]
        );
        assert_eq!(swapped.rows[0], vec![Scalar::Long(1), Scalar::Null]);
        let err = execute(
            &sensors,
            "SELECT id, count() FROM sensors GROUP BY location",
        )
        .unwrap_err();
        assert!(err.message.contains("GROUP BY"), "{}", err.message);
        let err =
            execute(&sensors, "SELECT id, count() FROM sensors GROUP BY missing").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT id, count() FROM sensors").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = execute(
            &sensors,
            "SELECT id, count() FROM sensors GROUP BY id, note",
        )
        .unwrap_err();
        assert!(err.message.contains("multiple GROUP BY"), "{}", err.message);
    }

    #[test]
    fn min_max_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT min(id) FROM sensors",
                "expected/min_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(id) FROM sensors",
                "expected/max_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(note) FROM sensors",
                "expected/min_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(note) FROM sensors",
                "expected/max_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(reading) FROM sensors",
                "expected/min_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(reading) FROM sensors",
                "expected/max_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(id) FROM sensors WHERE id > 100",
                "expected/min_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(ok) FROM sensors",
                "expected/min_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(ok) FROM sensors",
                "expected/max_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(ch) FROM sensors",
                "expected/min_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(ch) FROM sensors WHERE ch = ''",
                "expected/min_ch_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(b) FROM sensors",
                "expected/min_b.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(code) FROM sensors",
                "expected/min_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(flags) FROM sensors",
                "expected/min_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(flags) FROM sensors",
                "expected/max_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(ts) FROM sensors",
                "expected/min_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(ts) FROM sensors",
                "expected/max_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(day) FROM sensors",
                "expected/min_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(day) FROM sensors",
                "expected/max_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(tag) FROM sensors",
                "expected/min_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(tag) FROM sensors",
                "expected/max_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(location) FROM sensors",
                "expected/min_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(location) FROM sensors",
                "expected/max_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(score) FROM sensors",
                "expected/min_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT max(score) FROM sensors",
                "expected/max_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT MIN(ID) FROM sensors",
                "expected/min_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT min(id) FROM sensors LIMIT 0",
                "expected/min_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_min_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, min(id) FROM sensors GROUP BY location ORDER BY min(id)",
                "expected/group_min_by_value.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, max(note) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_max_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, max(reading) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_max_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, min(reading) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_min_reading.tsv",
            ),
            (
                days.clone(),
                "SELECT min(id) FROM days",
                "expected/min_id.tsv",
            ),
            (
                days.clone(),
                "SELECT min(note) FROM days",
                "expected/min_note.tsv",
            ),
            (
                days.clone(),
                "SELECT max(note) FROM days",
                "expected/max_note.tsv",
            ),
            (
                days.clone(),
                "SELECT min(id) FROM days WHERE id = NULL",
                "expected/min_null.tsv",
            ),
            (
                days.clone(),
                "SELECT location, min(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_min_id.tsv",
            ),
            (
                days,
                "SELECT location, max(note) FROM days GROUP BY location ORDER BY location",
                "expected/group_max_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT min() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT min(*) FROM sensors").unwrap_err();
        assert!(err.message.contains("too few arguments"), "{}", err.message);
        let err = execute(&sensors, "SELECT min(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT min(id), max(id) FROM sensors").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
    }

    #[test]
    fn sum_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT sum(id) FROM sensors",
                "expected/sum_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(reading) FROM sensors",
                "expected/sum_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(score) FROM sensors",
                "expected/sum_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(b) FROM sensors",
                "expected/sum_b.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(code) FROM sensors",
                "expected/sum_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(flags) FROM sensors",
                "expected/sum_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(b) FROM sensors WHERE b = 0",
                "expected/sum_b_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(id) FROM sensors WHERE id > 100",
                "expected/sum_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(reading) FROM sensors WHERE id = -1",
                "expected/sum_neg_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(reading) FROM sensors WHERE reading = 0.0",
                "expected/sum_zeros.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(ts) FROM sensors",
                "expected/sum_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(day) FROM sensors",
                "expected/sum_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(location) FROM sensors",
                "expected/sum_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SUM(ID) FROM sensors",
                "expected/sum_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(id) FROM sensors LIMIT 0",
                "expected/sum_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, sum(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_sum_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sum(id) FROM sensors GROUP BY location ORDER BY sum(id)",
                "expected/group_sum_by_value.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, sum(reading) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_sum_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, sum(score) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_sum_score.tsv",
            ),
            (
                days.clone(),
                "SELECT sum(id) FROM days",
                "expected/sum_id.tsv",
            ),
            (
                days.clone(),
                "SELECT sum(ts) FROM days",
                "expected/sum_ts.tsv",
            ),
            (
                days.clone(),
                "SELECT sum(location) FROM days",
                "expected/sum_location.tsv",
            ),
            (
                days.clone(),
                "SELECT sum(id) FROM days WHERE id = NULL",
                "expected/sum_null.tsv",
            ),
            (
                days,
                "SELECT location, sum(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_sum_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT sum() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sum(*) FROM sensors").unwrap_err();
        assert!(err.message.contains("too few arguments"), "{}", err.message);
        let err = execute(&sensors, "SELECT sum(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("no matching function"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sum(ch) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: A [CHAR -> SHORT]"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sum(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: `alpha` [STRING -> DOUBLE]"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sum(tag) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: `v1` [VARCHAR -> DOUBLE]"),
            "{}",
            err.message
        );
    }

    #[test]
    fn avg_matches_captured_exec() {
        let sensors = root();
        let days =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                &sensors,
                "SELECT avg(id) FROM sensors",
                "expected/avg_id.tsv",
            ),
            (
                &sensors,
                "SELECT avg(reading) FROM sensors",
                "expected/avg_reading.tsv",
            ),
            (
                &sensors,
                "SELECT avg(score) FROM sensors",
                "expected/avg_score.tsv",
            ),
            (&sensors, "SELECT avg(b) FROM sensors", "expected/avg_b.tsv"),
            (
                &sensors,
                "SELECT avg(code) FROM sensors",
                "expected/avg_code.tsv",
            ),
            (
                &sensors,
                "SELECT avg(flags) FROM sensors",
                "expected/avg_flags.tsv",
            ),
            (
                &sensors,
                "SELECT avg(ok) FROM sensors",
                "expected/avg_ok.tsv",
            ),
            (
                &sensors,
                "SELECT avg(id) FROM sensors WHERE id > 100",
                "expected/avg_null.tsv",
            ),
            (
                &sensors,
                "SELECT avg(reading) FROM sensors WHERE id = -1",
                "expected/avg_neg_zero.tsv",
            ),
            (
                &sensors,
                "SELECT avg(reading) FROM sensors WHERE reading = 0.0",
                "expected/avg_zeros.tsv",
            ),
            (
                &sensors,
                "SELECT avg(b) FROM sensors WHERE b = 0",
                "expected/avg_b_zero.tsv",
            ),
            (
                &sensors,
                "SELECT avg(ts) FROM sensors",
                "expected/avg_ts.tsv",
            ),
            (
                &sensors,
                "SELECT avg(day) FROM sensors",
                "expected/avg_day.tsv",
            ),
            (
                &sensors,
                "SELECT avg(location) FROM sensors",
                "expected/avg_location.tsv",
            ),
            (
                &sensors,
                "SELECT AVG(ID) FROM sensors",
                "expected/avg_case.tsv",
            ),
            (
                &sensors,
                "SELECT avg(id) FROM sensors LIMIT 0",
                "expected/avg_limit_zero.tsv",
            ),
            (
                &sensors,
                "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_avg_id.tsv",
            ),
            (
                &sensors,
                "SELECT id, avg(reading) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_avg_reading.tsv",
            ),
            (
                &sensors,
                "SELECT location, avg(id) FROM sensors GROUP BY location ORDER BY avg(id)",
                "expected/group_avg_by_value.tsv",
            ),
            (&days, "SELECT avg(id) FROM days", "expected/avg_id.tsv"),
            (
                &days,
                "SELECT avg(id) FROM days WHERE id = NULL",
                "expected/avg_null.tsv",
            ),
            (&days, "SELECT avg(ts) FROM days", "expected/avg_ts.tsv"),
            (
                &days,
                "SELECT location, avg(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_avg_id.tsv",
            ),
            (
                &days,
                "SELECT location, avg(id) FROM days GROUP BY location ORDER BY avg(id)",
                "expected/group_avg_by_value.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT avg() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT avg(*) FROM sensors").unwrap_err();
        assert!(err.message.contains("too few arguments"), "{}", err.message);
        let err = execute(&sensors, "SELECT avg(ch) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: A [CHAR -> SHORT]"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT avg(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: `alpha` [STRING -> DOUBLE]"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT avg(tag) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("inconvertible value: `v1` [VARCHAR -> DOUBLE]"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT avg(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn first_last_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT first(id) FROM sensors",
                "expected/first_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(id) FROM sensors",
                "expected/last_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(reading) FROM sensors",
                "expected/first_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(reading) FROM sensors",
                "expected/last_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(ok) FROM sensors",
                "expected/first_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(ok) FROM sensors",
                "expected/last_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(note) FROM sensors",
                "expected/first_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(note) FROM sensors",
                "expected/last_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(ch) FROM sensors",
                "expected/first_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(tag) FROM sensors",
                "expected/last_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(ts) FROM sensors",
                "expected/first_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(day) FROM sensors",
                "expected/last_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(location) FROM sensors",
                "expected/first_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(location) FROM sensors",
                "expected/last_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(id) FROM sensors WHERE id > 100",
                "expected/first_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(reading) FROM sensors WHERE id = 2",
                "expected/first_reading_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(note) FROM sensors WHERE note = ''",
                "expected/last_empty_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(ch) FROM sensors WHERE ch = ''",
                "expected/first_empty_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT FIRST(ID) FROM sensors",
                "expected/first_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT first(id) FROM sensors LIMIT 0",
                "expected/first_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT last(score) FROM sensors",
                "expected/last_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_first_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, last(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_last_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, first(reading) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_first_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, first(id) FROM sensors GROUP BY location ORDER BY first(id)",
                "expected/group_first_by_value.tsv",
            ),
            (
                days.clone(),
                "SELECT first(id) FROM days",
                "expected/first_id.tsv",
            ),
            (
                days.clone(),
                "SELECT last(id) FROM days",
                "expected/last_id.tsv",
            ),
            (
                days.clone(),
                "SELECT first(note) FROM days",
                "expected/first_note.tsv",
            ),
            (
                days.clone(),
                "SELECT last(note) FROM days",
                "expected/last_note.tsv",
            ),
            (
                days.clone(),
                "SELECT first(ts) FROM days",
                "expected/first_ts.tsv",
            ),
            (
                days.clone(),
                "SELECT last(ts) FROM days",
                "expected/last_ts.tsv",
            ),
            (
                days.clone(),
                "SELECT first(id) FROM days WHERE id = NULL",
                "expected/first_null.tsv",
            ),
            (
                days.clone(),
                "SELECT location, first(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_first_id.tsv",
            ),
            (
                days.clone(),
                "SELECT location, last(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_last_id.tsv",
            ),
            (
                days.clone(),
                "SELECT location, last(note) FROM days GROUP BY location ORDER BY last(note)",
                "expected/group_last_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT first() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT last() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT first(*) FROM sensors").unwrap_err();
        assert!(err.message.contains("too few arguments"), "{}", err.message);
        let err = execute(&sensors, "SELECT first(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn count_distinct_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT count_distinct(id) FROM sensors",
                "expected/cd_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(reading) FROM sensors",
                "expected/cd_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(score) FROM sensors",
                "expected/cd_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(ok) FROM sensors",
                "expected/cd_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(ch) FROM sensors",
                "expected/cd_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(note) FROM sensors",
                "expected/cd_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(location) FROM sensors",
                "expected/cd_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(b) FROM sensors",
                "expected/cd_b.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(flags) FROM sensors",
                "expected/cd_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(day) FROM sensors",
                "expected/cd_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(ts) FROM sensors",
                "expected/cd_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(id) FROM sensors WHERE id > 100",
                "expected/cd_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(id) FROM sensors WHERE id = NULL",
                "expected/cd_null_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(note) FROM sensors WHERE note = ''",
                "expected/cd_empty_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(ch) FROM sensors WHERE ch = ''",
                "expected/cd_empty_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(reading) FROM sensors WHERE reading = 0.0",
                "expected/cd_zeros.tsv",
            ),
            (
                sensors.clone(),
                "SELECT COUNT_DISTINCT(ID) FROM sensors",
                "expected/cd_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT count_distinct(id) FROM sensors LIMIT 0",
                "expected/cd_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count_distinct(id) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_cd_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_cd_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count_distinct(ch) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_cd_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count_distinct(flags) FROM sensors GROUP BY location ORDER BY location",
                "expected/group_cd_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, count_distinct(note) FROM sensors GROUP BY id ORDER BY id",
                "expected/group_cd_by_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT location, count_distinct(note) FROM sensors GROUP BY location ORDER BY count_distinct(note)",
                "expected/group_cd_by_value.tsv",
            ),
            (
                days.clone(),
                "SELECT count_distinct(id) FROM days",
                "expected/cd_id.tsv",
            ),
            (
                days.clone(),
                "SELECT count_distinct(note) FROM days",
                "expected/cd_note.tsv",
            ),
            (
                days.clone(),
                "SELECT count_distinct(location) FROM days",
                "expected/cd_location.tsv",
            ),
            (
                days.clone(),
                "SELECT count_distinct(id) FROM days WHERE id = NULL",
                "expected/cd_null.tsv",
            ),
            (
                days.clone(),
                "SELECT location, count_distinct(id) FROM days GROUP BY location ORDER BY location",
                "expected/group_cd_id.tsv",
            ),
            (
                days.clone(),
                "SELECT location, count_distinct(note) FROM days GROUP BY location ORDER BY count_distinct(note)",
                "expected/group_cd_by_value.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT count_distinct() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT count_distinct(*) FROM sensors").unwrap_err();
        assert!(err.message.contains("too few arguments"), "{}", err.message);
        let err = execute(&sensors, "SELECT count_distinct(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(
            &sensors,
            "SELECT location, count_distinct(reading) FROM sensors GROUP BY location",
        )
        .unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `count_distinct` with the argument types: (DOUBLE)"
            ),
            "{}",
            err.message
        );
        let err = execute(
            &sensors,
            "SELECT count_distinct(score) FROM sensors GROUP BY location",
        )
        .unwrap_err();
        assert!(err.message.contains("(FLOAT)"), "{}", err.message);
        let err = execute(
            &sensors,
            "SELECT count_distinct(ok) FROM sensors GROUP BY location",
        )
        .unwrap_err();
        assert!(err.message.contains("(BOOLEAN)"), "{}", err.message);
    }

    #[test]
    fn concat_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT concat(note, location) FROM sensors",
                "expected/concat_note_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, '!') FROM sensors",
                "expected/concat_bang.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat('pre-', location) FROM sensors",
                "expected/concat_prefix.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, null) FROM sensors",
                "expected/concat_null_lit.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(ch, note) FROM sensors",
                "expected/concat_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(id, note) FROM sensors",
                "expected/concat_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(reading, '') FROM sensors",
                "expected/concat_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(day, '') FROM sensors",
                "expected/concat_day.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(ts, '') FROM sensors",
                "expected/concat_ts.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(ok, '') FROM sensors",
                "expected/concat_ok.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(flags, '') FROM sensors",
                "expected/concat_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT CONCAT(NOTE, LOCATION) FROM sensors",
                "expected/concat_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat( note , location ) FROM sensors",
                "expected/concat_space.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat('it''s', '!') FROM sensors LIMIT 1",
                "expected/concat_quote.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note) FROM sensors",
                "expected/concat_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, location, tag) FROM sensors",
                "expected/concat_three.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, location) FROM sensors ORDER BY concat(note, location)",
                "expected/concat_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, location) FROM sensors ORDER BY note",
                "expected/concat_order_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, concat(note, location) FROM sensors ORDER BY concat(note, location)",
                "expected/concat_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, location) FROM sensors LIMIT 0",
                "expected/concat_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT concat(note, location) FROM sensors WHERE note = ''",
                "expected/concat_where_empty.tsv",
            ),
            (
                days.clone(),
                "SELECT concat(note, location) FROM days",
                "expected/concat_note_location.tsv",
            ),
            (
                days.clone(),
                "SELECT concat(location, note) FROM days ORDER BY ts",
                "expected/concat_order_ts.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT concat() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("no arguments provided"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT concat(missing, note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn length_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT length(note) FROM sensors",
                "expected/length_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(location) FROM sensors",
                "expected/length_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(tag) FROM sensors",
                "expected/length_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(ch) FROM sensors",
                "expected/length_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length('') FROM sensors LIMIT 1",
                "expected/length_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(null) FROM sensors LIMIT 1",
                "expected/length_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length('βeta') FROM sensors LIMIT 1",
                "expected/length_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length('東京') FROM sensors LIMIT 1",
                "expected/length_tokyo.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LENGTH(NOTE) FROM sensors",
                "expected/length_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(note) FROM sensors WHERE note = ''",
                "expected/length_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(note) FROM sensors ORDER BY length(note)",
                "expected/length_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, length(note) FROM sensors ORDER BY length(note)",
                "expected/length_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length(note) FROM sensors LIMIT 0",
                "expected/length_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT length(note) FROM days",
                "expected/length_note.tsv",
            ),
            (
                days.clone(),
                "SELECT length(location) FROM days ORDER BY ts",
                "expected/length_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT length() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length(note, location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `length` with the argument types: (STRING, SYMBOL)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `length` with the argument types: (LONG)"),
            "{}",
            err.message
        );
    }

    #[test]
    fn lower_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT lower(note) FROM sensors",
                "expected/lower_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(location) FROM sensors",
                "expected/lower_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(tag) FROM sensors",
                "expected/lower_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(ch) FROM sensors",
                "expected/lower_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower('BETA') FROM sensors LIMIT 1",
                "expected/lower_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower('Héllo') FROM sensors LIMIT 1",
                "expected/lower_hello.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower('東京') FROM sensors LIMIT 1",
                "expected/lower_tokyo.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower('') FROM sensors LIMIT 1",
                "expected/lower_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(null) FROM sensors LIMIT 1",
                "expected/lower_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LOWER(NOTE) FROM sensors",
                "expected/lower_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(note) FROM sensors ORDER BY lower(note)",
                "expected/lower_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, lower(note) FROM sensors ORDER BY lower(note)",
                "expected/lower_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(note) FROM sensors WHERE note = ''",
                "expected/lower_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lower(note) FROM sensors LIMIT 0",
                "expected/lower_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT lower(note) FROM days",
                "expected/lower_note.tsv",
            ),
            (
                days.clone(),
                "SELECT lower(location) FROM days ORDER BY ts",
                "expected/lower_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT lower() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lower(note, location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `lower` with the argument types: (STRING, SYMBOL)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lower(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lower(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `lower` with the argument types: (LONG)"),
            "{}",
            err.message
        );
    }

    #[test]
    fn upper_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT upper(note) FROM sensors",
                "expected/upper_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(location) FROM sensors",
                "expected/upper_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(tag) FROM sensors",
                "expected/upper_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(ch) FROM sensors",
                "expected/upper_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper('βeta') FROM sensors LIMIT 1",
                "expected/upper_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper('') FROM sensors LIMIT 1",
                "expected/upper_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(null) FROM sensors LIMIT 1",
                "expected/upper_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT UPPER(NOTE) FROM sensors",
                "expected/upper_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(note) FROM sensors ORDER BY upper(note)",
                "expected/upper_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, upper(note) FROM sensors ORDER BY upper(note)",
                "expected/upper_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(note) FROM sensors WHERE note = ''",
                "expected/upper_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT upper(note) FROM sensors LIMIT 0",
                "expected/upper_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT upper(note) FROM days",
                "expected/upper_note.tsv",
            ),
            (
                days.clone(),
                "SELECT upper(location) FROM days ORDER BY ts",
                "expected/upper_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT upper() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT upper(note, location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `upper` with the argument types: (STRING, SYMBOL)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT upper(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT upper(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `upper` with the argument types: (LONG)"),
            "{}",
            err.message
        );
    }

    #[test]
    fn left_right_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT left(note, 2) FROM sensors",
                "expected/left_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT right(note, 2) FROM sensors",
                "expected/right_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(location, 3) FROM sensors",
                "expected/left_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT right(tag, 4) FROM sensors",
                "expected/right_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(ch, 1) FROM sensors",
                "expected/left_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, 0) FROM sensors",
                "expected/left_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, 10) FROM sensors",
                "expected/left_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, -1) FROM sensors",
                "expected/left_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT right(note, -2) FROM sensors",
                "expected/right_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left('βeta', 1) FROM sensors LIMIT 1",
                "expected/left_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT right('東京', 1) FROM sensors LIMIT 1",
                "expected/right_tokyo.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(null, 2) FROM sensors LIMIT 1",
                "expected/left_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left('', 2) FROM sensors LIMIT 1",
                "expected/left_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LEFT(NOTE, 1) FROM sensors",
                "expected/left_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, 2) FROM sensors ORDER BY left(note, 2)",
                "expected/left_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, left(note, 1) FROM sensors ORDER BY left(note, 1)",
                "expected/left_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT right(note, 2) FROM sensors WHERE note = ''",
                "expected/right_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, 2) FROM sensors LIMIT 0",
                "expected/left_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, location) FROM sensors",
                "expected/left_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT left(note, b) FROM sensors",
                "expected/left_byte.tsv",
            ),
            (
                days.clone(),
                "SELECT left(note, 2) FROM days",
                "expected/left_note.tsv",
            ),
            (
                days.clone(),
                "SELECT right(note, 2) FROM days",
                "expected/right_note.tsv",
            ),
            (
                days.clone(),
                "SELECT left(location, 3) FROM days ORDER BY ts",
                "expected/left_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT left() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT left(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `left` with the argument types: (STRING)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT left(id, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `left` with the argument types: (LONG, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT left(note, 1.5) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `left` with the argument types: (STRING, DOUBLE)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT left(missing, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn substring_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT substring(note, 1, 2) FROM sensors",
                "expected/sub_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 0, 2) FROM sensors",
                "expected/sub_zero_start.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 1, 10) FROM sensors",
                "expected/sub_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, -1, 4) FROM sensors",
                "expected/sub_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, -2, 2) FROM sensors",
                "expected/sub_before.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 0, 0) FROM sensors",
                "expected/sub_zero_len.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 6, 2) FROM sensors",
                "expected/sub_past.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring('βeta', 1, 2) FROM sensors LIMIT 1",
                "expected/sub_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(null, 1, 2) FROM sensors LIMIT 1",
                "expected/sub_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring('', 1, 2) FROM sensors LIMIT 1",
                "expected/sub_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SUBSTRING(NOTE, 1, 1) FROM sensors",
                "expected/sub_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(ch, 1, 1) FROM sensors",
                "expected/sub_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(tag, 2, 3) FROM sensors",
                "expected/sub_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(location, 1, 3) FROM sensors",
                "expected/sub_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 2, 4) FROM sensors ORDER BY substring(note, 2, 4)",
                "expected/sub_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, substring(note, 1, 1) FROM sensors ORDER BY substring(note, 1, 1)",
                "expected/sub_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 1, 2) FROM sensors WHERE note = ''",
                "expected/sub_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, 1, 2) FROM sensors LIMIT 0",
                "expected/sub_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, location, 2) FROM sensors",
                "expected/sub_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, flags, 2) FROM sensors",
                "expected/sub_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, null, 2) FROM sensors",
                "expected/sub_null_start.tsv",
            ),
            (
                sensors.clone(),
                "SELECT substring(note, code, 1) FROM sensors",
                "expected/sub_code.tsv",
            ),
            (
                days.clone(),
                "SELECT substring(note, 2, 4) FROM days",
                "expected/sub_note.tsv",
            ),
            (
                days.clone(),
                "SELECT substring(note, -1, 4) FROM days",
                "expected/sub_neg.tsv",
            ),
            (
                days.clone(),
                "SELECT substring(location, 2, 3) FROM days ORDER BY ts",
                "expected/sub_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT substring() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `substring` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(id, 1, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `substring` with the argument types: (LONG, INT, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(note, 1.5, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `substring` with the argument types: (STRING, DOUBLE, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(note, id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `substring` with the argument types: (STRING, LONG, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(missing, 1, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(note, 1, -1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("negative substring length is not allowed"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT substring(note, 1, b) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("negative substring length is not allowed"),
            "{}",
            err.message
        );
    }

    #[test]
    fn trim_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT trim(note) FROM sensors",
                "expected/trim_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(location) FROM sensors",
                "expected/trim_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(tag) FROM sensors",
                "expected/trim_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(ch) FROM sensors",
                "expected/trim_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim('  BETA  ') FROM sensors LIMIT 1",
                "expected/trim_spaces.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(' a b ') FROM sensors LIMIT 1",
                "expected/trim_inner.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim('βeta') FROM sensors LIMIT 1",
                "expected/trim_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim('') FROM sensors LIMIT 1",
                "expected/trim_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(null) FROM sensors LIMIT 1",
                "expected/trim_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim('  ') FROM sensors LIMIT 1",
                "expected/trim_blank.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim('\t ab \t') FROM sensors LIMIT 1",
                "expected/trim_tab.tsv",
            ),
            (
                sensors.clone(),
                "SELECT TRIM(NOTE) FROM sensors",
                "expected/trim_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(note) FROM sensors ORDER BY trim(note)",
                "expected/trim_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, trim(note) FROM sensors ORDER BY trim(note)",
                "expected/trim_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(note) FROM sensors WHERE note = ''",
                "expected/trim_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT trim(note) FROM sensors LIMIT 0",
                "expected/trim_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT trim(note) FROM days",
                "expected/trim_note.tsv",
            ),
            (
                days.clone(),
                "SELECT trim(location) FROM days ORDER BY ts",
                "expected/trim_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT trim() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT trim(note, location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `trim` with the argument types: (STRING, SYMBOL)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT trim(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `trim` with the argument types: (LONG)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT trim(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn ltrim_rtrim_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT ltrim('  a b  ') FROM sensors LIMIT 1",
                "expected/ltrim_spaces.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim('  a b  ') FROM sensors LIMIT 1",
                "expected/rtrim_spaces.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim('  ') FROM sensors LIMIT 1",
                "expected/ltrim_blank.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim(null) FROM sensors LIMIT 1",
                "expected/rtrim_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim('\t ab ') FROM sensors LIMIT 1",
                "expected/rtrim_tab.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim(note) FROM sensors",
                "expected/ltrim_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim(note) FROM sensors",
                "expected/rtrim_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim(location) FROM sensors",
                "expected/ltrim_location.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim(tag) FROM sensors",
                "expected/rtrim_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim(ch) FROM sensors",
                "expected/ltrim_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LTRIM(NOTE) FROM sensors",
                "expected/ltrim_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim(note) FROM sensors ORDER BY ltrim(note)",
                "expected/ltrim_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, rtrim(note) FROM sensors ORDER BY rtrim(note)",
                "expected/rtrim_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ltrim(note) FROM sensors WHERE note = ''",
                "expected/ltrim_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rtrim(note) FROM sensors LIMIT 0",
                "expected/rtrim_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT ltrim(note) FROM days",
                "expected/ltrim_note.tsv",
            ),
            (
                days.clone(),
                "SELECT rtrim(location) FROM days ORDER BY ts",
                "expected/rtrim_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT ltrim() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ltrim(note, location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `ltrim` with the argument types: (STRING, SYMBOL)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ltrim(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `ltrim` with the argument types: (LONG)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT rtrim(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn replace_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT replace(note, 'a', 'X') FROM sensors",
                "expected/rep_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(note, 'ta', '') FROM sensors",
                "expected/rep_drop.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('βeta', 'β', 'B') FROM sensors LIMIT 1",
                "expected/rep_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(null, 'a', 'b') FROM sensors LIMIT 1",
                "expected/rep_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('ab', null, 'c') FROM sensors LIMIT 1",
                "expected/rep_null_old.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('ab', 'a', null) FROM sensors LIMIT 1",
                "expected/rep_null_new.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('', 'a', 'b') FROM sensors LIMIT 1",
                "expected/rep_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('aaa', 'a', 'bb') FROM sensors LIMIT 1",
                "expected/rep_grow.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('aaa', 'aa', 'b') FROM sensors LIMIT 1",
                "expected/rep_overlap.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('abc', '', 'x') FROM sensors LIMIT 1",
                "expected/rep_empty_old.tsv",
            ),
            (
                sensors.clone(),
                "SELECT REPLACE(NOTE, 'a', 'A') FROM sensors",
                "expected/rep_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')",
                "expected/rep_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, replace(note, 'a', 'X') FROM sensors ORDER BY replace(note, 'a', 'X')",
                "expected/rep_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(note, 'a', 'X') FROM sensors WHERE note = ''",
                "expected/rep_where_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(note, 'a', 'X') FROM sensors LIMIT 0",
                "expected/rep_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(note, 'a', location) FROM sensors",
                "expected/rep_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('ab', location, 'x') FROM sensors",
                "expected/rep_const_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace('', location, 'x') FROM sensors",
                "expected/rep_empty_value.tsv",
            ),
            (
                sensors.clone(),
                "SELECT replace(tag, '0', 'x') FROM sensors",
                "expected/rep_tag.tsv",
            ),
            (
                days.clone(),
                "SELECT replace(note, 'a', 'X') FROM days",
                "expected/rep_note.tsv",
            ),
            (
                days.clone(),
                "SELECT replace(location, 'a', 'A') FROM days ORDER BY ts",
                "expected/rep_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT replace() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT replace(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `replace` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT replace(note, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `replace` with the argument types: (STRING, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT replace(note, 'ab') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `replace` with the argument types: (STRING, STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT replace(id, '1', '2') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `replace` with the argument types: (LONG, CHAR, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT replace(missing, 'a', 'b') FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn starts_with_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT starts_with(note, 'a') FROM sensors",
                "expected/sw_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, 'β') FROM sensors",
                "expected/sw_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, '') FROM sensors",
                "expected/sw_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, null) FROM sensors",
                "expected/sw_null_prefix.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(null, 'a') FROM sensors LIMIT 1",
                "expected/sw_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with('', 'a') FROM sensors LIMIT 1",
                "expected/sw_empty_text.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with('alpha', 'alp') FROM sensors LIMIT 1",
                "expected/sw_prefix.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with('alpha', 'alphaextra') FROM sensors LIMIT 1",
                "expected/sw_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with('Alpha', 'a') FROM sensors LIMIT 1",
                "expected/sw_case_fold.tsv",
            ),
            (
                sensors.clone(),
                "SELECT STARTS_WITH(NOTE, 'a') FROM sensors",
                "expected/sw_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')",
                "expected/sw_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, starts_with(note, 'a') FROM sensors ORDER BY starts_with(note, 'a')",
                "expected/sw_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, '') FROM sensors WHERE note = ''",
                "expected/sw_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(note, 'a') FROM sensors LIMIT 0",
                "expected/sw_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(ch, 'A') FROM sensors",
                "expected/sw_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(tag, '0') FROM sensors",
                "expected/sw_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT starts_with(location, '東') FROM sensors",
                "expected/sw_location.tsv",
            ),
            (
                days.clone(),
                "SELECT starts_with(note, 'al') FROM days",
                "expected/sw_note.tsv",
            ),
            (
                days.clone(),
                "SELECT starts_with(location, 'mad') FROM days ORDER BY ts",
                "expected/sw_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT starts_with() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT starts_with(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `starts_with` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT starts_with(id, '1') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `starts_with` with the argument types: (LONG, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT starts_with(missing, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn strpos_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT strpos(note, 'a') FROM sensors",
                "expected/sp_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, 'ph') FROM sensors",
                "expected/sp_ph.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, 'z') FROM sensors",
                "expected/sp_miss.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, '') FROM sensors",
                "expected/sp_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, null) FROM sensors",
                "expected/sp_null_needle.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(null, 'a') FROM sensors LIMIT 1",
                "expected/sp_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos('', 'a') FROM sensors LIMIT 1",
                "expected/sp_empty_text.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos('βeta', 'β') FROM sensors LIMIT 1",
                "expected/sp_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos('aaa', 'aa') FROM sensors LIMIT 1",
                "expected/sp_overlap.tsv",
            ),
            (
                sensors.clone(),
                "SELECT STRPOS(NOTE, 'a') FROM sensors",
                "expected/sp_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')",
                "expected/sp_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, strpos(note, 'a') FROM sensors ORDER BY strpos(note, 'a')",
                "expected/sp_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, 'a') FROM sensors WHERE note = ''",
                "expected/sp_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, 'a') FROM sensors LIMIT 0",
                "expected/sp_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(note, location) FROM sensors",
                "expected/sp_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(tag, '0') FROM sensors",
                "expected/sp_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT strpos(ch, 'A') FROM sensors",
                "expected/sp_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT position(note, 'a') FROM sensors",
                "expected/sp_position.tsv",
            ),
            (
                days.clone(),
                "SELECT strpos(note, 'a') FROM days",
                "expected/sp_note.tsv",
            ),
            (
                days.clone(),
                "SELECT strpos(location, 'id') FROM days ORDER BY ts",
                "expected/sp_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT strpos() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT strpos(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `strpos` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT strpos(id, '1') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `strpos` with the argument types: (LONG, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT position(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `position` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT strpos(missing, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn split_part_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 1) FROM sensors",
                "expected/part_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 2) FROM sensors",
                "expected/part_second.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 3) FROM sensors",
                "expected/part_third.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('a-b-c', '-', 2) FROM sensors LIMIT 1",
                "expected/part_dash.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('a-b-c', '-', -1) FROM sensors LIMIT 1",
                "expected/part_last.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('a-b-c', '-', 4) FROM sensors LIMIT 1",
                "expected/part_past.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('a--b', '-', 2) FROM sensors LIMIT 1",
                "expected/part_double.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, '', 1) FROM sensors",
                "expected/part_empty_delim.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('abc', '', -1) FROM sensors LIMIT 1",
                "expected/part_empty_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(null, 'a', 1) FROM sensors LIMIT 1",
                "expected/part_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part('ab', null, 1) FROM sensors LIMIT 1",
                "expected/part_null_delim.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', null) FROM sensors",
                "expected/part_null_index.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SPLIT_PART(NOTE, 'a', 1) FROM sensors",
                "expected/part_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)",
                "expected/part_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, split_part(note, 'a', 1) FROM sensors ORDER BY split_part(note, 'a', 1)",
                "expected/part_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 1) FROM sensors WHERE note = ''",
                "expected/part_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT split_part(note, 'a', 1) FROM sensors LIMIT 0",
                "expected/part_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT split_part(note, 'a', 1) FROM days",
                "expected/part_note.tsv",
            ),
            (
                days.clone(),
                "SELECT split_part(location, 'a', 2) FROM days ORDER BY ts",
                "expected/part_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT split_part() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT split_part(note, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `split_part` with the argument types: (STRING, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT split_part(id, 'a', 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `split_part` with the argument types: (LONG, CHAR, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT split_part(note, 'a', id) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `split_part` with the argument types: (STRING, CHAR, LONG)"
            ),
            "{}",
            err.message
        );
        let err =
            execute(&sensors, "SELECT split_part(note, 'a', flags) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("index must be either a constant expression or a placeholder"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT split_part('a-b-c', '-', 0) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("field position must not be zero"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT split_part(missing, 'a', 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn case_aliases_match_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT to_lowercase(note) FROM sensors",
                "expected/tl_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_uppercase(note) FROM sensors",
                "expected/tu_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_uppercase('β') FROM sensors LIMIT 1",
                "expected/tu_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT TO_LOWERCASE(NOTE) FROM sensors",
                "expected/tl_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)",
                "expected/tl_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_uppercase(note) FROM sensors ORDER BY to_uppercase(note)",
                "expected/tu_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, to_lowercase(note) FROM sensors ORDER BY to_lowercase(note)",
                "expected/tl_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_lowercase(null) FROM sensors LIMIT 1",
                "expected/tl_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_lowercase(note) FROM sensors WHERE note = ''",
                "expected/tl_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_lowercase(note) FROM sensors LIMIT 0",
                "expected/tl_limit_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_uppercase(ch) FROM sensors",
                "expected/tu_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT to_uppercase(tag) FROM sensors",
                "expected/tu_tag.tsv",
            ),
            (
                days.clone(),
                "SELECT to_lowercase(note) FROM days",
                "expected/tl_note.tsv",
            ),
            (
                days.clone(),
                "SELECT to_uppercase(note) FROM days ORDER BY ts",
                "expected/tu_note.tsv",
            ),
            (
                days.clone(),
                "SELECT to_lowercase(location) FROM days ORDER BY ts",
                "expected/tl_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT to_lowercase() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `to_lowercase` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT to_lowercase(note, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `to_lowercase` with the argument types: (STRING, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT to_lowercase(id) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `to_lowercase` with the argument types: (LONG)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT to_uppercase(id) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `to_uppercase` with the argument types: (LONG)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT to_lowercase(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn pad_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT lpad(note, 6, 'x') FROM sensors",
                "expected/lp_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rpad(note, 6, 'x') FROM sensors",
                "expected/rp_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', 5, 'xy') FROM sensors LIMIT 1",
                "expected/lp_fill.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('abc', 2, 'x') FROM sensors LIMIT 1",
                "expected/lp_cut.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rpad('abc', 2, 'x') FROM sensors LIMIT 1",
                "expected/rp_cut.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', 0, 'x') FROM sensors LIMIT 1",
                "expected/lp_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', -1, 'x') FROM sensors LIMIT 1",
                "expected/lp_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(null, 3, 'x') FROM sensors LIMIT 1",
                "expected/lp_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', null, 'x') FROM sensors LIMIT 1",
                "expected/lp_null_len.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', 3, null) FROM sensors LIMIT 1",
                "expected/lp_null_fill.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad('ab', 3, '') FROM sensors LIMIT 1",
                "expected/lp_empty_fill.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(note, 3) FROM sensors",
                "expected/lp_spaces.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rpad(note, 3) FROM sensors",
                "expected/rp_spaces.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(note, flags, 'x') FROM sensors",
                "expected/lp_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(note, b) FROM sensors",
                "expected/lp_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(ch, 3, 'x') FROM sensors",
                "expected/lp_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(note, 6, location) FROM sensors",
                "expected/lp_symbol.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LPAD(NOTE, 6, 'x') FROM sensors",
                "expected/lp_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT rpad(note, 6, 'x') FROM sensors ORDER BY rpad(note, 6, 'x')",
                "expected/rp_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT lpad(note, 6, 'x') FROM sensors LIMIT 0",
                "expected/lp_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT rpad(location, 8, '.') FROM days ORDER BY ts",
                "expected/rp_location.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT lpad() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `lpad` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lpad(id, 3, 'x') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `lpad` with the argument types: (LONG, INT, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lpad(note, id, 'x') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `lpad` with the argument types: (STRING, LONG, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT lpad(missing, 3, 'x') FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT rpad() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `rpad` requires arguments"),
            "{}",
            err.message
        );
    }

    #[test]
    fn quote_ident_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT quote_ident(note) FROM sensors",
                "expected/qi_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('a\"b') FROM sensors LIMIT 1",
                "expected/qi_quote.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('a b') FROM sensors LIMIT 1",
                "expected/qi_space.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('a-b') FROM sensors LIMIT 1",
                "expected/qi_dash.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('Alpha') FROM sensors LIMIT 1",
                "expected/qi_case_text.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('a$b') FROM sensors LIMIT 1",
                "expected/qi_dollar.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('1a') FROM sensors LIMIT 1",
                "expected/qi_digit.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(null) FROM sensors LIMIT 1",
                "expected/qi_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident('') FROM sensors LIMIT 1",
                "expected/qi_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(ch) FROM sensors",
                "expected/qi_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(tag) FROM sensors",
                "expected/qi_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT QUOTE_IDENT(NOTE) FROM sensors",
                "expected/qi_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(note) FROM sensors ORDER BY quote_ident(note)",
                "expected/qi_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, quote_ident(note) FROM sensors ORDER BY quote_ident(note)",
                "expected/qi_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(note) FROM sensors WHERE note = ''",
                "expected/qi_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT quote_ident(note) FROM sensors LIMIT 0",
                "expected/qi_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT quote_ident(location) FROM days ORDER BY ts",
                "expected/qi_location.tsv",
            ),
            (
                days.clone(),
                "SELECT quote_ident(note) FROM days",
                "expected/qi_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT quote_ident() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `quote_ident` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT quote_ident(note, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `quote_ident` with the argument types: (STRING, CHAR)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT quote_ident(id) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `quote_ident` with the argument types: (LONG)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT quote_ident(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn length_bytes_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT length_bytes(note) FROM sensors",
                "expected/lb_note.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(tag) FROM sensors",
                "expected/lb_tag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes('β') FROM sensors LIMIT 1",
                "expected/lb_beta.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes('東京') FROM sensors LIMIT 1",
                "expected/lb_tokyo.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(null) FROM sensors LIMIT 1",
                "expected/lb_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes('') FROM sensors LIMIT 1",
                "expected/lb_empty.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(ch) FROM sensors",
                "expected/lb_ch.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LENGTH_BYTES(NOTE) FROM sensors",
                "expected/lb_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(note) FROM sensors ORDER BY length_bytes(note)",
                "expected/lb_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, length_bytes(note) FROM sensors ORDER BY length_bytes(note)",
                "expected/lb_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(note) FROM sensors WHERE note = ''",
                "expected/lb_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT length_bytes(note) FROM sensors LIMIT 0",
                "expected/lb_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT length_bytes(note) FROM days",
                "expected/lb_note.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT length_bytes() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `length_bytes` requires arguments: length_bytes(VARCHAR)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length_bytes(note, 'a') FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "wrong number of arguments for function `length_bytes`; expected: 1, provided: 2"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length_bytes(id) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `length_bytes` at #1 expected: VARCHAR, actual: LONG"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length_bytes(location) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `length_bytes` at #1 expected: VARCHAR, actual: SYMBOL"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT length_bytes(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn size_pretty_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT size_pretty(id) FROM sensors",
                "expected/sz_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(0) FROM sensors LIMIT 1",
                "expected/sz_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(1024) FROM sensors LIMIT 1",
                "expected/sz_kib.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(1500) FROM sensors LIMIT 1",
                "expected/sz_round.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(1048576) FROM sensors LIMIT 1",
                "expected/sz_mib.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(1023) FROM sensors LIMIT 1",
                "expected/sz_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(-1024) FROM sensors LIMIT 1",
                "expected/sz_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(9223372036854775807) FROM sensors LIMIT 1",
                "expected/sz_max.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(null) FROM sensors LIMIT 1",
                "expected/sz_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(flags) FROM sensors",
                "expected/sz_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(b) FROM sensors",
                "expected/sz_byte_col.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(code) FROM sensors",
                "expected/sz_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SIZE_PRETTY(ID) FROM sensors",
                "expected/sz_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(id) FROM sensors ORDER BY size_pretty(id)",
                "expected/sz_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, size_pretty(id) FROM sensors ORDER BY size_pretty(id)",
                "expected/sz_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(id) FROM sensors WHERE id = 1",
                "expected/sz_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT size_pretty(id) FROM sensors LIMIT 0",
                "expected/sz_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT size_pretty(id) FROM days ORDER BY ts",
                "expected/sz_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT size_pretty() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `size_pretty` requires arguments: size_pretty(LONG)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT size_pretty(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "wrong number of arguments for function `size_pretty`; expected: 1, provided: 2"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT size_pretty(1.5) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `size_pretty` at #1 expected: LONG, actual: DOUBLE"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT size_pretty(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `size_pretty` at #1 expected: LONG, actual: STRING"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT size_pretty(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn abs_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT abs(id) FROM sensors",
                "expected/abs_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(reading) FROM sensors",
                "expected/abs_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(score) FROM sensors",
                "expected/abs_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(b) FROM sensors",
                "expected/abs_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(code) FROM sensors",
                "expected/abs_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(flags) FROM sensors",
                "expected/abs_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(-1.5) FROM sensors LIMIT 1",
                "expected/abs_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(-1) FROM sensors LIMIT 1",
                "expected/abs_int.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(null) FROM sensors LIMIT 1",
                "expected/abs_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ABS(ID) FROM sensors",
                "expected/abs_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(id) FROM sensors ORDER BY abs(id)",
                "expected/abs_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, abs(id) FROM sensors ORDER BY abs(id)",
                "expected/abs_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(reading) FROM sensors ORDER BY abs(reading)",
                "expected/abs_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(id) FROM sensors WHERE id = -1",
                "expected/abs_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT abs(id) FROM sensors LIMIT 0",
                "expected/abs_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT abs(id) FROM days ORDER BY ts",
                "expected/abs_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT abs() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `abs` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT abs(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `abs` with the argument types: (LONG, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT abs(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `abs` with the argument types: (STRING)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT abs(ok) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `abs` with the argument types: (BOOLEAN)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT abs(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn round_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT round(reading) FROM sensors",
                "expected/rd_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(score) FROM sensors",
                "expected/rd_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(id) FROM sensors",
                "expected/rd_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(flags) FROM sensors",
                "expected/rd_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(b) FROM sensors",
                "expected/rd_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(1.5) FROM sensors LIMIT 1",
                "expected/rd_half.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(2.5) FROM sensors LIMIT 1",
                "expected/rd_half2.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(-1.5) FROM sensors LIMIT 1",
                "expected/rd_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(reading, 1) FROM sensors",
                "expected/rd_scale.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(1.25, 1) FROM sensors LIMIT 1",
                "expected/rd_quarter.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(-1.25, 1) FROM sensors LIMIT 1",
                "expected/rd_neg_scale.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(15, -1) FROM sensors LIMIT 1",
                "expected/rd_tens.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(null) FROM sensors LIMIT 1",
                "expected/rd_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(reading, null) FROM sensors",
                "expected/rd_null_scale.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(1.5, 20) FROM sensors LIMIT 1",
                "expected/rd_far.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ROUND(READING) FROM sensors",
                "expected/rd_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(id) FROM sensors ORDER BY round(id)",
                "expected/rd_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, round(id) FROM sensors ORDER BY round(id)",
                "expected/rd_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(id) FROM sensors WHERE id = -1",
                "expected/rd_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT round(id) FROM sensors LIMIT 0",
                "expected/rd_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT round(id) FROM days ORDER BY ts",
                "expected/rd_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT round() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `round` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT round(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `round` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT round(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `round` with the argument types: (BOOLEAN)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT round(id, 1, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `round` with the argument types: (LONG, INT, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT round(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn floor_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT floor(reading) FROM sensors",
                "expected/fl_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(score) FROM sensors",
                "expected/fl_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(id) FROM sensors",
                "expected/fl_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(flags) FROM sensors",
                "expected/fl_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(b) FROM sensors",
                "expected/fl_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(code) FROM sensors",
                "expected/fl_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(1.5) FROM sensors LIMIT 1",
                "expected/fl_half.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(-1.5) FROM sensors LIMIT 1",
                "expected/fl_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(1) FROM sensors LIMIT 1",
                "expected/fl_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(3000000000) FROM sensors LIMIT 1",
                "expected/fl_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(null) FROM sensors LIMIT 1",
                "expected/fl_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT FLOOR(READING) FROM sensors",
                "expected/fl_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(id) FROM sensors ORDER BY floor(id)",
                "expected/fl_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(score) FROM sensors ORDER BY floor(score)",
                "expected/fl_score_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(reading) FROM sensors ORDER BY floor(reading)",
                "expected/fl_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, floor(id) FROM sensors ORDER BY floor(id)",
                "expected/fl_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(id) FROM sensors WHERE id = -1",
                "expected/fl_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT floor(id) FROM sensors LIMIT 0",
                "expected/fl_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT floor(id) FROM days ORDER BY ts",
                "expected/fl_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT floor() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `floor` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT floor(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `floor` with the argument types: (STRING)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT floor(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `floor` with the argument types: (BOOLEAN)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT floor(reading, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `floor` with the argument types: (DOUBLE, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT floor(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("unsupported: decimal floor"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT floor(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn ceil_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT ceil(reading) FROM sensors",
                "expected/ce_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(score) FROM sensors",
                "expected/ce_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(id) FROM sensors",
                "expected/ce_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(flags) FROM sensors",
                "expected/ce_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(b) FROM sensors",
                "expected/ce_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(code) FROM sensors",
                "expected/ce_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(1.5) FROM sensors LIMIT 1",
                "expected/ce_half.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(-1.5) FROM sensors LIMIT 1",
                "expected/ce_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(1) FROM sensors LIMIT 1",
                "expected/ce_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(3000000000) FROM sensors LIMIT 1",
                "expected/ce_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(null) FROM sensors LIMIT 1",
                "expected/ce_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT CEIL(READING) FROM sensors",
                "expected/ce_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceiling(reading) FROM sensors",
                "expected/ce_alias.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(id) FROM sensors ORDER BY ceil(id)",
                "expected/ce_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(score) FROM sensors ORDER BY ceil(score)",
                "expected/ce_score_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(reading) FROM sensors ORDER BY ceil(reading)",
                "expected/ce_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, ceil(id) FROM sensors ORDER BY ceil(id)",
                "expected/ce_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(id) FROM sensors WHERE id = -1",
                "expected/ce_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ceil(id) FROM sensors LIMIT 0",
                "expected/ce_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT ceil(id) FROM days ORDER BY ts",
                "expected/ce_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT ceil() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `ceil` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceiling() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `ceiling` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceil(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `ceil` with the argument types: (STRING)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceiling(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `ceiling` with the argument types: (BOOLEAN)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceil(reading, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `ceil` with the argument types: (DOUBLE, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceiling(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("unsupported: decimal ceiling"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ceil(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn sign_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT sign(reading) FROM sensors",
                "expected/sg_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(score) FROM sensors",
                "expected/sg_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(id) FROM sensors",
                "expected/sg_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(flags) FROM sensors",
                "expected/sg_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(b) FROM sensors",
                "expected/sg_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(code) FROM sensors",
                "expected/sg_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(-1.5) FROM sensors LIMIT 1",
                "expected/sg_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(-0.0) FROM sensors LIMIT 1",
                "expected/sg_negzero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(0) FROM sensors LIMIT 1",
                "expected/sg_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(1) FROM sensors LIMIT 1",
                "expected/sg_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(3000000000) FROM sensors LIMIT 1",
                "expected/sg_long.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(null) FROM sensors LIMIT 1",
                "expected/sg_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SIGN(READING) FROM sensors",
                "expected/sg_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(id) FROM sensors ORDER BY sign(id)",
                "expected/sg_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(score) FROM sensors ORDER BY sign(score)",
                "expected/sg_score_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(reading) FROM sensors ORDER BY sign(reading)",
                "expected/sg_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, sign(id) FROM sensors ORDER BY sign(id)",
                "expected/sg_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(id) FROM sensors WHERE id = -1",
                "expected/sg_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sign(id) FROM sensors LIMIT 0",
                "expected/sg_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT sign(id) FROM days ORDER BY ts",
                "expected/sg_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT sign() FROM sensors").unwrap_err();
        assert!(
            err.message.contains("function `sign` requires arguments"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sign(note) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("there is no matching function `sign` with the argument types: (STRING)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sign(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `sign` with the argument types: (BOOLEAN)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sign(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "there is no matching function `sign` with the argument types: (LONG, INT)"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sign(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn sqrt_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT sqrt(reading) FROM sensors",
                "expected/sq_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(score) FROM sensors",
                "expected/sq_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(id) FROM sensors",
                "expected/sq_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(flags) FROM sensors",
                "expected/sq_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(b) FROM sensors",
                "expected/sq_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(code) FROM sensors",
                "expected/sq_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(4) FROM sensors LIMIT 1",
                "expected/sq_four.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(-1) FROM sensors LIMIT 1",
                "expected/sq_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(null) FROM sensors LIMIT 1",
                "expected/sq_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT SQRT(READING) FROM sensors",
                "expected/sq_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(id) FROM sensors ORDER BY sqrt(id)",
                "expected/sq_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(reading) FROM sensors ORDER BY sqrt(reading)",
                "expected/sq_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, sqrt(id) FROM sensors ORDER BY sqrt(id)",
                "expected/sq_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(id) FROM sensors WHERE id = 1",
                "expected/sq_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT sqrt(id) FROM sensors LIMIT 0",
                "expected/sq_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT sqrt(id) FROM days ORDER BY ts",
                "expected/sq_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT sqrt() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `sqrt` requires arguments: sqrt(DOUBLE)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sqrt(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `sqrt` at #1 expected: DOUBLE, actual: STRING"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sqrt(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `sqrt` at #1 expected: DOUBLE, actual: BOOLEAN"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sqrt(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "wrong number of arguments for function `sqrt`; expected: 1, provided: 2"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT sqrt(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn power_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT power(reading, 2) FROM sensors",
                "expected/pw_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(score, 2) FROM sensors",
                "expected/pw_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(id, 2) FROM sensors",
                "expected/pw_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(flags, 2) FROM sensors",
                "expected/pw_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(b, 2) FROM sensors",
                "expected/pw_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(code, 2) FROM sensors",
                "expected/pw_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(-2, 3) FROM sensors LIMIT 1",
                "expected/pw_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(4, 0.5) FROM sensors LIMIT 1",
                "expected/pw_root.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(-2, 0.5) FROM sensors LIMIT 1",
                "expected/pw_imag.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(0, 0) FROM sensors LIMIT 1",
                "expected/pw_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(0, -1) FROM sensors LIMIT 1",
                "expected/pw_inv.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(-0.0, 1) FROM sensors LIMIT 1",
                "expected/pw_negzero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(null, 0) FROM sensors LIMIT 1",
                "expected/pw_null_base.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(reading, null) FROM sensors",
                "expected/pw_null_exp.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(reading, id) FROM sensors",
                "expected/pw_cols.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(reading, flags) FROM sensors",
                "expected/pw_flags_exp.tsv",
            ),
            (
                sensors.clone(),
                "SELECT POWER(READING, 2) FROM sensors",
                "expected/pw_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(id, 2) FROM sensors ORDER BY power(id, 2)",
                "expected/pw_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, power(id, 2) FROM sensors ORDER BY power(id, 2)",
                "expected/pw_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(id, 2) FROM sensors WHERE id = -1",
                "expected/pw_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT power(id, 2) FROM sensors LIMIT 0",
                "expected/pw_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT power(id, 2) FROM days ORDER BY ts",
                "expected/pw_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT power() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `power` requires arguments: power(DOUBLE, DOUBLE)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT power(reading) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "wrong number of arguments for function `power`; expected: 2, provided: 1"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT power(id, 1, 2) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "wrong number of arguments for function `power`; expected: 2, provided: 3"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT power(ok, 1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("expression type mismatch, expected: DOUBLE, actual: BOOLEAN"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT power(note, 2) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("expression type mismatch, expected: DOUBLE, actual: STRING"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT power(missing, 1) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn ln_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT ln(reading) FROM sensors",
                "expected/ln_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(score) FROM sensors",
                "expected/ln_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(id) FROM sensors",
                "expected/ln_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(flags) FROM sensors",
                "expected/ln_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(b) FROM sensors",
                "expected/ln_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(code) FROM sensors",
                "expected/ln_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(1) FROM sensors LIMIT 1",
                "expected/ln_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(0) FROM sensors LIMIT 1",
                "expected/ln_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(-1) FROM sensors LIMIT 1",
                "expected/ln_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(null) FROM sensors LIMIT 1",
                "expected/ln_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LN(READING) FROM sensors",
                "expected/ln_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(id) FROM sensors ORDER BY ln(id)",
                "expected/ln_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(reading) FROM sensors ORDER BY ln(reading)",
                "expected/ln_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, ln(id) FROM sensors ORDER BY ln(id)",
                "expected/ln_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(id) FROM sensors WHERE id = 1",
                "expected/ln_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT ln(id) FROM sensors LIMIT 0",
                "expected/ln_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT ln(id) FROM days ORDER BY ts",
                "expected/ln_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT ln() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `ln` requires arguments: ln(DOUBLE)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ln(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `ln` at #1 expected: DOUBLE, actual: STRING"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ln(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `ln` at #1 expected: DOUBLE, actual: BOOLEAN"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ln(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("wrong number of arguments for function `ln`; expected: 1, provided: 2"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT ln(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn log_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT log(reading) FROM sensors",
                "expected/lg_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(score) FROM sensors",
                "expected/lg_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(id) FROM sensors",
                "expected/lg_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(flags) FROM sensors",
                "expected/lg_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(b) FROM sensors",
                "expected/lg_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(code) FROM sensors",
                "expected/lg_code.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(10) FROM sensors LIMIT 1",
                "expected/lg_ten.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(0) FROM sensors LIMIT 1",
                "expected/lg_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(-1) FROM sensors LIMIT 1",
                "expected/lg_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(null) FROM sensors LIMIT 1",
                "expected/lg_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT LOG(READING) FROM sensors",
                "expected/lg_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(id) FROM sensors ORDER BY log(id)",
                "expected/lg_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(reading) FROM sensors ORDER BY log(reading)",
                "expected/lg_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, log(id) FROM sensors ORDER BY log(id)",
                "expected/lg_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(id) FROM sensors WHERE id = 1",
                "expected/lg_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT log(id) FROM sensors LIMIT 0",
                "expected/lg_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT log(id) FROM days ORDER BY ts",
                "expected/lg_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT log() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `log` requires arguments: log(DOUBLE)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT log(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `log` at #1 expected: DOUBLE, actual: STRING"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT log(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `log` at #1 expected: DOUBLE, actual: BOOLEAN"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT log(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("wrong number of arguments for function `log`; expected: 1, provided: 2"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT log(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn exp_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT exp(reading) FROM sensors",
                "expected/ex_reading.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(score) FROM sensors",
                "expected/ex_score.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(id) FROM sensors",
                "expected/ex_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(flags) FROM sensors",
                "expected/ex_flags.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(b) FROM sensors",
                "expected/ex_byte.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(1) FROM sensors LIMIT 1",
                "expected/ex_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(0) FROM sensors LIMIT 1",
                "expected/ex_zero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(-1) FROM sensors LIMIT 1",
                "expected/ex_neg.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(-0.0) FROM sensors LIMIT 1",
                "expected/ex_negzero.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(1000) FROM sensors LIMIT 1",
                "expected/ex_big.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(null) FROM sensors LIMIT 1",
                "expected/ex_null.tsv",
            ),
            (
                sensors.clone(),
                "SELECT EXP(READING) FROM sensors",
                "expected/ex_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(id) FROM sensors ORDER BY exp(id)",
                "expected/ex_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(reading) FROM sensors ORDER BY exp(reading)",
                "expected/ex_reading_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, exp(id) FROM sensors ORDER BY exp(id)",
                "expected/ex_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(id) FROM sensors WHERE id = -1",
                "expected/ex_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT exp(id) FROM sensors LIMIT 0",
                "expected/ex_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT exp(id) FROM days ORDER BY ts",
                "expected/ex_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT exp() FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("function `exp` requires arguments: exp(DOUBLE)"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT exp(note) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `exp` at #1 expected: DOUBLE, actual: STRING"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT exp(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains(
                "argument type mismatch for function `exp` at #1 expected: DOUBLE, actual: BOOLEAN"
            ),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT exp(id, 1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("wrong number of arguments for function `exp`; expected: 1, provided: 2"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT exp(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn pi_matches_captured_exec() {
        let sensors = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1");
        let days = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let cases = [
            (
                sensors.clone(),
                "SELECT pi() FROM sensors LIMIT 1",
                "expected/pi_one.tsv",
            ),
            (
                sensors.clone(),
                "SELECT pi() FROM sensors",
                "expected/pi_all.tsv",
            ),
            (
                sensors.clone(),
                "SELECT PI() FROM sensors",
                "expected/pi_case.tsv",
            ),
            (
                sensors.clone(),
                "SELECT id, pi() FROM sensors ORDER BY id",
                "expected/pi_with_id.tsv",
            ),
            (
                sensors.clone(),
                "SELECT pi() FROM sensors WHERE id = 1",
                "expected/pi_where.tsv",
            ),
            (
                sensors.clone(),
                "SELECT pi() FROM sensors ORDER BY pi()",
                "expected/pi_order.tsv",
            ),
            (
                sensors.clone(),
                "SELECT pi() FROM sensors LIMIT 0",
                "expected/pi_limit_zero.tsv",
            ),
            (
                days.clone(),
                "SELECT pi() FROM days ORDER BY ts",
                "expected/pi_id.tsv",
            ),
        ];
        for (root, sql, file) in cases {
            let result = execute(&root, sql).unwrap();
            let expected = std::fs::read_to_string(root.join(file)).unwrap();
            assert_eq!(render_tsv(&result), expected, "{sql}");
        }
        let err = execute(&sensors, "SELECT pi(1) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("wrong number of arguments for function `pi`; expected: 0, provided: 1"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT pi(id) FROM sensors").unwrap_err();
        assert!(
            err.message
                .contains("wrong number of arguments for function `pi`; expected: 0, provided: 1"),
            "{}",
            err.message
        );
        let err = execute(&sensors, "SELECT pi(missing) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("Invalid column: missing"),
            "{}",
            err.message
        );
    }

    #[test]
    fn rejects_unimplemented_sql() {
        let err = execute(&root(), "INSERT INTO sensors VALUES (1)").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = execute(&root(), "SELECT median() FROM sensors").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = execute(&root(), "SELECT count(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("no matching function"),
            "{}",
            err.message
        );
    }
}
