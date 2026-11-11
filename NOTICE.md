# Notices

BricCraft application code and its procedural geometry, world generation, interface, shaders and synthesized sound are original work distributed under the MIT license.

FerroUI is included as a pinned git submodule. Its licenses and the notices of the individual framework crates are retained in `FerroUI/LICENSE` and their respective `NOTICE.md` files. The browser uses FerroUI's Skia backend, Inter fonts and browser platform. Rust dependencies retain their own licenses; the exact versions are recorded in `Cargo.lock`.

The construction dimensions use a stud as the horizontal unit and a plate as 0.4 of that unit. The catalogue is procedural and does not redistribute a third-party CAD part library or game assets.

## Geographic data

`crates/briccraft-import/tests/fixtures/brandenburg-osm.json` and its browser asset copy
`crates/briccraft-browser/wwwroot/assets/maps/brandenburg.json` contain an OpenStreetMap
extract around Brandenburg Gate. © OpenStreetMap contributors. Data is provided under
the [Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
Attribution and source provenance are retained in the fixture notices, import reports
and exported mapped worlds. See [OpenStreetMap copyright](https://www.openstreetmap.org/copyright).
Application code remains MIT; that license does not replace the source data license.

## Model fixtures and samples

The small BoxVertexColors GLB regression fixture comes from the Khronos glTF sample
assets and is CC0; its exact source and notice accompany the fixture. The optional
ToyCar example is fetched only when selected. Its original model is by Guido Odendahl
and its extensions/scene composition by Eric Chadwick, both released under CC0-1.0
(2020), as recorded in the [official ToyCar notice](https://github.com/KhronosGroup/glTF-Sample-Assets/blob/main/Models/ToyCar/README.md).
ToyCar geometry is not bundled into the application. Imports of user-selected models
retain supplied source notices; their original licenses continue to apply.
