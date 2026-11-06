use qdb_types::Error;

pub fn need(buf: &[u8], offset: usize, len: usize, what: &str) -> Result<(), Error> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| Error::new(format!("{what}: offset overflow at {offset}")))?;
    if end > buf.len() {
        return Err(Error::new(format!(
            "{what}: need {len} bytes at {offset}, file has {}",
            buf.len()
        )));
    }
    Ok(())
}

pub fn i32_at(buf: &[u8], offset: usize, what: &str) -> Result<i32, Error> {
    need(buf, offset, 4, what)?;
    Ok(i32::from_le_bytes(
        buf[offset..offset + 4].try_into().unwrap(),
    ))
}

pub fn i64_at(buf: &[u8], offset: usize, what: &str) -> Result<i64, Error> {
    need(buf, offset, 8, what)?;
    Ok(i64::from_le_bytes(
        buf[offset..offset + 8].try_into().unwrap(),
    ))
}

pub fn u16_at(buf: &[u8], offset: usize, what: &str) -> Result<u16, Error> {
    need(buf, offset, 2, what)?;
    Ok(u16::from_le_bytes(
        buf[offset..offset + 2].try_into().unwrap(),
    ))
}

/// QuestDB `putStr`: i32 UTF-16 length, then little-endian `char`s. `-1` is null.
pub fn utf16_str_at(
    buf: &[u8],
    offset: usize,
    what: &str,
) -> Result<(Option<String>, usize), Error> {
    let len = i32_at(buf, offset, what)?;
    if len == qdb_types::NULL_STRING_LEN {
        return Ok((None, 4));
    }
    if len < 0 {
        return Err(Error::new(format!(
            "{what}: invalid string length {len} at {offset}"
        )));
    }
    let chars = len as usize;
    let bytes = chars
        .checked_mul(2)
        .ok_or_else(|| Error::new(format!("{what}: string length overflow")))?;
    need(buf, offset + 4, bytes, what)?;
    let mut units = Vec::with_capacity(chars);
    for i in 0..chars {
        units.push(u16_at(buf, offset + 4 + i * 2, what)?);
    }
    let text = String::from_utf16(&units)
        .map_err(|_| Error::new(format!("{what}: invalid UTF-16 at {offset}")))?;
    Ok((Some(text), 4 + bytes))
}
