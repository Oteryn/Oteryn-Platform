use super::{Error, invalid};
use image::{Rgba, RgbaImage};
use std::io::{self, Cursor, Write};

pub(super) const EDGE: u32 = 384;

pub(super) fn geometry(kind: u32) -> Result<(u32, u32), Error> {
    match kind {
        0 => Ok((32, 32)),
        1 => Ok((32, 64)),
        2 => Ok((64, 32)),
        3 => Ok((64, 64)),
        _ => Err(invalid("unknown sprite layout")),
    }
}

pub(super) fn decode(encoded: &[u8]) -> Result<RgbaImage, Error> {
    if encoded.len() <= 45 {
        return Err(invalid("truncated sprite sheet"));
    }
    let mut payload = encoded[32..].to_vec();
    let dictionary = u32::from_le_bytes(
        payload[1..5]
            .try_into()
            .map_err(|_| invalid("LZMA header"))?,
    );
    if dictionary > 64 * 1024 * 1024 {
        return Err(invalid("LZMA dictionary exceeds limit"));
    }
    // The source size field is unreliable; the game reader uses the end marker.
    payload[5..13].fill(0xff);
    let mut output = Bounded(Vec::new());
    lzma_rs::lzma_decompress(&mut Cursor::new(payload), &mut output)
        .map_err(|e| invalid(format!("invalid LZMA sheet: {e}")))?;
    bmp(&output.0)
}

fn bmp(data: &[u8]) -> Result<RgbaImage, Error> {
    if data.len() < 54 || &data[..2] != b"BM" {
        return Err(invalid("invalid BMP header"));
    }
    let word = |offset: usize| -> Result<u32, Error> {
        let bytes = data
            .get(offset..offset + 4)
            .ok_or_else(|| invalid("truncated BMP"))?;
        Ok(u32::from_le_bytes(
            bytes.try_into().map_err(|_| invalid("BMP word"))?,
        ))
    };
    let offset = word(10)? as usize;
    let dib = word(14)? as usize;
    let height = word(22)? as i32;
    let compression = word(30)?;
    if word(18)? != EDGE
        || height.unsigned_abs() != EDGE
        || data[26..28] != [1, 0]
        || data[28..30] != [32, 0]
        || ![0, 3].contains(&compression)
        || dib < 40
    {
        return Err(invalid("unsupported BMP layout"));
    }
    let header_end = 14_usize
        .checked_add(dib)
        .ok_or_else(|| invalid("BMP header overflow"))?;
    let minimum = if compression == 3 {
        header_end.max(70)
    } else {
        header_end
    };
    if offset < minimum
        || offset
            .checked_add((EDGE * EDGE * 4) as usize)
            .is_none_or(|end| end > data.len())
    {
        return Err(invalid("truncated or overlapping BMP pixels"));
    }
    if compression == 3
        && [word(54)?, word(58)?, word(62)?, word(66)?]
            != [0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000]
    {
        return Err(invalid("unsupported BMP masks"));
    }
    let mut image = RgbaImage::new(EDGE, EDGE);
    for y in 0..EDGE {
        let row = if height < 0 { y } else { EDGE - 1 - y };
        for x in 0..EDGE {
            let at = offset + ((row * EDGE + x) * 4) as usize;
            let [b, g, r, a] = [data[at], data[at + 1], data[at + 2], data[at + 3]];
            image.put_pixel(
                x,
                y,
                Rgba(if [r, g, b] == [255, 0, 255] {
                    [0; 4]
                } else {
                    [r, g, b, a]
                }),
            );
        }
    }
    Ok(image)
}

struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(bytes.len()) > 1024 * 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "sheet exceeds decoded limit",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
