// Source failures retain retry information without silently changing providers.
export class SourceHttpError extends Error {
    constructor(status, retryAfter, now = Date.now()) {
        const milliseconds = /^\d+(?:\.\d+)?$/.test(retryAfter ?? '')
            ? Number(retryAfter) * 1000 : Date.parse(retryAfter ?? '') - now;
        super(`Map source returned HTTP ${status}${retryAfter ? `; retry after ${retryAfter}` : ''}`);
        this.name = 'SourceHttpError';
        this.status = status;
        this.retryAfterMs = Number.isFinite(milliseconds) ? Math.max(0, milliseconds) : 0;
    }
}

export function abortError() {
    return new DOMException('Map generation cancelled', 'AbortError');
}

export function throwIfAborted(signal) {
    if (signal?.aborted) throw signal.reason ?? abortError();
}
