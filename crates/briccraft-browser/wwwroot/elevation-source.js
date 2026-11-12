// Public DEM transport only. Raw PNG decoding and sampling run in the WASM worker.
import { fetchBounded } from './map-source.js';
import { planElevation } from './elevation-plan.js';

const CACHE = 'briccraft-elevation-terrarium-v1';
const MAX_TILE_BYTES = 1024 * 1024;
const MAX_TOTAL_BYTES = 12 * 1024 * 1024;
const CACHE_TILES = 48;
const CREDITS = 'Mapzen Terrain Tiles. ArcticDEM: DigitalGlobe imagery, NSF awards 1043681, 1559691, 1542736. '
    + 'Australia: © Commonwealth of Australia (Geoscience Australia) 2017. '
    + 'Austria: © offene Daten Österreichs – Digitales Geländemodell (DGM) Österreich. '
    + 'Canada: contains information licensed under the Open Government Licence – Canada. '
    + 'Europe: produced using Copernicus data and information funded by the European Union – EU-DEM layers. '
    + 'ETOPO1: U.S. National Oceanic and Atmospheric Administration. '
    + 'Mexico: INEGI, Continental relief, 2016. '
    + 'New Zealand: Copyright 2011 Crown copyright (c), Land Information New Zealand and the New Zealand Government, all rights reserved. '
    + 'Norway: © Kartverket. United Kingdom: © Environment Agency copyright and/or database right 2015, all rights reserved. '
    + 'United States 3DEP and global GMTED2010/SRTM: courtesy of the U.S. Geological Survey.';

export function terrainSource(resolution) {
    return {
        name: 'Mapzen Terrain Tiles', url: 'https://registry.opendata.aws/terrain-tiles/',
        attribution: CREDITS, license: 'Multiple open source-data licenses; see provider credits',
        license_url: 'https://github.com/tilezen/joerd/blob/master/docs/attribution.md',
        resolution_m: resolution,
    };
}

async function tileBytes(tile, signal, cache) {
    signal?.throwIfAborted();
    let bytes;
    try {
        const hit = await cache?.match(tile.url);
        if (hit) bytes = await hit.arrayBuffer();
    } catch { /* Disabled cache is not a data-source failure. */ }
    signal?.throwIfAborted();
    if (bytes && bytes.byteLength <= MAX_TILE_BYTES) return bytes;
    bytes = await fetchBounded(tile.url, { signal, credentials: 'omit' }, MAX_TILE_BYTES);
    signal?.throwIfAborted();
    try {
        await cache?.put(tile.url, new Response(bytes, { headers: { 'Content-Type': 'image/png' } }));
    } catch { /* A cache quota cannot prevent generation. */ }
    return bytes;
}

export async function fetchElevation(data, signal, progress = () => {}) {
    return fetchElevationPlan(planElevation(data), signal, progress);
}

// Live mapped regions supply a shared-grid plan, retaining the same bounded transport.
export async function fetchElevationPlan(plan, signal, progress = () => {}) {
    let cache;
    try { cache = await caches.open(CACHE); } catch { /* Optional disk cache. */ }
    const buffers = new Array(plan.tiles.length);
    let next = 0, completed = 0, totalBytes = 0;
    async function load() {
        for (;;) {
            const index = next++;
            if (index >= plan.tiles.length) return;
            const bytes = await tileBytes(plan.tiles[index], signal, cache);
            totalBytes += bytes.byteLength;
            if (totalBytes > MAX_TOTAL_BYTES) throw new Error('Elevation tiles exceed the 12 MB area budget');
            buffers[index] = bytes;
            progress(++completed, plan.tiles.length);
        }
    }
    await Promise.all(Array.from({ length: Math.min(3, plan.tiles.length) }, load));
    signal?.throwIfAborted();
    try {
        const keys = await cache?.keys();
        for (const key of (keys ?? []).slice(0, Math.max(0, (keys?.length ?? 0) - CACHE_TILES)))
            await cache.delete(key);
    } catch { /* Best-effort eviction. */ }
    const packed = new Uint8Array(totalBytes);
    let offset = 0;
    const tiles = plan.tiles.map((tile, index) => {
        const bytes = new Uint8Array(buffers[index]);
        packed.set(bytes, offset);
        const descriptor = { z: tile.z, x: tile.x, y: tile.y, offset, length: bytes.length };
        offset += bytes.length;
        return descriptor;
    });
    return {
        bytes: packed.buffer,
        metadata: {
            bounds: plan.bounds, width: plan.width, height: plan.height, tiles,
            source: terrainSource(plan.resolution_m), vertical_datum: 'Unknown',
        },
    };
}
