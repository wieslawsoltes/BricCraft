# Telegraph Hill offline area

The sample covers a **450 m radius** around **37.8024° N, 122.4058° W** in San
Francisco. Its map data comes from the official OpenStreetMap API, downloaded on
2026-11-11, under ODbL-1.0. Source URLs are in `telegraph-hill.json.license`.

The original area response contained 22,080 elements. The derived fixture retains
**all 2,579 supported root ways and 16 supported root relations**, plus their
complete referenced geometry. Unrelated point features, barriers, route relations
and unsupported ways were removed; no supported feature was discarded to meet a
budget. Six inner courtyard ways missing from the map response were recovered
from the official full-way API, preserving the holes in three building relations.

The final graph contains **20,209 elements: 17,579 nodes, 2,614 ways and 16
relations**, occupying 3,898,495 bytes. The sample explicitly uses
`max_features: 24000`; the normal import default remains 20,000. Coordinates,
tags, source identities, node order and relation membership remain unchanged.
No polygon was repaired or clipped during preprocessing.

`telegraph-hill.provenance.json` records the hashes of all seven source responses,
the final fixture hash, every original supported root ID and the element counts.
The source graph can be reconstructed from those recorded responses using:

```sh
python3 scripts/filter-osm-fixture.py \
  /tmp/briccraft-telegraph-hill.json \
  crates/briccraft-browser/wwwroot/assets/maps/telegraph-hill.json \
  /tmp/briccraft-telegraph-way-*.json \
  --max-elements 24000 \
  --provenance crates/briccraft-browser/wwwroot/assets/maps/telegraph-hill.provenance.json
```

The filter refuses conflicting geometry, incomplete references or an oversized
complete result. It performs no fetching, coordinate rounding or tag rewriting.

## Elevation and conversion

The companion `../terrain/telegraph-hill.json` manifest and
`../terrain/telegraph-hill.dem` bundle contain four raw Mapzen Terrarium PNG tiles
from the public AWS Terrain Tiles archive. All four provider source headers name
`ned_topobathy/ned19_n38x00_w122x50_ca_sanfrancisco_topobathy_2010.tif` and have a
2017-11-11 tile modification date. The manifest preserves source URLs, ETags and
provider credits. Vertical datum remains **Unknown**; no undocumented datum
conversion is applied.

At **8 m/stud**, origin `[48,18,48]`, a 54-plate height budget, `fit_height: true`
and vertical exaggeration 1, native conversion produces:

| Measurement | Result |
| --- | ---: |
| Total editable parts | 12,820 |
| Total occupied cells | 184,645 |
| Building features / roads / land-cover features / water features | 1,351 / 1,069 / 40 / 1 |
| Complete terrain columns | 12,996 |
| Terrain parts / solid terrain cells | 9,406 / 166,474 |
| Sampled source elevation range | −13.288241 to 91.942585 m |
| Effective horizontal scale | 8 m/stud, unchanged |
| Effective vertical scale | 3.25443896 m/plate |
| Vertical compression ratio | 0.98327240, about 1.67% compression |
| Occupied bounds, exclusive maximum | `[-9,18,-9]..[105,71,105]` |

The report marks 415 building heights as inferred, 1,275 raised foundation columns
and one inferred water level. The reference JSON contains exact expected metrics
and conversion settings. One native debug run completed conversion and coverage
verification in 0.58 s; this measurement is not a browser frame-rate guarantee.

The dedicated worker integration test decodes the actual PNG bundle, checks every
original supported ID and every retained reference, converts through the same
dispatcher as the browser, verifies complete ground coverage and nonoverlapping
catalog parts, and checks attribution, datum and vertical bounds:

```sh
cargo test -p briccraft-import-worker --test telegraph_hill
```
