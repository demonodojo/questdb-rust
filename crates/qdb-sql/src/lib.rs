//! A deliberately small SQL surface for the H1 read slice.
//!
//! Accepted form:
//! `SELECT <cols>|* FROM <table> [WHERE <col> <op> <literal>] [ORDER BY <col> [ASC]]`
//!
//! Anything else, including `SELECT 1`, DML and joins, returns `unsupported`.
//! Comparison with a NULL literal follows the QuestDB 10.1.0 result captured for
//! this fixture: `col = NULL` keeps nulls and `col != NULL` keeps non-nulls.
//! Inequalities with a null on either side do not match.

mod exec;
mod parse;

pub use exec::{execute, render_tsv, QueryResult};
pub use parse::Query;
