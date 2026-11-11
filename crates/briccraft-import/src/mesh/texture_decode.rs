use crate::{error::invalid, limits::limit, ImportError, ImportResult};
use std::io::Cursor;

pub(super) struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

fn capacity(width: usize, height: usize, maximum: usize) -> ImportResult<usize> {
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(invalid(
            "embedded texture dimensions must be between 1 and 8192",
        ));
    }
    let bytes = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| invalid("decoded texture size overflow"))?;
    limit("decoded texture bytes", bytes, maximum)?;
    Ok(bytes)
}

pub(super) fn decode(data: &[u8], maximum: usize) -> ImportResult<Image> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        png(data, maximum)
    } else if data.starts_with(&[0xff, 0xd8]) {
        jpeg(data, maximum)
    } else {
        Err(ImportError::Unsupported(
            "embedded base-color textures must be PNG or JPEG".into(),
        ))
    }
}

fn png(data: &[u8], maximum: usize) -> ImportResult<Image> {
    let mut decoder =
        png::Decoder::new_with_limits(Cursor::new(data), png::Limits { bytes: maximum });
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_ignore_text_chunk(true);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let header = decoder
        .read_header_info()
        .map_err(|e| invalid(format!("PNG header: {e}")))?;
    capacity(header.width as usize, header.height as usize, maximum)?;
    let mut reader = decoder
        .read_info()
        .map_err(|e| invalid(format!("PNG metadata: {e}")))?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| invalid("PNG output buffer size overflow"))?;
    limit("decoded texture bytes", size, maximum)?;
    let mut pixels = vec![0; size];
    let frame = reader
        .next_frame(&mut pixels)
        .map_err(|e| invalid(format!("PNG pixels: {e}")))?;
    pixels.truncate(frame.buffer_size());
    let (width, height) = (frame.width as usize, frame.height as usize);
    let channels = match frame.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return Err(invalid("PNG palette expansion failed")),
    };
    let rgb = if channels == 3 {
        pixels
    } else {
        let mut rgb = Vec::with_capacity(width * height * 3);
        for p in pixels.chunks_exact(channels) {
            if channels < 3 {
                rgb.extend([p[0]; 3]);
            } else {
                rgb.extend_from_slice(&p[..3]);
            }
        }
        rgb
    };
    Ok(Image { width, height, rgb })
}

fn jpeg(data: &[u8], maximum: usize) -> ImportResult<Image> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(data));
    decoder.set_max_decoding_buffer_size(maximum);
    decoder
        .read_info()
        .map_err(|e| invalid(format!("JPEG header: {e}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| invalid("JPEG has no image header"))?;
    let (width, height) = (info.width as usize, info.height as usize);
    capacity(width, height, maximum)?;
    if !matches!(
        info.pixel_format,
        jpeg_decoder::PixelFormat::L8 | jpeg_decoder::PixelFormat::RGB24
    ) {
        return Err(ImportError::Unsupported(
            "JPEG textures must contain 8-bit grayscale or RGB pixels".into(),
        ));
    }
    let pixels = decoder
        .decode()
        .map_err(|e| invalid(format!("JPEG pixels: {e}")))?;
    let rgb = if info.pixel_format == jpeg_decoder::PixelFormat::L8 {
        pixels.into_iter().flat_map(|value| [value; 3]).collect()
    } else {
        pixels
    };
    Ok(Image { width, height, rgb })
}

#[cfg(test)]
mod tests {
    #[test]
    fn png_and_jpeg_decode_with_budget_enforcement() {
        let mut png = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut png, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[196, 40, 28, 255, 13, 105, 171, 255])
                .unwrap();
        }
        let image = super::decode(&png, 4096).unwrap();
        assert_eq!(image.rgb, [196, 40, 28, 13, 105, 171]);
        assert!(super::decode(&png, 4).is_err());
        let mut jpeg = Vec::new();
        jpeg_encoder::Encoder::new(&mut jpeg, 100)
            .encode(
                &[196, 40, 28, 196, 40, 28],
                2,
                1,
                jpeg_encoder::ColorType::Rgb,
            )
            .unwrap();
        let image = super::decode(&jpeg, 4096).unwrap();
        assert_eq!((image.width, image.height), (2, 1));
        assert!(image.rgb[0] > 190 && image.rgb[1] < 45);
        assert!(super::decode(&jpeg, 4).is_err());
    }
}
