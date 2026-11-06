use crate::parse::{
    date_from_literal, parse_query, timestamp_from_literal, AggFunc, AggregateExpr, CmpOp, Filter,
    GroupedSelect, Query, SelectList,
};
use qdb_storage::{open_table, Table};
use qdb_types::{Error, Scalar};
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
        }
    }

    fn finish(&self) -> Scalar {
        match self {
            Self::Count(total) => Scalar::Long(*total),
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
        AggFunc::Count => qdb_types::TAG_LONG,
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
