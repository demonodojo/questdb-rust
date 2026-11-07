use crate::column::{read_column, read_symbol_map};
use crate::meta::{parse_meta, ColumnMeta, TableMeta};
use crate::txn::{parse_txn, PartitionRecord, TxnHeader};
use qdb_platform::{FileSystem, StdFs};
use qdb_types::{civil_from_unix_days, Error, Scalar, PARTITION_BY_DAY, PARTITION_BY_NONE};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub meta: TableMeta,
    pub txn: TxnHeader,
    pub columns: Vec<Vec<Scalar>>,
    /// Symbol dictionaries in symbol-column order. The index is the QuestDB key.
    pub symbol_maps: Vec<Vec<String>>,
}

impl Table {
    pub fn row_count(&self) -> usize {
        self.columns.first().map(|c| c.len()).unwrap_or(0)
    }

    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.meta
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case(name))
    }
}

pub fn open_table(data_dir: &Path, table_name: &str) -> Result<Table, Error> {
    open_table_with(&StdFs, data_dir, table_name)
}

pub fn open_table_with<F: FileSystem>(
    fs: &F,
    data_dir: &Path,
    table_name: &str,
) -> Result<Table, Error> {
    if table_name.is_empty() || table_name.contains(['/', '\\']) || table_name.starts_with('.') {
        return Err(Error::new(format!("invalid table name `{table_name}`")));
    }
    let table_dir = locate_table(fs, data_dir, table_name)?;
    let meta_bytes = fs.read(&table_dir.join("_meta"))?;
    let txn_bytes = fs.read(&table_dir.join("_txn"))?;
    let meta = parse_meta(&meta_bytes)?;
    let txn = parse_txn(&txn_bytes)?;
    let symbol_columns = meta
        .columns
        .iter()
        .filter(|column| column.symbol_index.is_some())
        .count();
    if symbol_columns != txn.symbol_counts.len() {
        return Err(Error::new(format!(
            "table `{table_name}` has {symbol_columns} symbol columns in _meta and {} in _txn",
            txn.symbol_counts.len()
        )));
    }
    let partitions = partition_slices(&meta, &txn)?;
    let mut symbols = Vec::new();
    for column in &meta.columns {
        if let Some(symbol_index) = column.symbol_index {
            let count = txn.symbol_counts[symbol_index];
            let chars = fs.read(&column_path(&table_dir, &column.name, "c"))?;
            let offsets = fs.read(&column_path(&table_dir, &column.name, "o"))?;
            symbols.push(read_symbol_map(&chars, &offsets, count, &column.name)?);
        }
    }
    let mut columns = meta.columns.iter().map(|_| Vec::new()).collect::<Vec<_>>();
    for partition in &partitions {
        let partition_dir = table_dir.join(&partition.dir);
        if !fs.is_dir(&partition_dir) {
            return Err(Error::new(format!(
                "missing partition {} of table `{table_name}`",
                partition_dir.display()
            )));
        }
        if partition.rows == 0 {
            continue;
        }
        for (index, column) in meta.columns.iter().enumerate() {
            let data = fs.read(&column_path(&partition_dir, &column.name, "d"))?;
            let aux = if needs_aux(column) {
                Some(fs.read(&column_path(&partition_dir, &column.name, "i"))?)
            } else {
                None
            };
            let symbol_values = column.symbol_index.map(|index| symbols[index].as_slice());
            let values = read_column(column, partition.rows, &data, aux.as_deref(), symbol_values)?;
            columns[index].extend(values);
        }
    }
    Ok(Table {
        name: table_name.to_string(),
        meta,
        txn,
        columns,
        symbol_maps: symbols,
    })
}

struct PartitionSlice {
    dir: String,
    rows: usize,
}

fn partition_slices(meta: &TableMeta, txn: &TxnHeader) -> Result<Vec<PartitionSlice>, Error> {
    if meta.partition_by == PARTITION_BY_NONE {
        if txn.fixed_row_count != 0 {
            return Err(Error::new(format!(
                "PARTITION BY NONE table has fixed_row_count {}",
                txn.fixed_row_count
            )));
        }
        return Ok(vec![PartitionSlice {
            dir: "default".to_string(),
            rows: usize::try_from(txn.transient_row_count)
                .map_err(|_| Error::new("_txn: transient row count does not fit usize"))?,
        }]);
    }
    if meta.partition_by != PARTITION_BY_DAY {
        return Err(Error::new(format!(
            "unsupported partition by {}",
            meta.partition_by
        )));
    }
    if txn.partitions.is_empty() {
        if txn.fixed_row_count != 0 || txn.transient_row_count != 0 {
            return Err(Error::new(
                "_txn: DAY table has rows but no partition records",
            ));
        }
        return Ok(Vec::new());
    }
    let last = txn.partitions.len() - 1;
    let mut fixed = 0i64;
    let mut slices = Vec::with_capacity(txn.partitions.len());
    for (index, partition) in txn.partitions.iter().enumerate() {
        let rows = if index == last {
            txn.transient_row_count
        } else {
            let stored = i64::try_from(partition.stored_row_count).map_err(|_| {
                Error::new("_txn: partition row count does not fit a signed 64-bit integer")
            })?;
            fixed = fixed
                .checked_add(stored)
                .ok_or_else(|| Error::new("_txn: fixed row count overflow"))?;
            stored
        };
        if rows < 0 {
            return Err(Error::new(format!(
                "_txn: negative partition row count {rows}"
            )));
        }
        slices.push(PartitionSlice {
            dir: day_partition_dir(partition)?,
            rows: usize::try_from(rows)
                .map_err(|_| Error::new("_txn: partition row count does not fit usize"))?,
        });
    }
    if fixed != txn.fixed_row_count {
        return Err(Error::new(format!(
            "_txn: non-last partitions sum to {fixed} rows but fixed_row_count is {}",
            txn.fixed_row_count
        )));
    }
    Ok(slices)
}

fn day_partition_dir(partition: &PartitionRecord) -> Result<String, Error> {
    const MICROS_PER_DAY: i64 = 86_400 * 1_000_000;
    let days = partition.timestamp.div_euclid(MICROS_PER_DAY);
    let (year, month, day) = civil_from_unix_days(days);
    let mut name = format!("{year:04}-{month:02}-{day:02}");
    if partition.name_txn > -1 {
        name.push('.');
        name.push_str(&partition.name_txn.to_string());
    }
    Ok(name)
}

fn needs_aux(column: &ColumnMeta) -> bool {
    matches!(column.tag, qdb_types::TAG_STRING | qdb_types::TAG_VARCHAR)
}

fn column_path(dir: &Path, name: &str, suffix: &str) -> PathBuf {
    dir.join(format!("{name}.{suffix}"))
}

fn locate_table<F: FileSystem>(
    fs: &F,
    data_dir: &Path,
    table_name: &str,
) -> Result<PathBuf, Error> {
    let nested = data_dir.join("db").join(table_name);
    if fs.is_dir(&nested) {
        return Ok(nested);
    }
    let direct = data_dir.join(table_name);
    if fs.is_dir(&direct) {
        return Ok(direct);
    }
    Err(Error::new(format!(
        "table `{table_name}` was not found under {}",
        data_dir.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qdb_types::Scalar;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h1")
    }

    fn bmp(ch: char) -> u16 {
        u16::try_from(u32::from(ch)).expect("fixture characters are in the BMP")
    }

    #[test]
    fn reads_original_sensors_fixture() {
        let table = open_table(&fixture_root(), "sensors").unwrap();
        assert_eq!(table.meta.version, 426);
        assert_eq!(table.meta.table_id, 8);
        assert!(!table.meta.wal_enabled);
        assert_eq!(table.txn.txn, 4);
        assert_eq!(table.txn.transient_row_count, 4);
        assert_eq!(table.txn.fixed_row_count, 0);
        assert_eq!(table.txn.max_timestamp, 1_577_934_248_000_000);
        assert_eq!(table.txn.symbol_counts, vec![2]);
        assert_eq!(table.row_count(), 4);
        assert_eq!(
            table
                .meta
                .columns
                .iter()
                .map(|c| c.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "ts", "id", "flags", "reading", "score", "ok", "code", "b", "ch", "note",
                "location", "tag", "day"
            ]
        );

        let ts = &table.columns[table.column_index("ts").unwrap()];
        assert_eq!(ts[0], Scalar::Timestamp(1_577_934_245_000_000));
        assert_eq!(ts[3], Scalar::Timestamp(1_577_934_248_000_000));

        let id = &table.columns[table.column_index("id").unwrap()];
        assert_eq!(
            id,
            &vec![
                Scalar::Long(1),
                Scalar::Long(2),
                Scalar::Null,
                Scalar::Long(-1)
            ]
        );

        let flags = &table.columns[table.column_index("flags").unwrap()];
        assert_eq!(flags[1], Scalar::Null);
        assert_eq!(flags[2], Scalar::Int(0));

        let reading = &table.columns[table.column_index("reading").unwrap()];
        assert_eq!(reading[0], Scalar::Double(1.5));
        assert_eq!(reading[1], Scalar::Null);
        assert_eq!(reading[2], Scalar::Double(0.0));
        assert_eq!(reading[3], Scalar::Double(-0.0));

        let score = &table.columns[table.column_index("score").unwrap()];
        assert_eq!(score[0], Scalar::Float(1.25));
        assert_eq!(score[1], Scalar::Null);
        assert_eq!(score[3], Scalar::Float(-1.5));

        let ok = &table.columns[table.column_index("ok").unwrap()];
        assert_eq!(
            ok,
            &vec![
                Scalar::Boolean(true),
                Scalar::Boolean(false),
                Scalar::Boolean(true),
                Scalar::Boolean(false)
            ]
        );

        let code = &table.columns[table.column_index("code").unwrap()];
        assert_eq!(code[2], Scalar::Short(-32768));
        let byte = &table.columns[table.column_index("b").unwrap()];
        assert_eq!(byte[2], Scalar::Byte(-128));
        let ch = &table.columns[table.column_index("ch").unwrap()];
        assert_eq!(ch[0], Scalar::Char(bmp('A')));
        assert_eq!(ch[1], Scalar::Char(0));
        assert_eq!(ch[2], Scalar::Char(bmp('β')));
        assert_eq!(ch[3], Scalar::Char(bmp('東')));

        let note = &table.columns[table.column_index("note").unwrap()];
        assert_eq!(
            note,
            &vec![
                Scalar::Text("alpha".to_string()),
                Scalar::Null,
                Scalar::Text(String::new()),
                Scalar::Text("βeta".to_string()),
            ]
        );
        let location = &table.columns[table.column_index("location").unwrap()];
        assert_eq!(
            location,
            &vec![
                Scalar::Text("madrid".to_string()),
                Scalar::Null,
                Scalar::Text("madrid".to_string()),
                Scalar::Text("東京".to_string()),
            ]
        );
        let tag = &table.columns[table.column_index("tag").unwrap()];
        assert_eq!(tag[0], Scalar::Text("v1".to_string()));
        assert_eq!(tag[1], Scalar::Null);
        assert_eq!(tag[2], Scalar::Text("héllo".to_string()));
        assert_eq!(tag[3], Scalar::Text("0123456789ABCDEF".to_string()));

        let day = &table.columns[table.column_index("day").unwrap()];
        assert_eq!(day[0], Scalar::Date(1_577_923_200_000));
        assert_eq!(day[1], Scalar::Null);
        assert_eq!(day[2], Scalar::Date(0));
        assert_eq!(day[3], Scalar::Date(1_592_224_200_000));
    }

    #[test]
    fn reads_two_day_partitions_including_name_txn_suffix() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/h2");
        let table = open_table(&root, "days").unwrap();
        assert_eq!(table.meta.partition_by, PARTITION_BY_DAY);
        assert!(!table.meta.wal_enabled);
        assert_eq!(table.txn.fixed_row_count, 2);
        assert_eq!(table.txn.transient_row_count, 2);
        assert_eq!(table.txn.partitions.len(), 2);
        assert_eq!(table.txn.partitions[0].name_txn, -1);
        assert_eq!(table.txn.partitions[0].stored_row_count, 2);
        assert_eq!(table.txn.partitions[0].timestamp, 1_577_923_200_000_000);
        assert_eq!(table.txn.partitions[1].name_txn, 1);
        assert_eq!(table.txn.partitions[1].stored_row_count, 0);
        assert_eq!(table.txn.partitions[1].timestamp, 1_578_009_600_000_000);
        assert_eq!(table.row_count(), 4);

        let ts = &table.columns[table.column_index("ts").unwrap()];
        assert_eq!(
            ts,
            &vec![
                Scalar::Timestamp(1_577_934_245_000_000),
                Scalar::Timestamp(1_578_006_000_000_000),
                Scalar::Timestamp(1_578_009_600_000_000),
                Scalar::Timestamp(1_578_054_600_000_000),
            ]
        );
        let id = &table.columns[table.column_index("id").unwrap()];
        assert_eq!(
            id,
            &vec![
                Scalar::Long(1),
                Scalar::Long(2),
                Scalar::Long(3),
                Scalar::Null,
            ]
        );
        let note = &table.columns[table.column_index("note").unwrap()];
        assert_eq!(
            note,
            &vec![
                Scalar::Text("alpha".to_string()),
                Scalar::Null,
                Scalar::Text("beta".to_string()),
                Scalar::Text("東京".to_string()),
            ]
        );
        let location = &table.columns[table.column_index("location").unwrap()];
        assert_eq!(
            location,
            &vec![
                Scalar::Text("madrid".to_string()),
                Scalar::Null,
                Scalar::Text("madrid".to_string()),
                Scalar::Text("東京".to_string()),
            ]
        );
    }

    #[test]
    fn missing_table_is_an_error() {
        let err = open_table(&fixture_root(), "no_such_table").unwrap_err();
        assert!(err.message.contains("no_such_table"), "{}", err.message);
    }
}
