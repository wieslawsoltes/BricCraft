//! Decode bounded display-map PNGs through the framework's current Skia render interface.
use ferroui_base::media::{imaging::Bitmap, IImage};
use std::{io::Cursor, rc::Rc};

pub fn decode(bytes: &[u8]) -> Result<Rc<dyn IImage>, String> {
    if bytes.len() < 24
        || bytes.len() > 2 * 1024 * 1024
        || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
        || &bytes[12..16] != b"IHDR"
    {
        return Err("The map tile is not a bounded PNG image.".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if width != 256 || height != 256 {
        return Err("The map tile must be 256 × 256 pixels.".into());
    }
    let bitmap = Bitmap::from_stream(&mut Cursor::new(bytes))
        .map_err(|error| format!("Cannot decode the map tile: {error}"))?;
    let size = bitmap.pixel_size();
    if size.width != 256 || size.height != 256 {
        return Err("The decoded map tile has unexpected dimensions.".into());
    }
    Ok(Rc::new(bitmap))
}
