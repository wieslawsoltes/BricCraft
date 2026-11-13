import { abortError, throwIfAborted } from './source-errors.js';

// One warm WASM worker serves sequential regions. Active cancellation must
// terminate it: a synchronous Rust conversion cannot consume an abort message.
// A successful conversion keeps the initialized engine for the next region;
// an idle lease releases its memory when exploration stops.
export function createMapRegionConverter({ workerFactory, idleTimeoutMs = 60_000 } = {}) {
    let worker, pending, idleTimer, nextId = 0;

    function retire() {
        clearTimeout(idleTimer);
        idleTimer = undefined;
        if (!worker) return;
        const retired = worker;
        worker = undefined;
        retired.onmessage = retired.onerror = retired.onmessageerror = null;
        retired.terminate();
    }

    function finish(error, value) {
        const job = pending;
        if (!job) return;
        pending = undefined;
        job.signal?.removeEventListener('abort', job.abort);
        if (error || idleTimeoutMs <= 0) retire();
        else {
            idleTimer = setTimeout(() => { if (!pending) retire(); }, idleTimeoutMs);
            idleTimer.unref?.();
        }
        if (error) job.reject(error); else job.resolve(value);
    }

    function ensureWorker() {
        if (worker) return worker;
        const created = workerFactory();
        worker = created;
        created.onerror = event => {
            if (worker !== created) return;
            if (pending) finish(new Error(event.message || 'Mapped terrain conversion failed'));
            else retire();
        };
        created.onmessageerror = () => {
            if (worker !== created) return;
            if (pending) finish(new Error('The map conversion worker returned an unreadable result'));
            else retire();
        };
        created.onmessage = ({ data }) => {
            const job = pending;
            if (worker !== created || !job || data?.id !== job.id) return;
            try {
                throwIfAborted(job.signal);
                if (data.type === 'progress') job.progress(data.progress);
                else if (data.type === 'error') finish(new Error(data.message));
                else if (data.type === 'complete') {
                    if (typeof data.source !== 'string' || !data.report || typeof data.report !== 'object')
                        throw new Error('The map conversion worker returned an invalid plan');
                    finish(null, { source: data.source, report: data.report, name: data.name });
                }
            } catch (error) { finish(error); }
        };
        return created;
    }

    return function convert(request, vectors, elevation, signal, progress) {
        throwIfAborted(signal);
        if (pending) return Promise.reject(new Error('A mapped region is already being converted'));
        clearTimeout(idleTimer);
        idleTimer = undefined;
        return new Promise((resolve, reject) => {
            const job = { id: ++nextId, signal, progress, resolve, reject };
            job.abort = () => { if (pending === job) finish(signal?.reason ?? abortError()); };
            pending = job;
            signal?.addEventListener('abort', job.abort, { once: true });
            if (signal?.aborted) { job.abort(); return; }
            try {
                const active = ensureWorker();
                const transfers = [vectors.bytes];
                if (elevation?.bytes) transfers.push(elevation.bytes);
                active.postMessage({ id: job.id, request, vectors, elevation }, transfers);
            } catch (error) { finish(error); }
        });
    };
}
