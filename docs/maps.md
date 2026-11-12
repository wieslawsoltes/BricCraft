# Places and live mapped worlds

## Choose where to build

Open **Places** from the desktop or mobile toolbar. Drag the native map, use its zoom
buttons or wheel/pinch, and tap/click to place the selection pin. The selected-area
outline follows the radius. Presets open Berlin, London, Kraków or San Francisco;
**Go to coordinates…** accepts another latitude/longitude. Address search is not included.

| Control | Behavior |
| --- | --- |
| **Build this area** | One complete finite area from Overpass and optional measured terrain. Radius choices are 160 m, 500 m and 1.5 km. |
| **Free roam from here** | A new Creative world starting in Walk mode, with further mapped regions generated as the resident view moves. |
| **World scale** | 2, 4 or 8 metres per stud; smaller values retain more horizontal detail. |
| **Use real terrain elevation** | Adds a measured DEM. Turning it off keeps geographic features on flat ground. |
| **Pause streaming / Resume / retry** | Stops or resumes new source work while retaining loaded terrain and constructions. |
| **3D models & advanced imports** | Existing Overpass endpoint/local-file controls, bundled samples and model conversion. |

The radius applies to **Build this area**. Free roam uses 128 × 128-stud source regions
and continues beyond that selection. Walk with WASD or the touch pad, Space to jump,
and Shift to run. The normal construction tools, designs and world saves remain available.
Cars and aircraft can cross mapped regions. Boats currently need the island sea;
guided trains still use the island railway.

## Sources and fidelity

The picker displays OpenStreetMap raster tiles. Explicit area generation uses Overpass
OSM geometry; free roam reads zoom-14 **Shortbread MVT** tiles from the separate OSMF
vector service. Vector tiles contain mapped buildings, surface roads, water and land
cover, with provider generalization and clipping. They contain fewer tags than a full
OSM extract. Missing heights and widths are recorded as inferences; vector identifiers
are not presented as OSM way/relation IDs. No facade scans or photogrammetry are inferred.
See the [Shortbread schema](https://shortbread-tiles.org/schema/1.1/).

Elevation comes independently from [Mapzen Terrain Tiles](https://registry.opendata.aws/terrain-tiles/).
Its geographic resolution and survey age vary. The application retains source credits
and an unknown/mixed vertical datum when the provider does not establish one. The
[terrain attribution list](https://github.com/tilezen/joerd/blob/master/docs/attribution.md)
describes the underlying datasets. OSM attribution remains visible on the map and
travels with generated worlds: [© OpenStreetMap contributors](https://www.openstreetmap.org/copyright).

## What happens while walking

The renderer retains a 96 × 72 × 96-cell window and shifts it in 16-stud steps. A missing
source region is fetched and converted in a dedicated Web Worker, then validated before
publication. Requests follow current/requested resident windows, with one region producer
at a time. Source generation does not reset the session, move the camera, spend Studs or
add an undo action. Player edits retain stable global identities.

The map reference and horizontal scale stay fixed. With terrain enabled, the first area
also establishes the elevation reference and metres per plate with headroom. Neighboring
regions retain that scale and a shared DEM sampling lattice. Terrain or buildings outside
the supported 72-plate range produce an error; they are not clipped or independently refit.
Start a separate world at a suitable scale when the fixed range cannot cover another area.

The source store retains at most **32 regions / 250,000 source parts**, separately from
the render cache. Distant pristine regions may be evicted; active/pending regions and
those referenced by edits or undo history are protected. A protected-cache limit is
reported without discarding constructions. Saves include retained geography and edits.
New areas still require an available source, and the saved game is not a tile archive.

If a source is slow or unavailable, the last complete view remains visible and movement
waits inside its boundary. Turn, build in the loaded area, or open **Places → Resume / retry**.
Changing worlds or cancelling rejects late worker responses. Public services have no
availability guarantee; neither networking nor headless validation establishes hardware FPS.

## Configure tile providers

Defaults are:

| Purpose | HTTPS template |
| --- | --- |
| Picker raster | `https://tile.openstreetmap.org/{z}/{x}/{y}.png` |
| Free-roam vector | `https://vector.openstreetmap.org/shortbread_v1/{z}/{x}/{y}.mvt` |

Deployment owners can change either template in this site's local storage without a
rebuild. The object contains optional `raster` and `vector` string keys:

```js
localStorage.setItem('briccraft.map-providers.v1', JSON.stringify({
  raster: 'https://maps.example.org/raster/{z}/{x}/{y}.png',
  vector: 'https://maps.example.org/shortbread_v1/{z}/{x}/{y}.mvt'
}));
location.reload();
```

The raster service must return 256 × 256 PNGs; vector data must be compatible with the
supported Shortbread layers at zoom 14. Both must allow this application to fetch them
through browser CORS. URLs require HTTPS and `{z}`, `{x}`, `{y}`, without embedded
username/password credentials. No API keys are shipped or provided through this setting.
Keep the OSM credits and add any notices required by an alternative provider when adapting
the deployment. Remove the local-storage entry to restore defaults. This setting changes
tile transport; it does not replace the advanced Overpass endpoint or DEM source.

The picker fetches only visible tiles after a short viewport debounce. Free roam requests
only source tiles needed by current/requested resident views. Neither path downloads
routes, neighboring zoom stacks or offline map archives. Browser requests retain normal
HTTP caching and Referer behavior; bounded memory caches reuse tiles. Vector reuse honors
available expiry headers, with a seven-day fallback. HTTP 429/503 responses delay further
region requests before a user retry. These behaviors follow the primary
[raster tile policy](https://operations.osmfoundation.org/policies/tiles/) and
[vector tile policy](https://operations.osmfoundation.org/policies/vector/).
