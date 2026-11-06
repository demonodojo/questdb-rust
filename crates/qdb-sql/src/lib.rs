//! A deliberately small SQL surface for the H1 read slice.
//!
//! Accepted form:
//! `SELECT <cols>|* FROM <table> [WHERE <col> <op> <literal>] [ORDER BY <col> [ASC|DESC] [, ...]] [LIMIT n | LIMIT lo, hi]`
//!
//! `LIMIT n` keeps the first `n` rows after `WHERE` and `ORDER BY`. A negative
//! `n` keeps the last `|n|` rows in that same order. `LIMIT lo, hi` keeps the
//! half-open interval, with negative bounds counted from the end. A negative
//! lower bound and a positive upper bound are rejected when there is no
//! `ORDER BY`, or when the only key is the designated timestamp. Any other
//! key list keeps the last `|lo|` rows. Equal keys keep their incoming order.
//! Anything else, including `SELECT 1`, DML and joins, returns
//! `unsupported`.
//! Comparison with a NULL literal follows the QuestDB 10.1.0 result captured for
//! this fixture: `col = NULL` keeps nulls and `col != NULL` keeps non-nulls.
//! Inequalities with a null on either side do not match.

mod exec;
mod parse;

pub use exec::{execute, render_tsv, QueryResult};
pub use parse::Query;
