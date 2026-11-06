//! `_txn` A/B header from `TableUtils` and `TxReader.unsafeLoadBaseOffset`.

use crate::bytes::{i32_at, i64_at};
use qdb_types::Error;

const TX_BASE_OFFSET_VERSION: usize = 0;
const TX_BASE_OFFSET_A: usize = 8;
const TX_BASE_OFFSET_SYMBOLS_SIZE_A: usize = 12;
const TX_BASE_OFFSET_PARTITIONS_SIZE_A: usize = 16;
const TX_BASE_OFFSET_B: usize = 32;
const TX_BASE_OFFSET_SYMBOLS_SIZE_B: usize = 36;
const TX_BASE_OFFSET_PARTITIONS_SIZE_B: usize = 40;
const TX_OFFSET_TXN: usize = 0;
const TX_OFFSET_TRANSIENT_ROW_COUNT: usize = 8;
const TX_OFFSET_FIXED_ROW_COUNT: usize = 16;
const TX_OFFSET_MIN_TIMESTAMP: usize = 24;
const TX_OFFSET_MAX_TIMESTAMP: usize = 32;
const TX_OFFSET_MAP_WRITER_COUNT: usize = 128;
const SYMBOL_WRITER_SLOT: usize = 8;
const PARTITION_RECORD_BYTES: usize = 32;
/// Low 44 bits of the partition size word. Flag bits above that are not a row count.
const PARTITION_SIZE_MASK: u64 = (1 << 44) - 1;
const PARTITION_PARQUET_FORMAT_BIT: u64 = 1 << 61;
const PARTITION_COMPOSITE_BIT: u64 = 1 << 61;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionRecord {
    pub timestamp: i64,
    /// Row count stored in the partition record. The active partition's count lives in
    /// `TxnHeader::transient_row_count` and can disagree with this field.
    pub stored_row_count: u64,
    pub name_txn: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxnHeader {
    pub txn: i64,
    pub transient_row_count: i64,
    pub fixed_row_count: i64,
    pub min_timestamp: i64,
    pub max_timestamp: i64,
    /// Distinct values per symbol column, in symbol-column order.
    pub symbol_counts: Vec<i32>,
    pub partitions: Vec<PartitionRecord>,
}

impl TxnHeader {
    pub fn row_count(&self) -> Result<usize, Error> {
        if self.transient_row_count < 0 || self.fixed_row_count < 0 {
            return Err(Error::new(format!(
                "_txn: negative row count transient={} fixed={}",
                self.transient_row_count, self.fixed_row_count
            )));
        }
        let sum = self
            .fixed_row_count
            .checked_add(self.transient_row_count)
            .ok_or_else(|| Error::new("_txn: row count overflow"))?;
        usize::try_from(sum).map_err(|_| Error::new("_txn: row count does not fit usize"))
    }
}

pub fn parse_txn(bytes: &[u8]) -> Result<TxnHeader, Error> {
    let version = i64_at(bytes, TX_BASE_OFFSET_VERSION, "_txn version")?;
    let use_a = version & 1 == 0;
    let (base, symbols_size, partitions_size) = if use_a {
        (
            i32_at(bytes, TX_BASE_OFFSET_A, "_txn offset A")?,
            i32_at(bytes, TX_BASE_OFFSET_SYMBOLS_SIZE_A, "_txn symbols size A")?,
            i32_at(
                bytes,
                TX_BASE_OFFSET_PARTITIONS_SIZE_A,
                "_txn partitions size A",
            )?,
        )
    } else {
        (
            i32_at(bytes, TX_BASE_OFFSET_B, "_txn offset B")?,
            i32_at(bytes, TX_BASE_OFFSET_SYMBOLS_SIZE_B, "_txn symbols size B")?,
            i32_at(
                bytes,
                TX_BASE_OFFSET_PARTITIONS_SIZE_B,
                "_txn partitions size B",
            )?,
        )
    };
    if base < 0
        || symbols_size < 0
        || symbols_size % SYMBOL_WRITER_SLOT as i32 != 0
        || partitions_size < 0
        || partitions_size as usize % PARTITION_RECORD_BYTES != 0
    {
        return Err(Error::new(format!(
            "_txn: invalid section base={base} symbols_size={symbols_size} partitions_size={partitions_size}"
        )));
    }
    let base = base as usize;
    let txn = i64_at(bytes, base + TX_OFFSET_TXN, "_txn txn")?;
    if txn != version {
        return Err(Error::new(format!(
            "_txn: section txn {txn} does not match header version {version}"
        )));
    }
    let transient_row_count = i64_at(
        bytes,
        base + TX_OFFSET_TRANSIENT_ROW_COUNT,
        "_txn transient row count",
    )?;
    let fixed_row_count = i64_at(
        bytes,
        base + TX_OFFSET_FIXED_ROW_COUNT,
        "_txn fixed row count",
    )?;
    let min_timestamp = i64_at(bytes, base + TX_OFFSET_MIN_TIMESTAMP, "_txn min timestamp")?;
    let max_timestamp = i64_at(bytes, base + TX_OFFSET_MAX_TIMESTAMP, "_txn max timestamp")?;
    let map_writer_count = i32_at(
        bytes,
        base + TX_OFFSET_MAP_WRITER_COUNT,
        "_txn map writer count",
    )?;
    let symbol_columns = symbols_size as usize / SYMBOL_WRITER_SLOT;
    if map_writer_count < 0 || map_writer_count as usize != symbol_columns {
        return Err(Error::new(format!(
            "_txn: symbol column count {map_writer_count} does not match section size {symbols_size}"
        )));
    }
    let mut symbol_counts = Vec::with_capacity(symbol_columns);
    for index in 0..symbol_columns {
        let at = base + TX_OFFSET_MAP_WRITER_COUNT + 4 + index * SYMBOL_WRITER_SLOT;
        let count = i32_at(bytes, at, "_txn symbol count")?;
        if count < 0 {
            return Err(Error::new(format!("_txn: negative symbol count {count}")));
        }
        symbol_counts.push(count);
    }
    let partitions = read_partitions(bytes, base, symbols_size as usize, partitions_size as usize)?;
    Ok(TxnHeader {
        txn,
        transient_row_count,
        fixed_row_count,
        min_timestamp,
        max_timestamp,
        symbol_counts,
        partitions,
    })
}

fn read_partitions(
    bytes: &[u8],
    base: usize,
    symbols_size: usize,
    partitions_size: usize,
) -> Result<Vec<PartitionRecord>, Error> {
    let size_at = base + TX_OFFSET_MAP_WRITER_COUNT + 4 + symbols_size;
    let recorded = i32_at(bytes, size_at, "_txn partition table size")?;
    if recorded < 0 || recorded as usize != partitions_size {
        return Err(Error::new(format!(
            "_txn: partition table size {recorded} does not match header {partitions_size}"
        )));
    }
    let mut partitions = Vec::with_capacity(partitions_size / PARTITION_RECORD_BYTES);
    let mut previous_timestamp = i64::MIN;
    for index in 0..partitions_size / PARTITION_RECORD_BYTES {
        let at = size_at + 4 + index * PARTITION_RECORD_BYTES;
        let timestamp = i64_at(bytes, at, "_txn partition timestamp")?;
        if index > 0 && timestamp <= previous_timestamp {
            return Err(Error::new(format!(
                "_txn: partition timestamp {timestamp} is not after {previous_timestamp}"
            )));
        }
        previous_timestamp = timestamp;
        let masked = i64_at(bytes, at + 8, "_txn partition size")? as u64;
        if masked & PARTITION_PARQUET_FORMAT_BIT != 0 {
            return Err(Error::new(format!(
                "_txn: partition {index} is parquet and this reader only opens native columns"
            )));
        }
        let version = i64_at(bytes, at + 24, "_txn partition version")? as u64;
        if version & PARTITION_COMPOSITE_BIT != 0 {
            return Err(Error::new(format!(
                "_txn: partition {index} is composite and this reader only opens plain directories"
            )));
        }
        let name_txn = i64_at(bytes, at + 16, "_txn partition name txn")?;
        if name_txn < -1 {
            return Err(Error::new(format!(
                "_txn: partition {index} name txn {name_txn} is invalid"
            )));
        }
        partitions.push(PartitionRecord {
            timestamp,
            stored_row_count: masked & PARTITION_SIZE_MASK,
            name_txn,
        });
    }
    Ok(partitions)
}
