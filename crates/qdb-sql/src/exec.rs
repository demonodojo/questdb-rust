use crate::parse::{
    date_from_literal, parse_query, timestamp_from_literal, CmpOp, CountExpr, Filter,
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
            SelectList::Count(_) | SelectList::Grouped(_)
        )
    {
        return Err(Error::new("unsupported: GROUP BY without count()"));
    }
    let projection = match &query.columns {
        SelectList::Count(count) if query.group_by.is_none() => {
            return execute_count(table, query, count);
        }
        SelectList::Count(count) => return execute_group(table, query, None, count),
        SelectList::Grouped(grouped) => {
            return execute_group(table, query, Some(grouped), &grouped.count);
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

fn execute_count(table: &Table, query: &Query, count: &CountExpr) -> Result<QueryResult, Error> {
    for order in &query.order_by {
        if !order.column.eq_ignore_ascii_case(&count.label) {
            return Err(Error::new(format!(
                "ORDER BY expressions must appear in select list. Invalid column: {}",
                order.column
            )));
        }
    }
    let counted = counted_column(table, count)?;
    let mut total = 0i64;
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        total += count_increment(&table.columns, row, counted);
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
        vec![vec![Scalar::Long(total)]]
    };
    Ok(QueryResult {
        columns: vec![count.label.clone()],
        rows,
    })
}

fn execute_group(
    table: &Table,
    query: &Query,
    grouped: Option<&GroupedSelect>,
    count: &CountExpr,
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
        let by_count = order.column.eq_ignore_ascii_case(&count.label);
        let by_key = order.column.eq_ignore_ascii_case(group_name);
        if !by_count && !by_key {
            return Err(Error::new(format!(
                "ORDER BY expressions must appear in select list. Invalid column: {}",
                order.column
            )));
        }
    }
    let counted = counted_column(table, count)?;
    let mut groups: Vec<(Scalar, i64)> = Vec::new();
    for row in 0..table.row_count() {
        if let Some(filter) = &query.filter {
            if !row_matches(table, row, filter)? {
                continue;
            }
        }
        let key = table.columns[key_index][row].clone();
        let increment = count_increment(&table.columns, row, counted);
        if let Some(group) = groups.iter_mut().find(|(found, _)| found == &key) {
            group.1 += increment;
        } else {
            groups.push((key, increment));
        }
    }
    let tag = table.meta.columns[key_index].tag;
    groups.sort_by(|left, right| {
        for order in &query.order_by {
            let mut ordering = if order.column.eq_ignore_ascii_case(&count.label) {
                left.1.cmp(&right.1)
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
            vec![grouped.key.clone(), count.label.clone()]
        }
        Some(grouped) => vec![count.label.clone(), grouped.key.clone()],
        None => vec![count.label.clone()],
    };
    let rows = row_ids
        .into_iter()
        .map(|index| {
            let (key, total) = &groups[index];
            let count_value = Scalar::Long(*total);
            match grouped {
                Some(grouped) if grouped.key_at == 0 => vec![key.clone(), count_value],
                Some(_) => vec![count_value, key.clone()],
                None => vec![count_value],
            }
        })
        .collect();
    Ok(QueryResult { columns, rows })
}

fn counted_column(table: &Table, count: &CountExpr) -> Result<Option<(usize, i32)>, Error> {
    let Some(name) = &count.column else {
        return Ok(None);
    };
    let index = table
        .column_index(name)
        .ok_or_else(|| Error::new(format!("Invalid column: {name}")))?;
    let tag = table.meta.columns[index].tag;
    if tag == qdb_types::TAG_BOOLEAN {
        return Err(Error::new(
            "there is no matching function `count` with the argument types: (BOOLEAN)",
        ));
    }
    Ok(Some((index, tag)))
}

fn count_increment(columns: &[Vec<Scalar>], row: usize, counted: Option<(usize, i32)>) -> i64 {
    let Some((index, tag)) = counted else {
        return 1;
    };
    let value = &columns[index][row];
    if value.is_null() || (tag == qdb_types::TAG_CHAR && matches!(value, Scalar::Char(0))) {
        0
    } else {
        1
    }
}

fn limit_follows_timestamp_order(table: &Table, query: &Query) -> Result<bool, Error> {
    // A single designated-timestamp key uses the ordered scan. Any other key
    // list, including `ts` plus another column, uses the sort path.
    let [order] = query.order_by.as_slice() else {
        return Ok(query.order_by.is_empty());
    };
    let index = table
        .column_index(&order.column)
        .ok_or_else(|| Error::new(format!("unknown ORDER BY column `{}`", order.column)))?;
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
    fn rejects_unimplemented_sql() {
        let err = execute(&root(), "INSERT INTO sensors VALUES (1)").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = execute(&root(), "SELECT sum() FROM sensors").unwrap_err();
        assert!(err.message.contains("unsupported"), "{}", err.message);
        let err = execute(&root(), "SELECT count(ok) FROM sensors").unwrap_err();
        assert!(
            err.message.contains("no matching function"),
            "{}",
            err.message
        );
    }
}
