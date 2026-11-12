//! Small bounded reader for the public Mapbox Vector Tile 2.x protobuf wire format.
use crate::{error::invalid, ImportResult};

pub enum Value<'a> {
    Varint(u64),
    Fixed64([u8; 8]),
    Bytes(&'a [u8]),
    Fixed32([u8; 4]),
}

pub struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }
    pub fn empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    pub fn varint(&mut self) -> ImportResult<u64> {
        let mut value = 0u64;
        for shift in (0..70).step_by(7) {
            let b = *self
                .bytes
                .get(self.offset)
                .ok_or_else(|| invalid("truncated MVT varint"))?;
            self.offset += 1;
            if shift == 63 && b > 1 {
                return Err(invalid("MVT varint overflows 64 bits"));
            }
            value |= u64::from(b & 127) << shift;
            if b & 128 == 0 {
                return Ok(value);
            }
        }
        Err(invalid("unterminated MVT varint"))
    }

    fn take(&mut self, length: usize) -> ImportResult<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| invalid("MVT length overflows"))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| invalid("truncated MVT field"))?;
        self.offset = end;
        Ok(bytes)
    }

    pub fn field(&mut self) -> ImportResult<Option<(u32, Value<'a>)>> {
        if self.empty() {
            return Ok(None);
        }
        let key = self.varint()?;
        let number = u32::try_from(key >> 3).map_err(|_| invalid("MVT field number overflows"))?;
        if number == 0 {
            return Err(invalid("MVT field number zero is invalid"));
        }
        let value = match key & 7 {
            0 => Value::Varint(self.varint()?),
            1 => Value::Fixed64(self.take(8)?.try_into().unwrap()),
            2 => {
                let length = usize::try_from(self.varint()?)
                    .map_err(|_| invalid("MVT field is too large"))?;
                Value::Bytes(self.take(length)?)
            }
            5 => Value::Fixed32(self.take(4)?.try_into().unwrap()),
            _ => return Err(invalid("unsupported MVT protobuf wire type")),
        };
        Ok(Some((number, value)))
    }
}

pub fn string(bytes: &[u8]) -> ImportResult<String> {
    if bytes.len() > 4096 {
        return Err(invalid("MVT string exceeds 4096 bytes"));
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| invalid("MVT strings must be UTF-8"))
}

pub fn scalar(bytes: &[u8]) -> ImportResult<String> {
    let mut reader = Reader::new(bytes);
    let mut result = None;
    while let Some((number, value)) = reader.field()? {
        let current = match (number, value) {
            (1, Value::Bytes(v)) => Some(string(v)?),
            (2, Value::Fixed32(v)) => {
                let value = f32::from_le_bytes(v);
                if !value.is_finite() {
                    return Err(invalid("MVT float is not finite"));
                }
                Some(value.to_string())
            }
            (3, Value::Fixed64(v)) => {
                let value = f64::from_le_bytes(v);
                if !value.is_finite() {
                    return Err(invalid("MVT double is not finite"));
                }
                Some(value.to_string())
            }
            (4, Value::Varint(v)) => Some((v as i64).to_string()),
            (5, Value::Varint(v)) => Some(v.to_string()),
            (6, Value::Varint(v)) => Some(zigzag(v).to_string()),
            (7, Value::Varint(v)) if v <= 1 => Some((v == 1).to_string()),
            (1..=7, _) => return Err(invalid("MVT scalar has an invalid wire type")),
            _ => None,
        };
        if let Some(current) = current {
            if result.replace(current).is_some() {
                return Err(invalid("MVT value has multiple scalar fields"));
            }
        }
    }
    result.ok_or_else(|| invalid("MVT value has no supported scalar"))
}

pub fn zigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}
