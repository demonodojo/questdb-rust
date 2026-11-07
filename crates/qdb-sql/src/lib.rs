//! A deliberately small SQL surface for the H1 read slice.
//!
//! Accepted form:
//! `SELECT <cols>|*|<aggregate>|<col>, <aggregate> FROM <table> [WHERE <col> <op> <literal>] [GROUP BY <col>] [ORDER BY <col>|<aggregate> [ASC|DESC] [, ...]] [LIMIT n | LIMIT lo, hi]`
//!
//! An aggregate is `count()`, `count(<col>)`, `min(<col>)`, `max(<col>)`, `sum(<col>)`, `avg(<col>)`, `first(<col>)`, `last(<col>)` or `count_distinct(<col>)`.
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
//! becomes `0.0`. `avg` divides that sum by the number of values and always
//! returns `DOUBLE`. A `BOOLEAN` counts as `0.0` or `1.0`. `first` and `last`
//! keep the value of the earliest and latest row in read order, including null
//! and a `CHAR` 0, and keep the column type. `count_distinct` skips nulls and a
//! `CHAR` 0, counts an empty string and a numeric zero, and treats `-0.0` as
//! different from `0.0`. With no values it returns 0. Inside `GROUP BY` it has
//! no `FLOAT`, `DOUBLE` or `BOOLEAN` overload. `concat` joins two or more
//! values into a string. A text null is empty, a numeric null is `null`, and a
//! null float is `NaN`. `length` returns the character count as `INT`, or `-1`
//! when the text is null. `lower` returns the text in lowercase, or NULL when
//! the text is null. `upper` returns the text in uppercase, or NULL when the
//! text is null. `left` and `right` keep that many characters. A negative
//! count drops characters from the other end, and a null text or count stays
//! NULL. `substring` takes a 1-based start and a length. A constant length of 0
//! is empty, and a constant start that ends before the text is NULL. A negative
//! length is rejected. `trim` removes ASCII spaces from both ends and keeps a
//! null text null. `ltrim` and `rtrim` do the same from the left or the right.
//! `replace` substitutes non-overlapping matches. A null argument stays null,
//! and an empty search leaves the text unchanged. `starts_with` returns a
//! boolean. A null text or prefix is false, and an empty prefix matches any
//! non-null text. `strpos` and `position` return the 1-based index of the first
//! match, `0` when it is missing, and NULL when the text or the search is
//! null. `sign` keeps the numeric type. A `DOUBLE` of `-0.0` becomes `0.0`.
//! A null literal becomes the byte `0`. `ceil` and `ceiling` return `FLOAT` for `BYTE`, `SHORT`, `INT` and
//! `FLOAT`, and `DOUBLE` for `LONG` and `DOUBLE`. `-0.0` keeps its sign.
//! `floor` returns `FLOAT` for `BYTE`, `SHORT`, `INT` and `FLOAT`, and
//! `DOUBLE` for `LONG` and `DOUBLE`. `-0.0` keeps its sign. `round` returns `DOUBLE`. With no scale it rounds half toward positive
//! infinity. A non-zero scale rounds half away from zero.
//! `abs` returns the magnitude and keeps an integer type, except a `BYTE`,
//! which becomes `SHORT`, and a `FLOAT`, which becomes `DOUBLE`.
//! `size_pretty` prints a byte count with one decimal and a binary unit.
//! `length_bytes` counts UTF-8 bytes and returns `-1` for null text.
//! `quote_ident` quotes a text only when it is not an identifier.
//! `lpad` and `rpad` pad or trim to a width. Two arguments pad with spaces.
//! A negative width is NULL. `to_lowercase` and `to_uppercase` are the same functions as `lower`
//! and `upper`. `split_part` returns one field around a delimiter. Index 1 is the
//! first field and a negative index counts from the end. Index 0 is rejected.
//! An aggregate without `GROUP BY` returns one
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
