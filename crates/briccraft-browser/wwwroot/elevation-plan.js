// Geometry only: transport and decoding live in separate modules.
import { mapBounds } from './map-source.js';

const EARTH_RADIUS = 6378137;
const MAX_LATITUDE = 85;
export const TILE_SIZE = 256;
export const MAX_ELEVATION_TILES = 16;
export const TERRAIN_TILE_ROOT = 'https://elevation-tiles-prod.s3.amazonaws.com/terrarium';

export function mercatorPixel(latitude, longitude, zoom) {
    const n = TILE_SIZE * 2 ** zoom;
    const latitudeRadians = latitude * Math.PI / 180;
    return [(longitude + 180) / 360 * n,
        (1 - Math.asinh(Math.tan(latitudeRadians)) / Math.PI) / 2 * n];
}

export function tilesForBounds(bounds, zoom) {
    const [west, north] = mercatorPixel(bounds.north, bounds.west, zoom);
    const [east, south] = mercatorPixel(bounds.south, bounds.east, zoom);
    // Values live at pixel centers. Include both neighbors for interpolation,
    // including the adjacent tile when the selected area crosses a tile seam.
    const firstX = Math.floor(Math.floor(west - 0.5) / TILE_SIZE);
    const lastX = Math.floor((Math.floor(east - 0.5) + 1) / TILE_SIZE);
    const firstY = Math.floor(Math.floor(north - 0.5) / TILE_SIZE);
    const lastY = Math.floor((Math.floor(south - 0.5) + 1) / TILE_SIZE);
    const n = 2 ** zoom;
    if (firstX < 0 || firstY < 0 || lastX >= n || lastY >= n)
        throw new Error('Choose an area away from the date line and Mercator poles');
    const count = (lastX - firstX + 1) * (lastY - firstY + 1);
    if (count > MAX_ELEVATION_TILES) return { count, tiles: [] };
    const tiles = [];
    for (let y = firstY; y <= lastY; y++) {
        for (let x = firstX; x <= lastX; x++) {
            tiles.push({ z: zoom, x, y, url: `${TERRAIN_TILE_ROOT}/${zoom}/${x}/${y}.png` });
        }
    }
    return { count, tiles };
}

export function planElevation(data) {
    const { latitude, longitude, radius, meters_per_stud: scale } = data;
    const [south, west, north, east] = mapBounds(latitude, longitude, radius);
    if (!Number.isFinite(scale) || scale < 0.05 || scale > 1000)
        throw new Error('Meters per stud must be between 0.05 and 1000');
    // Rasterized cells may straddle the map boundary. The DEM includes a small
    // physical border so every cell center can be sampled without clamping.
    const border = Math.max(scale * 2, 5);
    const dy = border / EARTH_RADIUS * 180 / Math.PI;
    const dx = dy / Math.cos(latitude * Math.PI / 180);
    const bounds = { south: south - dy, west: west - dx, north: north + dy, east: east + dx };
    if (bounds.south < -MAX_LATITUDE || bounds.north > MAX_LATITUDE
        || bounds.west < -180 || bounds.east > 180)
        throw new Error('The terrain border crosses the supported map extent');
    let zoom = Math.min(15, Math.max(0,
        Math.ceil(Math.log2(2 * Math.PI * EARTH_RADIUS * Math.cos(latitude * Math.PI / 180)
            / (TILE_SIZE * scale)))));
    let coverage = tilesForBounds(bounds, zoom);
    while (coverage.count > MAX_ELEVATION_TILES && zoom > 0) {
        coverage = tilesForBounds(bounds, --zoom);
    }
    if (!coverage.tiles.length) throw new Error('This area exceeds the elevation tile budget');
    const samples = Math.max(3, Math.min(513, Math.ceil(2 * (radius + border) / scale) + 1));
    return {
        bounds, width: samples, height: samples, tiles: coverage.tiles,
        resolution_m: 2 * Math.PI * EARTH_RADIUS * Math.cos(latitude * Math.PI / 180)
            / (TILE_SIZE * 2 ** zoom),
    };
}
