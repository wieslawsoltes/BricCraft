# Berlin Shortbread regression tiles

The two zoom-14 tiles contain the selected Berlin starting view around
Brandenburg Gate (52.516275, 13.377704) and its adjacent eastern region.
They were retrieved once on 2026-11-12 to validate the real provider schema.
`provenance.json` records their exact URLs, source modification times, sizes,
and SHA-256 hashes. They are test inputs, not bundled offline map downloads.

Map data © OpenStreetMap contributors, available under the
[Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/).
See [OpenStreetMap copyright](https://www.openstreetmap.org/copyright).

The data uses [Shortbread 1.1](https://shortbread-tiles.org/schema/1.1/),
served by `https://vector.openstreetmap.org/shortbread_v1/`. Building footprints
are real mapped data; the standard layer does not carry height tags, so BricCraft
reports its configurable default building heights as inferred.

CI serves these files locally and intercepts source requests. It must never
pan a live public map or download tile grids from public OSM services. Runtime
requests only the currently needed region, retains HTTP caching semantics,
shows attribution, and supports a replacement HTTPS tile-provider template.
Provider requirements: <https://operations.osmfoundation.org/policies/vector/>.
