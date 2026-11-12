//! Bounded Shortbread MVT decoding for live OSM-backed regions.
//! Provider tiles contain generalized, sometimes clipped geometry; absent tags
//! remain absent and inferred building heights are recorded by the OSM converter.
mod geometry;
mod protobuf;
mod shortbread;

#[cfg(test)]
mod tests;

use crate::{
    error::invalid,
    limits::{limit, progress},
    osm, ImportPlan, ImportProgress, ImportReport, ImportResult, ImportStage, OsmOptions,
    SourceKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TileRange {
    pub z: u8,
    pub x: u32,
    pub y: u32,
    pub offset: usize,
    pub length: usize,
}

pub fn import_shortbread(
    packed: &[u8],
    tiles: &[TileRange],
    options: &OsmOptions,
    callback: &mut dyn FnMut(ImportProgress) -> bool,
) -> ImportResult<ImportPlan> {
    options.bounds.validate()?;
    options.limits.input(packed.len())?;
    limit("vector tile bytes", packed.len(), 8 * 1024 * 1024)?;
    if tiles.is_empty() || tiles.len() > 16 {
        return Err(invalid("a mapped region requires 1–16 vector tiles"));
    }
    let mut consumed = 0;
    let mut seen = BTreeSet::new();
    let mut features = Vec::new();
    let mut vertices = 0;
    for (index, tile) in tiles.iter().enumerate() {
        if tile.z != 14
            || tile.x >= 1 << tile.z
            || tile.y >= 1 << tile.z
            || tile.offset != consumed
            || tile.length > 2 * 1024 * 1024
            || !seen.insert((tile.z, tile.x, tile.y))
        {
            return Err(invalid("invalid or duplicated zoom-14 vector tile range"));
        }
        let end = tile
            .offset
            .checked_add(tile.length)
            .ok_or_else(|| invalid("vector tile range overflow"))?;
        let bytes = packed
            .get(tile.offset..end)
            .ok_or_else(|| invalid("vector tile range exceeds packed bytes"))?;
        progress(callback, ImportStage::Parse, index, tiles.len())?;
        shortbread::tile(bytes, tile, &options.limits, &mut features, &mut vertices)?;
        consumed = end;
    }
    if consumed != packed.len() {
        return Err(invalid("unreferenced bytes in vector tile bundle"));
    }
    // Empty, valid tiles are meaningful. Missing required source tiles are not:
    // never publish a ground-only area when part of its map bundle is absent.
    let n = f64::from(1u32 << 14);
    let tile_x = |longitude: f64| (longitude + 180.0) / 360.0 * n;
    let tile_y = |latitude: f64| {
        (1.0 - latitude.to_radians().tan().asinh() / std::f64::consts::PI) * 0.5 * n
    };
    let west = tile_x(options.bounds.west).floor() as u32;
    let east = tile_x(options.bounds.east).ceil() as u32;
    let north = tile_y(options.bounds.north).floor() as u32;
    let south = tile_y(options.bounds.south).ceil() as u32;
    if u64::from(east - west) * u64::from(south - north) > 16 {
        return Err(invalid(
            "mapped region coverage exceeds sixteen vector tiles",
        ));
    }
    for y in north..south {
        for x in west..east {
            if !seen.contains(&(14, x, y)) {
                return Err(invalid(format!(
                    "vector tile 14/{x}/{y} required for the complete region is missing"
                )));
            }
        }
    }
    let mut report = ImportReport::new(SourceKind::OpenStreetMap);
    report.source_name = "OpenStreetMap Shortbread vector tiles".into();
    report.input_features = features.len();
    report.warn("Shortbread vector tiles provide mapped footprints, surface roads, water and land cover, generalized and clipped by their provider. They do not contain complete OSM tags, facade scans, underground routes or mechanical railway geometry. Missing heights and widths use explicitly reported defaults.");
    report.warn("Vector feature IDs are bundle-local identifiers, not OSM way or relation IDs. Height evidence identifies them as vector_tile.");
    features.sort_by_key(|f| match f.kind {
        osm::data::Kind::Landcover => 0,
        osm::data::Kind::Water => 1,
        osm::data::Kind::Road => 2,
        osm::data::Kind::Building => 3,
    });
    osm::import_features(features, options, report, callback)
}
