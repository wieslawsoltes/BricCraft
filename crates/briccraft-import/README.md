# BricCraft import

A renderer-independent Rust library that converts mapped geometry and static 3D surfaces into catalog-valid brick placements. It has no FerroUI, browser, network or filesystem dependency. The same conversion runs in native tools and the application's dedicated Web Worker.

## Source fidelity

| Source | Geometry retained | Color and material handling | Explicit limits |
| --- | --- | --- | --- |
| OpenStreetMap JSON | Mapped building polygons, concave outlines, joined multipolygon rings and courtyards; road/water polylines and water polygons; tagged height and `min_height` | Building/roof color tags and explicit road widths; defaults are reported | Missing heights use tagged levels × 3 m or the configured default, with per-building evidence. No terrain elevation, facade scan, bridge elevation or detailed roof geometry is invented. |
| OBJ | Positive and relative indices, homogeneous vertices, continued lines, concave polygon triangulation | Vertex RGB; optional supplied MTL diffuse colors | External MTL/image files are not fetched. Free-form surfaces must be tessellated first. |
| glTF 2.0 / GLB | Scene hierarchy, matrix/TRS transforms, instanced static meshes, triangle lists/strips/fans, interleaved and sparse accessors | Base-color factors, linear vertex colors, embedded PNG/JPEG base-color textures, UV sets, `KHR_texture_transform`, repeat/mirror/clamp sampling | External buffers are rejected; external textures are reported and omitted. Draco/meshopt/quantized geometry requires an uncompressed export. Skinning and active morphs require a baked static mesh. Animations use the static default pose. |

Textures use bounded decoding and bilinear sampling in linear color space. Alpha coverage, clearcoat, transmission, lighting and normal-map effects become solid brick materials. Fine details and colors are quantized to the available grid and palette. Importing a mesh does not infer a vehicle rig or physical joints from its appearance.

OSM geometry is map data, not a source of absent 3D models. Imported buildings follow the source footprints; elevations marked `inferred: true` must not be presented as measured. The report retains the evidence for each height, its OSM ID, the projection, scale, source bounds, warnings and occupied-grid bounds.

## Grid and streaming contract

`Placement.position` is a signed global coordinate: X/Z studs and Y plates. One plate is 0.4 stud. The importer does not clamp X/Z to the application's resident window. `ImportPlan::batches(n)` exposes bounded slices for chunked host ingestion; the host owns chunk storage and streaming.

Maps use a local equirectangular projection centered on the requested bounds, with +X east and +Z south. `meters_per_stud` scales horizontal and vertical dimensions together. `origin[1]` is the optional ground plate; features begin one plate above it. Local areas are limited to 20 km per side, ±85° latitude and a single side of the dateline.

Meshes use a uniform fit of their longest source dimension. `max_height_plates` can reduce that fit uniformly, preserving aspect ratio. An explicit `units_per_stud` physical scale or an OSM height that exceeds the height budget returns an error; the importer never independently squashes Y. In the browser, the baseline is plate 18 and the available import height is 54 plates.

Triangle/voxel overlap uses separating-axis tests against the actual triangle. Exact-cover packing replaces occupied cells with the largest fitting rectangular catalog part of the same color. Packing never fills missing source cells or covers a courtyard merely to make larger bricks. Surface mode preserves open interiors; optional solid mode fills only regions unreachable from the exterior of the sampled mesh.

## API

```rust
use briccraft_import::{GeoBounds, OsmOptions, import_overpass_json};

let options = OsmOptions {
    bounds: GeoBounds::around(52.516275, 13.377704, 160.0)?,
    meters_per_stud: 4.0,
    origin: [48, 18, 48],
    max_height_plates: Some(54),
    ..OsmOptions::default()
};
let plan = import_overpass_json(&overpass_json, &options)?;
for batch in plan.batches(256) {
    // Persist/ingest global placements using the host's chunk storage.
}
```

The entry points are `import_overpass_json`, `import_obj`, `mesh::import_obj_with_mtl`, `import_glb` and `import_gltf_json`. Each format also exposes a `*_with_progress` variant in its module. Returning `false` from a progress callback cancels conversion and returns `ImportError::Cancelled`, without returning a partially successful plan.

`ImportPlan`, reports, options, projections and progress events implement Serde. The library reports `Parse`, `Project`, `Voxelize`, `Pack` and `Complete` stages. Progress is per stage; `completed/total` units change between stages.

Default budgets are 32 MiB input, 20,000 map elements/scene nodes, 250,000 vertices, 250,000 triangles, 1,000,000 occupied cells, 100,000 parts and 20,000,000 candidate-work operations per conversion phase. Embedded texture decoding has a combined byte budget tied to `max_input_bytes` and a maximum dimension of 8,192 pixels. Geometry buffers, accessor ranges, integer arithmetic, index values, finite coordinates and provider error messages are validated before their results are used.

## Browser worker

`briccraft-import-worker` is a small `wasm32-unknown-unknown` cdylib. It exports:

```text
processImport(kind, Uint8Array, optionsJson, progressCallback) -> ImportPlan JSON
defaultOptions(kind) -> options JSON
mapOptions(latitude, longitude, radiusMeters, metersPerStud) -> OsmOptions JSON
mapQuery(optionsJson) -> bounded Overpass query
```

Supported kinds are `osm`, `obj`, `glb` and `gltf`. The callback receives a JSON progress event; returning `false` cancels. The browser host can terminate the worker immediately to interrupt parsing, decoding or conversion. Fetch, endpoint selection, request pacing, caching, attribution display and user-selected file handling belong to the host.

For offline conversion through that same dispatcher:

```sh
cargo run -p briccraft-import-worker --example convert -- glb model.glb plan.json
cargo test -p briccraft-import -p briccraft-import-worker
cargo check -p briccraft-import-worker --target wasm32-unknown-unknown
```

## Data sources and attribution

OpenStreetMap data is available under the [Open Database License](https://www.openstreetmap.org/copyright). Every OSM plan includes `© OpenStreetMap contributors`, the source link and `ODbL-1.0` license metadata. Keep that attribution with saved/exported data and show it visibly when displaying an OSM world. See the [OSMF attribution guidelines](https://osmfoundation.org/wiki/Licence/Attribution_Guidelines), including the database and games/simulations sections.

The [Overpass operator's public-service guidance](https://dev.overpass-api.de/overpass-doc/en/preface/commons.html) discourages applications from relying on public instances as an unrestricted backend. This crate makes no network calls. The application uses explicit bounded requests, a configurable endpoint, cached imports and no automatic map requests while moving through the world. A production service with sustained traffic should provide its own suitable data service. Provider throttling or a partial result is surfaced to the user, not treated as a complete map.

Binary layout, scene transforms and accessor behavior follow the [Khronos glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html). Embedded texture transforms follow [KHR_texture_transform](https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_texture_transform/README.md).

The committed fixtures have their own data licenses and provenance in [tests/fixtures/README.md](tests/fixtures/README.md). The optional [Khronos ToyCar sample](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/Models/ToyCar/README.md) is CC0-1.0, credited to Guido Odendahl and Eric Chadwick. It is fetched only after an explicit sample-model action and is not bundled in this crate.

## Recorded validation

The actual 5,422,412-byte ToyCar GLB, fetched on 2026-11-11, converted with a 48-stud fit and 54-plate height budget to **2,056 parts, 8,565 occupied cells and 14 palette colors from 108,936 triangles**. The observed native debug conversion took 2.869 seconds on the development runner; this is one measurement, not a browser frame-rate guarantee. Embedded textures preserve the source paint colors at the chosen brick resolution. Transparent glass, fabric shading and clearcoat are represented as solid brick colors, and camera/light objects do not become bricks.

The Brandenburg fixture centered at **52.516275° N, 13.377704° E**, with a 160 m radius, 4 m per stud and origin `[48,18,48]`, converts to **169 parts, 7,329 occupied cells, 22 building features and 17 road features**, within `[8,18,8]..[88,35,88]`. Building features include mapped building parts. Native and shipped WASM conversion produce identical placements, reports and attribution for these options. Tests verify the explicit 26 m gate height, inferred-height evidence, signed global positions, courtyards, concave footprints, source-direction projection and nonoverlapping catalog-part coverage. Changing the projection center changes where the sampling grid falls on small source features, so coarse-grid counts can differ even at the same radius and scale.
