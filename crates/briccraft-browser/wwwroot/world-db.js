// Transactional storage, independent of UI and WASM. Version1's island is retained.
const NAME = 'briccraft-worlds', WORLDS = 'worlds', META = 'metadata';
export const MAX_WORLD_BYTES = 64 * 1024 * 1024;
export async function database() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(NAME, 2);
        request.onupgradeneeded = () => {
            const db = request.result;
            if (!db.objectStoreNames.contains(WORLDS)) db.createObjectStore(WORLDS);
            if (!db.objectStoreNames.contains(META)) db.createObjectStore(META);
            const store = request.transaction.objectStore(WORLDS);
            const legacy = store.get('current');
            legacy.onsuccess = () => {
                if (!legacy.result?.source) return;
                const record = describe(legacy.result.source, 'legacy-island', 'My original island');
                record.updated = legacy.result.updated || Date.now();
                store.put(record, record.id);
                request.transaction.objectStore(META).put(manifest(record),`world:${record.id}`);
                // Keep the original slot as a migration backup, outside the library list.
                request.transaction.objectStore(META).put(record.id, 'active');
            };
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}
export function describe(source, id, name) {
    if (source.length > MAX_WORLD_BYTES) throw new Error('World exceeds the 64 MB save limit');
    const info=JSON.parse(source).world_info || {};
    const record={
        id:id || info.id || crypto.randomUUID(),
        name:(name || info.name || 'Untitled brick world').trim().slice(0,80) || 'Untitled brick world',
        preset:info.preset || 'Classic island', source, updated:Date.now(), archived:false,
    };
    // Never stringify parsed world data: global i64 coordinates and u64 seeds
    // can exceed JavaScript's exact integer range. Identity is applied in Rust.
    return record;
}
function manifest({id,name,preset,updated,archived}) { return {id,name,preset,updated,archived}; }
async function transaction(mode, stores, run) {
    const db = await database();
    try {
        return await new Promise((resolve, reject) => {
            const tx = db.transaction(stores, mode);
            let result;
            run(tx, value => { result=value; });
            tx.oncomplete = () => resolve(result);
            tx.onerror = () => reject(tx.error || new Error('Storage transaction failed'));
            tx.onabort = () => reject(tx.error || new Error('Storage transaction was interrupted'));
        });
    } finally { db.close(); }
}
export async function saveRecord(record, activate=true) {
    return transaction('readwrite',[WORLDS,META],tx => {
        tx.objectStore(WORLDS).put(record,record.id);
        tx.objectStore(META).put(manifest(record),`world:${record.id}`);
        if (activate) tx.objectStore(META).put(record.id,'active');
    });
}
export async function readRecord(id) {
    return transaction('readonly',[WORLDS],(tx,result) => {
        tx.objectStore(WORLDS).get(id).onsuccess = e => result(e.target.result);
    });
}
export async function activeId() {
    return transaction('readonly',[META],(tx,result) => {
        tx.objectStore(META).get('active').onsuccess = e => result(e.target.result);
    });
}
export async function allRecords() {
    return transaction('readonly',[META],(tx,result) => {
        tx.objectStore(META).getAll().onsuccess = e => result(
            e.target.result.filter(r => r.id).sort((a,b) => b.updated-a.updated));
    });
}
export async function archiveRecord(id, archived) {
    return transaction('readwrite',[WORLDS,META],tx => {
        const store=tx.objectStore(WORLDS);
        store.get(id).onsuccess = e => {
            if (e.target.result) {
                const record={...e.target.result,archived};
                store.put(record,id); tx.objectStore(META).put(manifest(record),`world:${id}`);
            }
        };
    });
}
