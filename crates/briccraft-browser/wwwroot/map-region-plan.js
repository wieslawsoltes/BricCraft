import { tilesForBounds, TILE_SIZE } from './elevation-plan.js';

export const VECTOR_TILE_URL = 'https://vector.openstreetmap.org/shortbread_v1/{z}/{x}/{y}.mvt';
export const VECTOR_ZOOM = 14;
const EARTH = 6378137;
const RADIANS = Math.PI / 180;

export function regionCoordinate(request, x, z) {
    return [request.latitude - (z - request.origin[2]) * request.meters_per_stud / EARTH / RADIANS,
        request.longitude + (x - request.origin[0]) * request.meters_per_stud
            / (EARTH * Math.cos(request.latitude * RADIANS)) / RADIANS];
}

function boundsFor(request, clip) {
    const [north, west] = regionCoordinate(request, clip[0], clip[1]);
    const [south, east] = regionCoordinate(request, clip[2], clip[3]);
    if (south < -85 || north > 85 || west < -180 || east > 180)
        throw new Error('This mapped region reaches the date line or supported Mercator limit');
    return { south, west, north, east };
}

export function planMapRegion(request) {
    const { latitude, longitude, meters_per_stud: scale, origin, clip } = request;
    if (!Number.isFinite(latitude) || Math.abs(latitude) > 85 || !Number.isFinite(longitude)
        || Math.abs(longitude) > 180 || !Number.isFinite(scale) || scale < 0.05 || scale > 1000
        || !Array.isArray(origin) || origin.length !== 3 || origin.some(v => !Number.isInteger(v) || Math.abs(v) > 1e9)
        || origin[1] < 0 || origin[1] > 45 || !Array.isArray(clip) || clip.length !== 4
        || clip.some(v => !Number.isInteger(v) || Math.abs(v) > 1e7 || v % 16 !== 0)
        || clip[2] <= clip[0] || clip[3] <= clip[1] || clip[2] - clip[0] > 256 || clip[3] - clip[1] > 256
        || Math.max(clip[2] - clip[0], clip[3] - clip[1]) * scale > 20000)
        throw new Error('The mapped region has invalid coordinates, scale, origin or chunk-aligned bounds');
    const template = request.vector_url || VECTOR_TILE_URL;
    if (typeof template !== 'string' || template.length > 2048
        || !['{z}', '{x}', '{y}'].every(key => template.includes(key)))
        throw new Error('The vector tile provider needs an HTTPS {z}/{x}/{y} URL template');
    const check = new URL(template.replaceAll('{z}', '14').replaceAll('{x}', '1').replaceAll('{y}', '1'));
    if (check.protocol !== 'https:' || check.username || check.password || check.hash)
        throw new Error('Use an HTTPS vector tile provider without credentials or a fragment');
    const bounds = boundsFor(request, clip);
    const n = 2 ** VECTOR_ZOOM;
    const tileX = longitude => (longitude + 180) / 360 * n;
    const tileY = latitude => (1 - Math.asinh(Math.tan(latitude * RADIANS)) / Math.PI) / 2 * n;
    // Source tiles cover the selected visible region only. No neighboring zoom
    // levels, route prediction, offline download, or speculative tile prefetch.
    const x0 = Math.floor(tileX(bounds.west)), x1 = Math.ceil(tileX(bounds.east)) - 1;
    const y0 = Math.floor(tileY(bounds.north)), y1 = Math.ceil(tileY(bounds.south)) - 1;
    if ((x1 - x0 + 1) * (y1 - y0 + 1) > 16)
        throw new Error('This region needs more than 16 vector tiles; choose a finer map scale');
    const tiles = [];
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
        tiles.push({ z: VECTOR_ZOOM, x, y, url: template.replaceAll('{z}', String(VECTOR_ZOOM))
            .replaceAll('{x}', String(x)).replaceAll('{y}', String(y)) });
    }
    return { bounds, tiles, elevation: request.terrain === false ? null : regionElevation(request, tiles) };
}

function regionElevation(request, vectorTiles) {
    // Building foundations use complete decoded footprints even when a region
    // cuts them in half. Cover the already requested vector tiles, including a
    // small geometry buffer, and keep DEM samples on one session-global lattice.
    const halo = 32;
    const cosine = Math.cos(request.latitude * RADIANS);
    const tileStuds = 2 * Math.PI * EARTH * cosine / (2 ** VECTOR_ZOOM * request.meters_per_stud);
    // Five source tiles cover the maximum normal square region footprint. The
    // stride depends only on the shared anchor/scale, never on the current area.
    const stride = 2 ** Math.max(0, Math.ceil(Math.log2((tileStuds * 5 + halo * 2) / 510)));
    const n = 2 ** VECTOR_ZOOM;
    const west = Math.min(...vectorTiles.map(t => t.x)) / n * 360 - 180;
    const east = (Math.max(...vectorTiles.map(t => t.x)) + 1) / n * 360 - 180;
    const latitude = y => Math.atan(Math.sinh(Math.PI * (1 - 2 * y / n))) / RADIANS;
    const north = latitude(Math.min(...vectorTiles.map(t => t.y)));
    const south = latitude(Math.max(...vectorTiles.map(t => t.y)) + 1);
    const projectX = lon => request.origin[0] + (lon - request.longitude) * RADIANS * EARTH * cosine / request.meters_per_stud;
    const projectZ = lat => request.origin[2] - (lat - request.latitude) * RADIANS * EARTH / request.meters_per_stud;
    const x0 = Math.floor((projectX(west) - halo - 0.5) / stride) * stride + 0.5;
    const z0 = Math.floor((projectZ(north) - halo - 0.5) / stride) * stride + 0.5;
    const x1 = Math.ceil((projectX(east) + halo - 0.5) / stride) * stride + 0.5;
    const z1 = Math.ceil((projectZ(south) + halo - 0.5) / stride) * stride + 0.5;
    const width = (x1 - x0) / stride + 1, height = (z1 - z0) / stride + 1;
    if (width > 513 || height > 513)
        throw new Error('This region exceeds the shared elevation-grid budget');
    const bounds = boundsFor(request, [x0, z0, x1, z1]);
    const zoom = Math.min(13, Math.max(0, Math.ceil(Math.log2(2 * Math.PI * EARTH
        * cosine / (TILE_SIZE * request.meters_per_stud * stride)))));
    const { tiles } = tilesForBounds(bounds, zoom);
    if (!tiles.length) throw new Error('This mapped region exceeds the elevation tile budget');
    return { bounds, width, height, tiles,
        resolution_m: Math.max(request.meters_per_stud * stride,
            2 * Math.PI * EARTH * cosine / (TILE_SIZE * 2 ** zoom)) };
}
