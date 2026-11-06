//! Column vectors. Fixed-width values are dense. STRING uses `.i` offsets and a
//! UTF-16 `.d` payload. VARCHAR uses the 16-byte auxiliary record from
//! `VarcharTypeDriver`. SYMBOL keys index the table-level `.c` / `.o` map.

use crate::bytes::{i32_at, i64_at, need, u16_at, utf16_str_at};
use crate::meta::ColumnMeta;
use qdb_types::{Error, Scalar, INT_NULL, LONG_NULL};

const VARCHAR_AUX_WIDTH: usize = 16;
const VARCHAR_FLAG_NULL: i32 = 4;
const VARCHAR_FLAG_INLINED: i32 = 1;
const VARCHAR_MAX_INLINED: i32 = 9;
const SYMBOL_HEADER: usize = 64;

pub fn read_column(
    meta: &ColumnMeta,
    rows: usize,
    data: &[u8],
    aux: Option<&[u8]>,
    symbols: Option<&[String]>,
) -> Result<Vec<Scalar>, Error> {
    match meta.tag {
        qdb_types::TAG_BOOLEAN => read_bool(data, rows, &meta.name),
        qdb_types::TAG_BYTE => read_fixed(data, rows, 1, &meta.name, |b, at| {
            Ok(Scalar::Byte(b[at] as i8))
        }),
        qdb_types::TAG_SHORT => read_fixed(data, rows, 2, &meta.name, |b, at| {
            let v = i16::from_le_bytes(b[at..at + 2].try_into().unwrap());
            Ok(Scalar::Short(v))
        }),
        qdb_types::TAG_CHAR => read_fixed(data, rows, 2, &meta.name, |b, at| {
            Ok(Scalar::Char(u16_at(b, at, "char")?))
        }),
        qdb_types::TAG_INT => read_fixed(data, rows, 4, &meta.name, |b, at| {
            let v = i32_at(b, at, "int")?;
            Ok(if v == INT_NULL {
                Scalar::Null
            } else {
                Scalar::Int(v)
            })
        }),
        qdb_types::TAG_SYMBOL => {
            let symbols =
                symbols.ok_or_else(|| Error::new(format!("{}: missing symbol map", meta.name)))?;
            read_fixed(data, rows, 4, &meta.name, |b, at| {
                let key = i32_at(b, at, "symbol key")?;
                if key == INT_NULL {
                    return Ok(Scalar::Null);
                }
                if key < 0 {
                    return Err(Error::new(format!(
                        "{}: invalid symbol key {key}",
                        meta.name
                    )));
                }
                let key = key as usize;
                symbols.get(key).cloned().map(Scalar::Text).ok_or_else(|| {
                    Error::new(format!(
                        "{}: symbol key {key} is outside the map of {} values",
                        meta.name,
                        symbols.len()
                    ))
                })
            })
        }
        qdb_types::TAG_LONG => read_i64_column(data, rows, &meta.name, Scalar::Long),
        qdb_types::TAG_DATE => read_i64_column(data, rows, &meta.name, Scalar::Date),
        qdb_types::TAG_TIMESTAMP => read_i64_column(data, rows, &meta.name, Scalar::Timestamp),
        qdb_types::TAG_FLOAT => read_fixed(data, rows, 4, &meta.name, |b, at| {
            let v = f32::from_le_bytes(b[at..at + 4].try_into().unwrap());
            Ok(if v.is_nan() {
                Scalar::Null
            } else {
                Scalar::Float(v)
            })
        }),
        qdb_types::TAG_DOUBLE => read_fixed(data, rows, 8, &meta.name, |b, at| {
            let v = f64::from_le_bytes(b[at..at + 8].try_into().unwrap());
            Ok(if v.is_nan() {
                Scalar::Null
            } else {
                Scalar::Double(v)
            })
        }),
        qdb_types::TAG_STRING => read_string(data, aux, rows, &meta.name),
        qdb_types::TAG_VARCHAR => read_varchar(data, aux, rows, &meta.name),
        other => Err(Error::new(format!(
            "{}: type tag {other} is not readable",
            meta.name
        ))),
    }
}

fn read_i64_column(
    data: &[u8],
    rows: usize,
    name: &str,
    wrap: fn(i64) -> Scalar,
) -> Result<Vec<Scalar>, Error> {
    read_fixed(data, rows, 8, name, |b, at| {
        let v = i64_at(b, at, name)?;
        Ok(if v == LONG_NULL {
            Scalar::Null
        } else {
            wrap(v)
        })
    })
}

fn read_bool(data: &[u8], rows: usize, name: &str) -> Result<Vec<Scalar>, Error> {
    read_fixed(data, rows, 1, name, |b, at| match b[at] {
        0 => Ok(Scalar::Boolean(false)),
        1 => Ok(Scalar::Boolean(true)),
        other => Err(Error::new(format!(
            "{name}: boolean byte {other} at row offset {at} is neither 0 nor 1"
        ))),
    })
}

fn read_fixed<F>(
    data: &[u8],
    rows: usize,
    width: usize,
    name: &str,
    decode: F,
) -> Result<Vec<Scalar>, Error>
where
    F: Fn(&[u8], usize) -> Result<Scalar, Error>,
{
    let bytes = rows
        .checked_mul(width)
        .ok_or_else(|| Error::new(format!("{name}: column size overflow")))?;
    need(data, 0, bytes, name)?;
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        out.push(decode(data, row * width)?);
    }
    Ok(out)
}

fn read_string(
    data: &[u8],
    aux: Option<&[u8]>,
    rows: usize,
    name: &str,
) -> Result<Vec<Scalar>, Error> {
    let aux = aux.ok_or_else(|| Error::new(format!("{name}: missing .i offset file")))?;
    let slots = rows
        .checked_add(1)
        .ok_or_else(|| Error::new(format!("{name}: row count overflow")))?;
    let aux_bytes = slots
        .checked_mul(8)
        .ok_or_else(|| Error::new(format!("{name}: offset file size overflow")))?;
    need(aux, 0, aux_bytes, name)?;
    let mut offsets = Vec::with_capacity(slots);
    for slot in 0..slots {
        let offset = i64_at(aux, slot * 8, name)?;
        if offset < 0 {
            return Err(Error::new(format!(
                "{name}: negative string offset {offset}"
            )));
        }
        offsets.push(offset as usize);
    }
    if offsets[0] != 0 {
        return Err(Error::new(format!(
            "{name}: string offset file does not start at 0"
        )));
    }
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let start = offsets[row];
        let end = offsets[row + 1];
        if end < start || end > data.len() {
            return Err(Error::new(format!(
                "{name}: string span {start}..{end} is outside the data file ({})",
                data.len()
            )));
        }
        let (text, used) = utf16_str_at(data, start, name)?;
        if start + used != end {
            return Err(Error::new(format!(
                "{name}: string storage length {used} does not match offset span {} at row {row}",
                end - start
            )));
        }
        out.push(match text {
            Some(text) => Scalar::Text(text),
            None => Scalar::Null,
        });
    }
    Ok(out)
}

fn read_varchar(
    data: &[u8],
    aux: Option<&[u8]>,
    rows: usize,
    name: &str,
) -> Result<Vec<Scalar>, Error> {
    let aux = aux.ok_or_else(|| Error::new(format!("{name}: missing varchar .i file")))?;
    let aux_bytes = rows
        .checked_mul(VARCHAR_AUX_WIDTH)
        .ok_or_else(|| Error::new(format!("{name}: varchar aux size overflow")))?;
    need(aux, 0, aux_bytes, name)?;
    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let at = row * VARCHAR_AUX_WIDTH;
        let raw = i32_at(aux, at, name)?;
        if raw == 0 {
            return Err(Error::new(format!(
                "{name}: varchar header at row {row} is uninitialized"
            )));
        }
        if raw & VARCHAR_FLAG_NULL != 0 {
            out.push(Scalar::Null);
            continue;
        }
        if raw & VARCHAR_FLAG_INLINED != 0 {
            let size = (raw >> 4) & 0xF;
            if size > VARCHAR_MAX_INLINED {
                return Err(Error::new(format!(
                    "{name}: inlined varchar size {size} at row {row}"
                )));
            }
            let start = at + 1;
            let size = size as usize;
            need(aux, start, size, name)?;
            let text = std::str::from_utf8(&aux[start..start + size])
                .map_err(|_| Error::new(format!("{name}: varchar row {row} is not UTF-8")))?;
            out.push(Scalar::Text(text.to_string()));
            continue;
        }
        let size = (raw >> 4) & 0x0FFF_FFFF;
        if size < 0 {
            return Err(Error::new(format!(
                "{name}: negative varchar size at row {row}"
            )));
        }
        let word = i64_at(aux, at + 8, name)? as u64;
        let data_offset = (word >> 16) as usize;
        let size = size as usize;
        need(data, data_offset, size, name)?;
        let text = std::str::from_utf8(&data[data_offset..data_offset + size])
            .map_err(|_| Error::new(format!("{name}: varchar row {row} is not UTF-8")))?;
        out.push(Scalar::Text(text.to_string()));
    }
    Ok(out)
}

pub fn read_symbol_map(
    chars: &[u8],
    offsets: &[u8],
    count: i32,
    name: &str,
) -> Result<Vec<String>, Error> {
    if count < 0 {
        return Err(Error::new(format!("{name}: negative symbol count {count}")));
    }
    let count = count as usize;
    let mut values = Vec::with_capacity(count);
    for key in 0..count {
        let at = SYMBOL_HEADER + key * 8;
        let start = i64_at(offsets, at, name)?;
        if start < 0 {
            return Err(Error::new(format!(
                "{name}: negative symbol offset {start}"
            )));
        }
        let (text, _) = utf16_str_at(chars, start as usize, name)?;
        let text = text.ok_or_else(|| {
            Error::new(format!("{name}: symbol key {key} points at a NULL string"))
        })?;
        values.push(text);
    }
    Ok(values)
}
