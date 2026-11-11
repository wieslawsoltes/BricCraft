import { describe, saveRecord, readRecord, allRecords, activeId, archiveRecord, MAX_WORLD_BYTES } from './world-db.js';

export function installStorage(runtime) {
    const report = text => runtime.briccraftMessage(text);
    let current = '';
    let operations = Promise.resolve();
    // Deferred callbacks never reenter Rust while an action holds the session borrow.
    const enqueue = (action,propagate=false) => {
        const task=operations.then(action);
        operations=task.catch(error => report(`World storage: ${error.message}`));
        return propagate ? task : operations;
    };
    const list = async () => {
        const worlds = (await allRecords()).map(world => ({
            id:world.id, name:world.name, detail:String(world.preset), archived:world.archived,
            updated:new Date(world.updated).toLocaleString(),
        }));
        runtime.briccraftWorldList(JSON.stringify(worlds));
    };
    const save = async (source,id,name) => {
        const record=describe(source,id,name);
        const live=JSON.parse(runtime.briccraftState());
        const activate=!!id || live.worldId===record.id;
        await saveRecord(record,activate);
        if(activate) { current=record.id; runtime.briccraftWorldIdentity(record.id,record.name); }
        await list(); return record;
    };
    const open = async id => {
        const record=await readRecord(id);
        if (!record) throw new Error('The selected world is no longer available');
        if (!runtime.briccraftImportWorld(record.source)) return false;
        current=id; runtime.briccraftWorldIdentity(id,record.name);
        await saveRecord({...record,archived:false}); await list(); return true;
    };
    globalThis.briccraftSave = source => enqueue(async () => {
        await save(source); report('World saved. Your collection is available in Worlds.');
    });
    globalThis.briccraftLoad = () => enqueue(async () => {
        try {
            const id=current || await activeId();
            if (id) await open(id); else report('No saved world yet. Use Save to keep this construction.');
        } finally { runtime.briccraftWorldReady(); }
    });
    globalThis.briccraftWorldHost = (action,payload) => enqueue(async () => {
      try {
        const data=JSON.parse(payload || '{}');
        if (action==='list') return list();
        if (action==='archive') { await archiveRecord(data.id,data.archived); return list(); }
        // A failed autosave prevents switching worlds.
        if (data.snapshot) await save(data.snapshot);
        if (action==='open') return await open(data.id);
        if (action==='copy') {
            await save(data.snapshot,crypto.randomUUID(),data.name);
            report('Saved a separate copy of this world.');
        }
        if (action==='create') {
            const id=crypto.randomUUID();
            if (runtime.briccraftCreateWorld(data.preset,data.name,String(data.seed),id)) {
                current=id; await save(runtime.briccraftSerializeWorld(),id,data.name);
                report(`Welcome to ${data.name || 'your new world'}.`);
            }
        }
      } finally { if(['open','copy','create'].includes(action)) runtime.briccraftWorldReady(); }
    });
    globalThis.briccraftExport = source => {
        const record=describe(source,current);
        download(source,`${record.name.replace(/[^a-z0-9 _-]/gi,'').trim() || 'my-world'}.briccraft`);
        queueMicrotask(() => report('World exported with source attribution and streamed edits.'));
    };
    const input=document.getElementById('world-file');
    globalThis.briccraftImport = () => { input.value=''; input.click(); };
    input.addEventListener('change',() => {
        const file=input.files?.[0]; if (!file) return;
        const state=JSON.parse(runtime.briccraftState());
        if (state.worldBusy || state.importBusy) {
            report('Finish the current world operation before importing a saved world file.');
            return;
        }
        // The chooser itself stays unlocked; an accepted selection locks before entering the queue.
        runtime.briccraftWorldBusy();
        enqueue(async () => {
            try {
                if (file.size>MAX_WORLD_BYTES) throw new Error('The world file exceeds the 64 MB limit');
                const source=await file.text();
                const previous=runtime.briccraftSerializeWorld(); if (!previous) return;
                await save(previous);
                if (runtime.briccraftImportWorld(source)) {
                    current=crypto.randomUUID();
                    const imported=describe(source,current,file.name.replace(/\.briccraft$/i,''));
                    runtime.briccraftWorldIdentity(current,imported.name);
                    await save(runtime.briccraftSerializeWorld(),current,imported.name);
                }
            } finally { runtime.briccraftWorldReady(); }
        });
    });
    globalThis.briccraftOpenSource = url => {
        try { const target=new URL(url); if(target.protocol==='https:') window.open(target.href,'_blank','noopener,noreferrer'); }
        catch { /* Only absolute HTTPS source links are opened. */ }
    };
    return {
        saveBeforeImport: source => enqueue(async () => {
            const expected=JSON.parse(source).world_info?.id;
            if(expected && JSON.parse(runtime.briccraftState()).worldId!==expected) throw new Error('The active world changed before import started');
            return save(source);
        },true),
        finishImport: () => {
            const source=runtime.briccraftSerializeWorld();
            return source ? enqueue(()=>save(source),true) : Promise.resolve();
        },
        beginImportedWorld: name => {
            const id=crypto.randomUUID();
            if(!runtime.briccraftCreateWorld('flat',name,'42',id)) throw new Error('Could not create import world');
            current=id;
        },
        ready: () => enqueue(async () => {
            const id=await activeId();
            if(id) await open(id);
            else { current=crypto.randomUUID(); runtime.briccraftWorldIdentity(current,'Sunlit archipelago'); }
            await list();
        }),
    };
}
export function download(source,name,type='application/json') {
    const url=URL.createObjectURL(new Blob([source],{type}));
    const link=document.createElement('a'); link.href=url; link.download=name; link.click();
    setTimeout(() => URL.revokeObjectURL(url),1000);
}
