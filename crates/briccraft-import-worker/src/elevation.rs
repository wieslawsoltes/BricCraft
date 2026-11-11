//! Bounded PNG transport adapter; the reusable terrain crate owns geodesy and interpolation.
use briccraft_terrain::{ElevationSource, GeoBounds, TerrariumMosaic, TerrariumTile, VerticalDatum};
use serde::Deserialize;
use std::io::Cursor;

const MAX_PACKED_BYTES: usize = 12 * 1024 * 1024;
const MAX_TILE_BYTES: usize = 1024 * 1024;
const TILE_SIDE: u32 = 256;

#[derive(Deserialize)]
struct TileRange {
    z: u8,
    x: u32,
    y: u32,
    offset: usize,
    length: usize,
}

#[derive(Deserialize)]
struct ElevationRequest {
    bounds: GeoBounds,
    width: u32,
    height: u32,
    tiles: Vec<TileRange>,
    source: ElevationSource,
    #[serde(default)]
    vertical_datum: VerticalDatum,
}

pub fn elevation_grid_json(metadata: &str, packed: &[u8]) -> Result<String, String> {
    if metadata.len() > 32 * 1024 || packed.len() > MAX_PACKED_BYTES {
        return Err("Elevation input exceeds its metadata or 12 MB tile budget".into());
    }
    let request: ElevationRequest = serde_json::from_str(metadata).map_err(|e| e.to_string())?;
    if request.tiles.is_empty() || request.tiles.len() > 16
        || !(2..=513).contains(&request.width) || !(2..=513).contains(&request.height)
    {
        return Err("Elevation needs 1–16 tiles and a 2–513 sample grid on each axis".into());
    }
    request.bounds.validate().map_err(|e| e.to_string())?;
    request.source.validate().map_err(|e| e.to_string())?;
    let mut tiles = Vec::with_capacity(request.tiles.len());
    let mut consumed = 0;
    for tile in request.tiles {
        if tile.z > 15 || tile.offset != consumed || tile.length == 0 || tile.length > MAX_TILE_BYTES {
            return Err("Invalid elevation tile zoom, range, or 1 MB tile budget".into());
        }
        let end = tile.offset.checked_add(tile.length).ok_or("Elevation range overflow")?;
        let bytes = packed.get(tile.offset..end).ok_or("Elevation range exceeds the packed bytes")?;
        let rgb = decode_rgb(bytes)?;
        tiles.push(TerrariumTile::from_rgb(tile.z, tile.x, tile.y, TILE_SIDE, TILE_SIDE, &rgb)
            .map_err(|e| e.to_string())?);
        consumed = end;
    }
    if consumed != packed.len() {
        return Err("Unreferenced bytes in the elevation tile bundle".into());
    }
    let mosaic = TerrariumMosaic::new(tiles).map_err(|e| e.to_string())?;
    let grid = mosaic.sample_grid(request.bounds, request.width, request.height,
        request.source, request.vertical_datum).map_err(|e| e.to_string())?;
    serde_json::to_string(&grid).map_err(|e| e.to_string())
}

fn decode_rgb(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = png::Decoder::new_with_limits(Cursor::new(bytes), png::Limits { bytes: MAX_TILE_BYTES });
    decoder.set_ignore_iccp_chunk(true);
    decoder.set_ignore_text_chunk(true);
    // Gamma/profile conversion would corrupt elevations. EXPAND only decodes
    // palette/transparency storage; channel values remain exact integers.
    decoder.set_transformations(png::Transformations::EXPAND);
    let header = decoder.read_header_info().map_err(|e| format!("Elevation PNG header: {e}"))?;
    if header.width != TILE_SIDE || header.height != TILE_SIDE || header.bit_depth != png::BitDepth::Eight {
        return Err("Terrarium sources must be 256×256, 8-bit PNG tiles".into());
    }
    let mut reader = decoder.read_info().map_err(|e| format!("Elevation PNG metadata: {e}"))?;
    let size = reader.output_buffer_size().ok_or("Elevation PNG output size overflow")?;
    if size > TILE_SIDE as usize * TILE_SIDE as usize * 4 {
        return Err("Elevation PNG decoded size exceeds one tile".into());
    }
    let mut pixels = vec![0; size];
    let frame = reader.next_frame(&mut pixels).map_err(|e| format!("Elevation PNG data: {e}"))?;
    pixels.truncate(frame.buffer_size());
    match frame.color_type {
        png::ColorType::Rgb => Ok(pixels),
        png::ColorType::Rgba => {
            let mut rgb = Vec::with_capacity(TILE_SIDE as usize * TILE_SIDE as usize * 3);
            for pixel in pixels.chunks_exact(4) {
                if pixel[3] != 255 { return Err("Elevation PNG contains missing/transparent samples".into()); }
                rgb.extend_from_slice(&pixel[..3]);
            }
            Ok(rgb)
        }
        _ => Err("Terrarium elevation needs raw RGB channels".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use briccraft_terrain::ElevationGrid;

    fn png(color: png::ColorType, pixel: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, TILE_SIDE, TILE_SIDE);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&pixel.repeat((TILE_SIDE * TILE_SIDE) as usize)).unwrap();
        }
        bytes
    }

    fn metadata(length: usize) -> serde_json::Value {
        serde_json::json!({
            "bounds":{"south":-1.0,"west":1.0,"north":-0.99,"east":1.01},
            "width":3,"height":3,"tiles":[{"z":1,"x":1,"y":1,"offset":0,"length":length}],
            "vertical_datum":"Unknown",
            "source":{"name":"test","url":"https://example.test","attribution":"test source",
                "license":"test","license_url":"https://example.test/license"}
        })
    }

    #[test]
    fn png_channels_preserve_fractional_and_negative_heights() {
        for (channels, height) in [([128,123,128],123.5),([127,254,64],-1.75)] {
            let bytes = png(png::ColorType::Rgb, &channels);
            let source = elevation_grid_json(&metadata(bytes.len()).to_string(), &bytes).unwrap();
            let grid: ElevationGrid = serde_json::from_str(&source).unwrap();
            assert!(grid.values.iter().all(|value| *value == Some(height)));
            assert_eq!(grid.vertical_datum, VerticalDatum::Unknown);
        }
    }

    #[test]
    fn invalid_ranges_dimensions_and_missing_channels_fail_before_import() {
        let bytes = png(png::ColorType::Rgb, &[128,0,0]);
        let mut request = metadata(bytes.len());
        request["tiles"][0]["offset"] = 1.into();
        assert!(elevation_grid_json(&request.to_string(), &bytes).is_err());
        request = metadata(bytes.len()); request["width"] = 514.into();
        assert!(elevation_grid_json(&request.to_string(), &bytes).is_err());
        let transparent = png(png::ColorType::Rgba, &[128,0,0,0]);
        assert!(decode_rgb(&transparent).unwrap_err().contains("missing"));
        assert!(decode_rgb(b"not a PNG").is_err());
    }
}
