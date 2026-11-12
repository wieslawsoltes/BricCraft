// Geographic tile decoding and brick conversion never run on the UI thread.
import init, { processMapRegion } from './import-engine/briccraft_import_worker.js';
let engine;
self.onmessage = async ({ data }) => {
    try {
        engine ||= init(); await engine;
        let last = 0;
        const source = processMapRegion(JSON.stringify(data.request), new Uint8Array(data.vectors.bytes),
            data.elevation ? JSON.stringify(data.elevation.metadata) : '',
            new Uint8Array(data.elevation?.bytes ?? 0), update => {
                const now = performance.now();
                if (now - last >= 80) {
                    self.postMessage({ type: 'progress', progress: JSON.parse(update) }); last = now;
                }
                return true;
            });
        const plan = JSON.parse(source);
        self.postMessage({ type: 'complete', source, report: plan.report, name: plan.report.source_name });
    } catch (error) {
        self.postMessage({ type: 'error', message: String(error?.message || error) });
    }
};
