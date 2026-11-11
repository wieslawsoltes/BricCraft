# Georeferenced brick terrain

`briccraft-terrain` is a browser-independent Rust library for elevation decoding,
geographic sampling, explicit vertical scaling and exact-cover terrain packing.
It has no networking, image decoder, filesystem or UI dependency. Hosts decode PNG
channels and supply source metadata; the same terrain code runs natively and in WASM.

## Elevation request

The OSM importer's optional `terrain` property accepts this schema:

```json
{
  "elevation": {
    "width": 2,
    "height": 2,
    "bounds": {"south": 37.801, "west": -122.407, "north": 37.803, "east": -122.404},
    "grid_space": "Geographic",
    "values": [25.0, 30.0, 15.0, 20.0],
    "vertical_datum": "Unknown",
    "source": {
      "name": "An attributed elevation survey",
      "url": "https://example.org/elevation",
      "attribution": "Elevation contributors",
      "license": "The source's actual license",
      "license_url": "https://example.org/license",
      "resolution_m": 30.0,
      "timestamp": null
    }
  },
  "vertical_exaggeration": 1.0,
  "fit_height": true,
  "auto_coarsen": true,
  "max_columns": 262144
}
```

Values are **meters**, row-major **north to south, west to east**. Bounds locate
the **first and last sample centers**, not the outer raster pixel edges. Both
dimensions must be at least two and there may be at most 1,048,576 samples.
`Geographic` rows are uniformly spaced in latitude; `WebMercator` rows are
uniformly spaced in Mercator Y. Sampling is bilinear in that declared grid space.

`null` means no data. A missing sample with a nonzero interpolation weight causes
an explicit error. Missing data is never treated as zero elevation and is not
silently filled. The raster must cover the complete requested map bounds. At the
outermost quantized terrain cells, the sample point is clamped to the requested
geographic box, by less than one horizontal stud; DEM values are never extrapolated.

`VerticalDatum` is `Egm96`, `Egm2008`, `Wgs84Ellipsoid` or `Unknown`. The library
preserves this evidence and performs no implicit geoid conversion. Mixed-source
datasets without a uniform verified datum must use `Unknown`. Source name and
attribution are required, metadata strings and resolution are validated, and all
source/license fields survive the import report.

## Raw elevation decoding

`decode_terrarium([r,g,b])` follows the source's
[Terrarium format](https://github.com/tilezen/joerd/blob/master/docs/formats.md):
`R × 256 + G + B / 256 − 32768` meters. Pass raw decoded PNG channels, without
display color-space conversion.

```rust
let tile = TerrariumTile::from_rgb(z, x, y, width, height, &raw_rgb)?;
let mosaic = TerrariumMosaic::new(tiles)?;
let grid = mosaic.sample_grid(bounds, width, height, source, VerticalDatum::Unknown)?;
```

Mosaics accept at most 64 same-zoom, same-dimension tiles and 4,194,304 decoded
pixels. The supported unbuffered tile dimensions are powers of two from 2 through
1024. Global WebMercator coordinates are offset by half a pixel to interpolate
pixel centers. Bilinear neighbors cross tile seams; a missing required neighbor
is an error. Hosts must fetch the one-pixel sampling halo, including adjacent tiles
when the requested boundary touches a tile edge. Geographic longitude wraps at
the tile grid; local map requests must still be split at the dateline.

`ElevationGrid::from_hgt` also decodes bounded/cropped signed big-endian 16-bit
height rasters, preserving the `−32768` void sentinel as `None`. The common
elevation-sample budget applies; larger source HGT tiles must first be cropped or
resampled by the host. The caller supplies bounds, provenance and datum explicitly.

## Full-area conversion and scale evidence

`HeightField` stores one exclusive top and surface material per X/Z column, not a
3D voxel allocation. It samples every column of the complete requested map extent.
The minimum sampled elevation becomes the reference elevation. The lowest ground
occupies one plate above the configured origin; absolute source elevation remains
in the report.

Without vertical fitting, the metric scale is
`meters_per_plate = meters_per_stud × 0.4 / vertical_exaggeration`.
`VerticalScale::fit` reserves enough space for measured relief, the tallest mapped
building and integer quantization. `fit_height: false` rejects overflow with a
suggested coarser horizontal scale. `fit_height: true` can increase meters per
plate, applying the same explicitly reported scale to terrain and buildings. It
does not change the physical height evidence stored for each OSM feature.

The OSM integration preflights the terrain column budget. With `auto_coarsen`, it
increases horizontal meters per stud and retries a complete conversion if feature,
packing, part or work budgets require it. There are at most nine bounded attempts.
The resulting report includes requested/effective horizontal and vertical scales,
coarsening and compression ratios, source min/max/reference elevation, column count,
filled-cell count, terrain part count and foundation/water inference counts.
No attempt returns a truncated area or partial successful plan.

## Terrain materials, structures and packing

Mapped land-cover polygons set the top plate's palette material. Roads and flowing
waterways follow the sampled terrain. Enclosed water polygons use the median
quantized DEM level inside the polygon; this is an explicitly reported inference,
not bathymetry or a measured water stage. `ele` tags are not mixed into an unknown
DEM datum. Building footprints keep their mapped outlines and holes. A level
foundation raises the terrain beneath each footprint to its highest sampled cell.

`pack_heightfield` covers the complete solid column volume with ordinary catalog
bricks and plates. Only the top plate uses the surface material; interior material
is earth. One consumed-height cursor per column makes auxiliary memory O(columns).
Parts never cross a signed 16×16-stud chunk boundary, so chunk-aligned resident
windows retain complete ground slabs. Packing has explicit part/work budgets and
supports cancellation; it never fills gaps or returns partial placements.

The importer combines those terrain parts with disjoint structure parts in one
ordinary `ImportPlan`. Existing streamed import validation publishes the plan
atomically, preserves it across distant windows and saves, and treats undo/redo as
one global construction change. No renderer-specific heightfield or special
uneditable terrain primitive is introduced.

## Focused validation

```sh
cargo test -p briccraft-terrain
cargo test -p briccraft-import --test terrain_import
```

Tests cover node orientation, bilinear weights, no-data rejection, declared Mercator
rows, Terrarium fractions and four-tile seams, HGT signed values/voids, projection
round trips, strict/fit vertical scales, exact nonoverlapping packing, chunk-boundary
alignment, full-area coarsening, terrain-aware OSM features and atomic streaming,
save/load and global undo/redo.
