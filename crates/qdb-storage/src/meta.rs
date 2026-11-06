//! `_meta` layout from `TableUtils.writeMetadata` in QuestDB 10.1.0.

use crate::bytes::{i32_at, utf16_str_at};
use qdb_types::{type_name, type_tag, Error, METADATA_VERSION_10_1_0, PARTITION_BY_NONE};

const META_OFFSET_COUNT: usize = 0;
const META_OFFSET_PARTITION_BY: usize = 4;
const META_OFFSET_TIMESTAMP_INDEX: usize = 8;
const META_OFFSET_VERSION: usize = 12;
const META_OFFSET_TABLE_ID: usize = 16;
const META_OFFSET_WAL_ENABLED: usize = 40;
const META_OFFSET_COLUMN_TYPES: usize = 128;
const META_COLUMN_DATA_SIZE: usize = 32;
/// `putInt(type) + putLong(flags) + putInt(index) + putInt(symbolCapacity) + putInt(parquet) = 24`.
const META_COLUMN_REPLACING_INDEX: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnMeta {
    pub name: String,
    pub raw_type: i32,
    pub tag: i32,
    pub symbol_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableMeta {
    pub column_count: usize,
    pub partition_by: i32,
    pub timestamp_index: i32,
    pub version: i32,
    pub table_id: i32,
    pub wal_enabled: bool,
    pub columns: Vec<ColumnMeta>,
}

pub fn parse_meta(bytes: &[u8]) -> Result<TableMeta, Error> {
    let column_count = i32_at(bytes, META_OFFSET_COUNT, "_meta column count")?;
    if column_count <= 0 {
        return Err(Error::new(format!(
            "_meta: column count {column_count} is not positive"
        )));
    }
    let column_count = column_count as usize;
    let partition_by = i32_at(bytes, META_OFFSET_PARTITION_BY, "_meta partition by")?;
    if partition_by != PARTITION_BY_NONE {
        return Err(Error::new(format!(
            "unsupported partition by {partition_by}; this reader only opens PARTITION BY NONE ({PARTITION_BY_NONE})"
        )));
    }
    let timestamp_index = i32_at(bytes, META_OFFSET_TIMESTAMP_INDEX, "_meta timestamp index")?;
    let version = i32_at(bytes, META_OFFSET_VERSION, "_meta version")?;
    if version != METADATA_VERSION_10_1_0 {
        return Err(Error::new(format!(
            "unsupported _meta version {version}; this reader accepts {METADATA_VERSION_10_1_0}"
        )));
    }
    let table_id = i32_at(bytes, META_OFFSET_TABLE_ID, "_meta table id")?;
    if META_OFFSET_WAL_ENABLED >= bytes.len() {
        return Err(Error::new("_meta: file is shorter than the header"));
    }
    let wal_enabled = bytes[META_OFFSET_WAL_ENABLED] != 0;
    if wal_enabled {
        return Err(Error::new(
            "WAL tables are not supported by this reader; column data may still sit in wal segments",
        ));
    }

    let records = column_count
        .checked_mul(META_COLUMN_DATA_SIZE)
        .ok_or_else(|| Error::new("_meta: column record size overflow"))?;
    let name_offset = META_OFFSET_COLUMN_TYPES + records;
    if name_offset > bytes.len() {
        return Err(Error::new("_meta: column records extend past the file"));
    }

    let mut raw = Vec::with_capacity(column_count);
    let mut symbol_count = 0usize;
    for index in 0..column_count {
        let at = META_OFFSET_COLUMN_TYPES + index * META_COLUMN_DATA_SIZE;
        let raw_type = i32_at(bytes, at, "_meta column type")?;
        if raw_type < 0 {
            return Err(Error::new(format!(
                "unsupported deleted column at index {index}"
            )));
        }
        let replacing = i32_at(
            bytes,
            at + META_COLUMN_REPLACING_INDEX,
            "_meta replacing index",
        )?;
        if replacing != 0 {
            return Err(Error::new(format!(
                "unsupported column replacement at index {index} (stored {replacing})"
            )));
        }
        let tag = type_tag(raw_type);
        if tag == qdb_types::TAG_TIMESTAMP && raw_type & qdb_types::TIMESTAMP_NANO_FLAG != 0 {
            return Err(Error::new(
                "nanosecond timestamps are not supported by this reader",
            ));
        }
        if !supported_tag(tag) {
            return Err(Error::new(format!(
                "unsupported column type {} (tag {tag}) at index {index}",
                type_name(tag)
            )));
        }
        let symbol_index = if tag == qdb_types::TAG_SYMBOL {
            let current = symbol_count;
            symbol_count += 1;
            Some(current)
        } else {
            None
        };
        raw.push((raw_type, tag, symbol_index));
    }
    if timestamp_index < -1 || timestamp_index as usize >= column_count {
        return Err(Error::new(format!(
            "_meta: timestamp index {timestamp_index} is outside 0..{column_count}"
        )));
    }

    let mut cursor = name_offset;
    let mut columns = Vec::with_capacity(column_count);
    for (index, (raw_type, tag, symbol_index)) in raw.into_iter().enumerate() {
        let (name, used) = utf16_str_at(bytes, cursor, "_meta column name")?;
        let name = name.ok_or_else(|| Error::new(format!("_meta: NULL column name at {index}")))?;
        if name.is_empty() || name.len() > 255 {
            return Err(Error::new(format!(
                "_meta: column name length {} is invalid at {index}",
                name.len()
            )));
        }
        cursor += used;
        columns.push(ColumnMeta {
            name,
            raw_type,
            tag,
            symbol_index,
        });
    }

    Ok(TableMeta {
        column_count,
        partition_by,
        timestamp_index,
        version,
        table_id,
        wal_enabled,
        columns,
    })
}

fn supported_tag(tag: i32) -> bool {
    matches!(
        tag,
        qdb_types::TAG_BOOLEAN
            | qdb_types::TAG_BYTE
            | qdb_types::TAG_SHORT
            | qdb_types::TAG_CHAR
            | qdb_types::TAG_INT
            | qdb_types::TAG_LONG
            | qdb_types::TAG_DATE
            | qdb_types::TAG_TIMESTAMP
            | qdb_types::TAG_FLOAT
            | qdb_types::TAG_DOUBLE
            | qdb_types::TAG_STRING
            | qdb_types::TAG_SYMBOL
            | qdb_types::TAG_VARCHAR
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncated_header_is_rejected() {
        let err = parse_meta(&[1, 0, 0]).unwrap_err();
        assert!(err.message.contains("_meta"), "{}", err.message);
    }
}
