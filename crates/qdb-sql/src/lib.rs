//! A deliberately small SQL surface for the H1 read slice.
//!
//! Accepted form:
//! `SELECT <cols>|*|<aggregate>|<col>, <aggregate> FROM <table> [WHERE <col> <op> <literal>] [GROUP BY <col>] [ORDER BY <col>|<aggregate> [ASC|DESC] [, ...]] [LIMIT n | LIMIT lo, hi]`
//!
//! An aggregate is `count()`, `count(<col>)`, `min(<col>)`, `max(<col>)` or `sum(<col>)`.
//!
//! `LIMIT n` keeps the first `n` rows after `WHERE` and `ORDER BY`. A negative
//! `n` keeps the last `|n|` rows in that same order. `LIMIT lo, hi` keeps the
//! half-open interval, with negative bounds counted from the end. A negative
//! lower bound and a positive upper bound are rejected when there is no
//! `ORDER BY`, or when the only key is the designated timestamp. Any other
//! key list keeps the last `|lo|` rows. Equal keys keep their incoming order.
//! `count()` counts filtered rows. `count(col)` skips nulls, and also a `CHAR`
//! whose stored value is 0. `count` of a `BOOLEAN` is rejected. `min` and `max`
//! skip those same values and return NULL when every value was skipped. A
//! `BOOLEAN` argument becomes `0.0` or `1.0`. `-0.0` and `0.0` tie, and the
//! value already chosen stays. `sum` skips nulls and returns NULL when every
//! value was skipped. Integers, dates, timestamps and symbol keys sum to
//! `LONG`. `FLOAT` and `DOUBLE` keep their type. `-0.0` summed with `0.0`
//! becomes `0.0`. An aggregate without `GROUP BY` returns one
//! row, or none when `LIMIT` removes it.
//! `GROUP BY` of one column returns one row per distinct value, in first-seen
//! order until `ORDER BY` sorts it. `-0.0` and `0.0` stay in different groups.
//! Anything else, including `SELECT 1`, DML and joins, returns
//! `unsupported`.
//! Comparison with a NULL literal follows the QuestDB 10.1.0 result captured for
//! this fixture: `col = NULL` keeps nulls and `col != NULL` keeps non-nulls.
//! Inequalities with a null on either side do not match.

mod exec;
mod parse;

pub use exec::{execute, render_tsv, QueryResult};
pub use parse::Query;
