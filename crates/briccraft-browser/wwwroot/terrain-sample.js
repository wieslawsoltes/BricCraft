import { fetchBounded } from './map-source.js';

/** The included real area uses the same decoder and conversion pipeline as live data. */
export async function loadTerrainSample(signal) {
    const metadataUrl = new URL('./assets/terrain/telegraph-hill.json', import.meta.url);
    const manifestBytes = await fetchBounded(metadataUrl, { signal }, 64 * 1024);
    const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));
    const [bytes, elevationBytes] = await Promise.all([
        fetchBounded(new URL('./assets/maps/telegraph-hill.json', import.meta.url), { signal }),
        fetchBounded(new URL('./assets/terrain/telegraph-hill.dem', import.meta.url), { signal }, 12 * 1024 * 1024),
    ]);
    signal?.throwIfAborted();
    return {
        bytes,
        elevation: { metadata: manifest.metadata, bytes: elevationBytes },
        options: {
            name: manifest.name, latitude: manifest.latitude, longitude: manifest.longitude,
            radius: manifest.radius, meters_per_stud: manifest.meters_per_stud,
            terrain: true, fit_height: true, vertical_exaggeration: 1,
        },
    };
}
