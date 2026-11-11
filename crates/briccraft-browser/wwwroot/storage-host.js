// IndexedDB is transactional and accommodates worlds larger than localStorage quotas.
const DB = 'briccraft-worlds';
const STORE = 'worlds';
function database() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open(DB, 1);
        request.onupgradeneeded = () => request.result.createObjectStore(STORE);
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}
export function installStorage(runtime) {
    const report = text => runtime.briccraftMessage(text);
    globalThis.briccraftSave = async source => {
        let db;
        try {
            db = await database();
            await new Promise((resolve, reject) => {
                const tx = db.transaction(STORE, 'readwrite');
                tx.objectStore(STORE).put({ source, updated: Date.now() }, 'current');
                tx.oncomplete = resolve;
                tx.onerror = () => reject(tx.error);
                tx.onabort = () => reject(tx.error || new Error('Save was interrupted'));
            });
            report('World saved in this browser. Export a file to keep another copy.');
        } catch (error) { report(`Save failed: ${error.message}. Export your world to a file.`); }
        finally { db?.close(); }
    };
    globalThis.briccraftLoad = async () => {
        let db;
        try {
            db = await database();
            const saved = await new Promise((resolve, reject) => {
                const request = db.transaction(STORE).objectStore(STORE).get('current');
                request.onsuccess = () => resolve(request.result);
                request.onerror = () => reject(request.error);
            });
            if (saved) runtime.briccraftImportWorld(saved.source);
            else report('No saved world yet. Use Save to keep this construction.');
        } catch (error) { report(`Could not load your world: ${error.message}`); }
        finally { db?.close(); }
    };
    globalThis.briccraftExport = source => {
        const url = URL.createObjectURL(new Blob([source], { type: 'application/json' }));
        const link = document.createElement('a');
        link.href = url; link.download = 'my-world.briccraft'; link.click();
        setTimeout(() => URL.revokeObjectURL(url), 1000);
        queueMicrotask(() => report('World exported as my-world.briccraft.'));
    };
    const input = document.getElementById('world-file');
    globalThis.briccraftImport = () => { input.value = ''; input.click(); };
    input.addEventListener('change', async () => {
        const file = input.files?.[0]; if (!file) return;
        if (file.size > 16 * 1024 * 1024) return report('The world file exceeds the 16 MB import limit.');
        try { runtime.briccraftImportWorld(await file.text()); }
        catch (error) { report(`Could not import this world: ${error.message}`); }
    });
}
