// Test geographic coverage, real seam halos, bounded transport and cancellation.
import assert from 'node:assert/strict';
import { setImmediate as settle } from 'node:timers/promises';
import { mapBounds } from '../crates/briccraft-browser/wwwroot/map-source.js';
import { planElevation, tilesForBounds, mercatorPixel } from '../crates/briccraft-browser/wwwroot/elevation-plan.js';
import { fetchElevation } from '../crates/briccraft-browser/wwwroot/elevation-source.js';

const data = { latitude: 37.8024, longitude: -122.4058, radius: 450, meters_per_stud: 8 };
const plan = planElevation(data);
assert.equal(plan.tiles.length, 4);
assert.equal(plan.width, 118);
assert.ok(plan.bounds.south < mapBounds(data.latitude, data.longitude, data.radius)[0]);
const sphericalLatitude = 450 / 6378137 * 180 / Math.PI;
assert.ok(Math.abs(mapBounds(0, 0, 450)[2] - sphericalLatitude) < 1e-12);

for (const latitude of [-80, -30, 0, 52, 80]) {
    for (const radius of [20, 160, 500, 1500]) {
        const coverage = planElevation({ latitude, longitude: 10, radius, meters_per_stud: 0.05 });
        assert.ok(coverage.tiles.length >= 1 && coverage.tiles.length <= 16);
        assert.ok(coverage.width <= 513 && coverage.height <= 513);
        assert.ok(coverage.resolution_m > 0);
    }
}
// A box centered exactly on a tile edge needs samples from both adjacent tiles.
const seam = tilesForBounds({ south: -0.01, west: -0.01, north: 0.01, east: 0.01 }, 3);
assert.deepEqual(new Set(seam.tiles.map(tile => tile.x)), new Set([3, 4]));
assert.deepEqual(new Set(seam.tiles.map(tile => tile.y)), new Set([3, 4]));
assert.deepEqual(mercatorPixel(0, 0, 0), [128, 128]);
assert.throws(() => planElevation({ ...data, radius: 2000 }));
assert.throws(() => planElevation({ ...data, meters_per_stud: NaN }));

const originalFetch = globalThis.fetch;
const originalCaches = globalThis.caches;
let active = 0, maximumActive = 0, fetches = 0;
globalThis.caches = { open: async () => { throw new Error('Cache disabled'); } };
try {
    globalThis.fetch = async () => {
        fetches++; active++; maximumActive = Math.max(maximumActive, active);
        await settle(); active--;
        return new Response(new Uint8Array([137, 80, 78, 71]), { status: 200 });
    };
    const updates = [];
    const result = await fetchElevation(data, new AbortController().signal, (done, total) => updates.push([done, total]));
    assert.equal(fetches, plan.tiles.length);
    assert.ok(maximumActive <= 3, 'Concurrent tile requests remain bounded');
    assert.equal(result.bytes.byteLength, plan.tiles.length * 4);
    assert.equal(result.metadata.vertical_datum, 'Unknown');
    assert.ok(result.metadata.source.attribution.includes('Copernicus'));
    assert.deepEqual(updates.at(-1), [plan.tiles.length, plan.tiles.length]);
    assert.deepEqual(result.metadata.tiles.map(t => t.offset), [0, 4, 8, 12]);

    const controller = new AbortController(); controller.abort();
    fetches = 0;
    await assert.rejects(fetchElevation(data, controller.signal), /abort/i);
    assert.equal(fetches, 0, 'Canceled requests must not contact the elevation service');

    globalThis.fetch = async () => new Response(new Uint8Array([1]), {
        headers: { 'Content-Length': String(2 * 1024 * 1024) },
    });
    await assert.rejects(fetchElevation(data, new AbortController().signal), /1 MB/);
    console.log(JSON.stringify({ test: 'Elevation source coverage and budgets',
        geographicScenarios: 20, seamInterpolation: true, maxConcurrentRequests: maximumActive,
        cancellation: true, byteBudget: true }));
} finally {
    globalThis.fetch = originalFetch;
    if (originalCaches === undefined) delete globalThis.caches;
    else globalThis.caches = originalCaches;
}
