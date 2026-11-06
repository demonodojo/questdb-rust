//! Read-only access to a QuestDB 10.1.0 native table directory.
//!
//! Supported layout, verified against tables written by the 10.1.0 binary
//! (`e11c837f8a169b533540bd6fa57ef3d520b68ad9`):
//! `PARTITION BY NONE` and `PARTITION BY DAY`, WAL disabled, column files
//! without a name-txn suffix. A day directory is `YYYY-MM-DD`, plus `.{nameTxn}`
//! when that field is greater than -1.
//!
//! The reader loads every committed row, in partition order, into memory.
//! The last partition uses `transient_row_count`. It refuses WAL tables, other
//! partition schemes, parquet or composite partitions, deleted columns and
//! column types it cannot decode.

mod bytes;
mod column;
mod meta;
mod table;
mod txn;

pub use meta::{ColumnMeta, TableMeta};
pub use table::{open_table, Table};
pub use txn::TxnHeader;
