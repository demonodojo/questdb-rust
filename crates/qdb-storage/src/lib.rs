//! Read-only access to a QuestDB 10.1.0 native table directory.
//!
//! Supported layout, verified against a table written by the 10.1.0 binary
//! (`e11c837f8a169b533540bd6fa57ef3d520b68ad9`):
//! `PARTITION BY NONE`, WAL disabled, column files without a name-txn suffix.
//!
//! The reader loads every committed row of that single `default` partition into
//! memory. It refuses WAL tables, other partition schemes, deleted columns and
//! column types it cannot decode.

mod bytes;
mod column;
mod meta;
mod table;
mod txn;

pub use meta::{ColumnMeta, TableMeta};
pub use table::{open_table, Table};
pub use txn::TxnHeader;
