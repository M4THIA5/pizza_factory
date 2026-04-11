use std::io::{self, Read, Write};

use ciborium::Value;

const MAX_FRAME: usize = 16 * 1024 * 1024;

pub fn write_frame(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let mut buf = Vec::new();
    ciborium::into_writer(v, &mut buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let len = u32::try_from(buf.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "frame CBOR trop grand"))?;
    w.write_all(&len.to_be_bytes())?;
    w.write_all(&buf)?;
    w.flush()?;
    Ok(())
}

pub fn read_frame(r: &mut impl Read) -> io::Result<Value> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("trame trop grande ({len} octets)"),
        ));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    ciborium::de::from_reader(&mut &buf[..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}
