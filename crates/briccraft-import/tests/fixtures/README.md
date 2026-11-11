# Test data provenance

These data files retain the licenses below. The repository's source-code license does not replace their data licenses.

## `brandenburg-osm.json`

- **Attribution:** © OpenStreetMap contributors.
- **License:** [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/); [OSM copyright and license information](https://www.openstreetmap.org/copyright).
- **Retrieved:** 2026-11-11 UTC, through one bounded read-only request to the official OSM API.
- **Source:** <https://api.openstreetmap.org/api/0.6/map.json?bbox=13.3773,52.5161,13.3781,52.5165>.
- **Transformation:** Retained supported building/building-part/highway/waterway ways and every node referenced by those ways. The 72 ways and 908 nodes retain their original coordinates and tags. No geometry or missing height was synthesized in the fixture. Source copyright/license fields are preserved.

The small queried rectangle intersects complete ways, so some included geometry extends beyond it. Import options apply the requested display bounds explicitly. The fixture contains the Brandenburger Tor way `518071791`, whose mapped `height=26` is used to verify explicit-height evidence. The JSON is a reproducible snapshot; it is not a live assertion about the current map.

## `BoxVertexColors.glb`

- **Author:** Marco Hutter.
- **Copyright:** 2023, Public.
- **License:** [Creative Commons Zero 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
- **Retrieved:** 2026-11-11 UTC; unchanged binary, 1,924 bytes.
- **Source and license:** [Khronos sample README](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/Models/BoxVertexColors/README.md).
- **Binary:** <https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models/BoxVertexColors/glTF-Binary/BoxVertexColors.glb>.

This actual GLB verifies indexed geometry, shared/interleaved buffer views, vertex-color interpolation, uniform scaling, surface holes and optional interior filling. It is a codec fixture, not a representation of a real building.
