//! Scalar types and on-disk null sentinels from QuestDB 10.1.0.
//!
//! Numeric tags match `io.questdb.cairo.ColumnType`. Null sentinels match
//! `TableWriter.configureNullSetters` and `io.questdb.std.Numbers`.

use std::fmt;

/// Metadata format version written by QuestDB 10.1.0 (`ColumnType.VERSION`).
pub const METADATA_VERSION_10_1_0: i32 = 426;

/// `PartitionBy.NONE`.
pub const PARTITION_BY_NONE: i32 = 3;

/// `TableUtils.NULL_LEN`.
pub const NULL_STRING_LEN: i32 = -1;

/// `Numbers.INT_NULL` and `SymbolTable.VALUE_IS_NULL`.
pub const INT_NULL: i32 = i32::MIN;

/// `Numbers.LONG_NULL`. Also the null sentinel for DATE and TIMESTAMP.
pub const LONG_NULL: i64 = i64::MIN;

pub const TAG_BOOLEAN: i32 = 1;
pub const TAG_BYTE: i32 = 2;
pub const TAG_SHORT: i32 = 3;
pub const TAG_CHAR: i32 = 4;
pub const TAG_INT: i32 = 5;
pub const TAG_LONG: i32 = 6;
pub const TAG_DATE: i32 = 7;
pub const TAG_TIMESTAMP: i32 = 8;
pub const TAG_FLOAT: i32 = 9;
pub const TAG_DOUBLE: i32 = 10;
pub const TAG_STRING: i32 = 11;
pub const TAG_SYMBOL: i32 = 12;
pub const TAG_VARCHAR: i32 = 26;

/// Nanosecond timestamp flag (`ColumnType.TIMESTAMP_NANO` uses bit 18).
pub const TIMESTAMP_NANO_FLAG: i32 = 1 << 18;

/// Low 8 bits of a column type. Negative types are deleted columns.
pub fn type_tag(column_type: i32) -> i32 {
    if column_type < 0 {
        -1
    } else {
        column_type & 0xFF
    }
}

pub fn type_name(tag: i32) -> &'static str {
    match tag {
        TAG_BOOLEAN => "BOOLEAN",
        TAG_BYTE => "BYTE",
        TAG_SHORT => "SHORT",
        TAG_CHAR => "CHAR",
        TAG_INT => "INT",
        TAG_LONG => "LONG",
        TAG_DATE => "DATE",
        TAG_TIMESTAMP => "TIMESTAMP",
        TAG_FLOAT => "FLOAT",
        TAG_DOUBLE => "DOUBLE",
        TAG_STRING => "STRING",
        TAG_SYMBOL => "SYMBOL",
        TAG_VARCHAR => "VARCHAR",
        _ => "UNSUPPORTED",
    }
}

/// A decoded column value. Float equality is bit-exact, so `-0.0` and `0.0` differ.
#[derive(Debug, Clone)]
pub enum Scalar {
    Null,
    Boolean(bool),
    Byte(i8),
    Short(i16),
    /// UTF-16 code unit. `0` is both SQL NULL and an empty char in QuestDB storage.
    Char(u16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// Milliseconds since Unix epoch. Not the null sentinel.
    Date(i64),
    /// Microseconds since Unix epoch. Not the null sentinel.
    Timestamp(i64),
    /// STRING, SYMBOL or VARCHAR text. Empty string is not null.
    Text(String),
}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Byte(a), Self::Byte(b)) => a == b,
            (Self::Short(a), Self::Short(b)) => a == b,
            (Self::Char(a), Self::Char(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Long(a), Self::Long(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Double(a), Self::Double(b)) => a.to_bits() == b.to_bits(),
            (Self::Date(a), Self::Date(b)) => a == b,
            (Self::Timestamp(a), Self::Timestamp(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Scalar {}

impl Scalar {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl fmt::Display for Scalar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::Boolean(v) => f.write_str(if *v { "true" } else { "false" }),
            Self::Byte(v) => write!(f, "{v}"),
            Self::Short(v) => write!(f, "{v}"),
            Self::Char(0) => Ok(()),
            Self::Char(v) => {
                let c = char::from_u32(u32::from(*v)).ok_or(fmt::Error)?;
                write!(f, "{c}")
            }
            Self::Int(v) => write!(f, "{v}"),
            Self::Long(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{}", format_f32(*v)),
            Self::Double(v) => write!(f, "{}", format_f64(*v)),
            Self::Date(v) => write!(f, "{}", format_date_millis(*v)),
            Self::Timestamp(v) => write!(f, "{}", format_timestamp_micros(*v)),
            Self::Text(v) => f.write_str(v),
        }
    }
}

pub fn format_f64(v: f64) -> String {
    if v.is_nan() {
        return "NULL".to_string();
    }
    if v == 0.0 {
        return if v.is_sign_negative() {
            "-0.0".to_string()
        } else {
            "0.0".to_string()
        };
    }
    let text = format!("{v}");
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

pub fn format_f32(v: f32) -> String {
    format_f64(f64::from(v))
}

pub fn format_timestamp_micros(us: i64) -> String {
    let (date, micros) = split_subseconds(us, 1_000_000);
    format!("{date}.{micros:06}Z")
}

pub fn format_date_millis(ms: i64) -> String {
    let (date, millis) = split_subseconds(ms, 1_000);
    format!("{date}.{millis:03}Z")
}

fn split_subseconds(value: i64, scale: i64) -> (String, u32) {
    let secs = value.div_euclid(scale);
    let frac = value.rem_euclid(scale) as u32;
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400) as u32;
    let (year, month, day) = civil_from_unix_days(days);
    let hh = sod / 3600;
    let mm = (sod % 3600) / 60;
    let ss = sod % 60;
    (
        format!("{year:04}-{month:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}"),
        frac,
    )
}

/// Days since 1970-01-01 to a civil date. Howard Hinnant's algorithm.
pub fn civil_from_unix_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (year as i32, m as u32, d)
}

/// Parse `YYYY-MM-DDTHH:MM:SS[.fraction]Z` into microseconds.
pub fn parse_timestamp_micros(text: &str) -> Option<i64> {
    parse_utc_scaled(text, 1_000_000)
}

/// Parse `YYYY-MM-DDTHH:MM:SS[.fraction]Z` into milliseconds.
pub fn parse_date_millis(text: &str) -> Option<i64> {
    parse_utc_scaled(text, 1_000)
}

fn parse_utc_scaled(text: &str, scale: i64) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, time) = text.split_once('T')?;
    let mut dates = date.split('-');
    let year: i32 = dates.next()?.parse().ok()?;
    let month: u32 = dates.next()?.parse().ok()?;
    let day: u32 = dates.next()?.parse().ok()?;
    if dates.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (hms, frac) = match time.split_once('.') {
        Some((hms, frac)) => (hms, frac),
        None => (time, ""),
    };
    let mut times = hms.split(':');
    let hour: u32 = times.next()?.parse().ok()?;
    let minute: u32 = times.next()?.parse().ok()?;
    let second: u32 = times.next()?.parse().ok()?;
    if times.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    if !frac.is_empty() && (frac.len() > 9 || !frac.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let days = unix_days_from_civil(year, month, day)?;
    let sod = i64::from(hour) * 3600 + i64::from(minute) * 60 + i64::from(second);
    let seconds = days.checked_mul(86_400)?.checked_add(sod)?;
    let frac_units = if frac.is_empty() {
        0
    } else {
        let padded = format!("{frac:0<9}");
        let nanos: i64 = padded.parse().ok()?;
        nanos / (1_000_000_000 / scale)
    };
    seconds.checked_mul(scale)?.checked_add(frac_units)
}

fn unix_days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }
    let y = i64::from(year) - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * u64::from(mp) + 2) / 5 + u64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe as i64 - 719_468)
}

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_roundtrip_matches_fixture_clock() {
        let us = 1_577_934_245_000_000;
        assert_eq!(format_timestamp_micros(us), "2020-01-02T03:04:05.000000Z");
        assert_eq!(
            parse_timestamp_micros("2020-01-02T03:04:05.000000Z"),
            Some(us)
        );
        assert_eq!(
            format_date_millis(1_577_923_200_000),
            "2020-01-02T00:00:00.000Z"
        );
        assert_eq!(format_date_millis(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn negative_zero_is_distinct_from_zero() {
        assert_ne!(Scalar::Double(-0.0), Scalar::Double(0.0));
        assert_eq!(format_f64(-0.0), "-0.0");
        assert_eq!(format_f64(0.0), "0.0");
        assert_eq!(format_f64(1.5), "1.5");
        assert_eq!(format_f32(1.25), "1.25");
    }

    #[test]
    fn type_tag_strips_flags_and_marks_deleted() {
        assert_eq!(type_tag(TAG_TIMESTAMP | (1 << 17)), TAG_TIMESTAMP);
        assert_eq!(type_tag(-1), -1);
    }
}
