import { fetchBounded, fetchMap } from './map-source.js';
import { download } from './storage-host.js';

const TOY='https://raw.githubusercontent.com/KhronosGroup/glTF-Sample-Assets/main/Models/ToyCar/glTF-Binary/ToyCar.glb';
const TOY_CREDIT={text:'ToyCar · Guido Odendahl & Eric Chadwick / Khronos',url:'https://github.com/KhronosGroup/glTF-Sample-Assets/tree/main/Models/ToyCar',license:'CC0-1.0',license_url:'https://creativecommons.org/publicdomain/zero/1.0/'};

export function installImports(runtime,storage) {
    let worker,controller,generation=0,report;
    const progress=(text,value=0,busy=true) => runtime.briccraftImportProgress(text,value,busy);
    const stop=() => { generation++; worker?.terminate(); worker=undefined; controller?.abort(); controller=undefined; };
    const fail=error => { stop(); progress(String(error?.message || error),0,false); };
    const convert=async (data,file) => {
        stop(); const token=generation; controller=new AbortController();
        progress('Preparing source geometry…',0.03);
        try {
            await storage.saveBeforeImport(data.snapshot);
            // Cancellation can occur while an IndexedDB save is queued. A stale
            // continuation must not restart progress or reuse a newer controller.
            if(token!==generation) return;
            let bytes;
            if(file) {
                if(file.size>32*1024*1024) throw new Error('Source exceeds the 32 MB input limit');
                bytes=await file.arrayBuffer(); data.name=file.name.replace(/\.[^.]+$/,'');
                if(data.kind!=='osm') {
                    data.kind=/\.glb$/i.test(file.name)?'glb':/\.obj$/i.test(file.name)?'obj':'gltf';
                }
            } else if(data.kind==='osm') {
                if(data.landmark) {
                    data.latitude=52.516275; data.longitude=13.377704; data.radius=160; data.meters_per_stud=4;
                    data.name='Brandenburg Gate';
                    bytes=await fetchBounded('./assets/maps/brandenburg.json',{signal:controller.signal});
                } else {
                    data.name=`Map ${data.latitude.toFixed(4)}, ${data.longitude.toFixed(4)}`;
                    progress('Loading mapped buildings, roads and water…',0.08);
                    bytes=await fetchMap(data,controller.signal);
                }
            } else {
                data.kind='glb'; data.name='Khronos ToyCar'; data.attribution=TOY_CREDIT;
                progress('Loading the CC0 ToyCar model…',0.08);
                bytes=await fetchBounded(TOY,{signal:controller.signal});
            }
            if(token!==generation) return;
            progress('Converting geometry to editable bricks…',0.12);
            worker=new Worker(new URL('./import-worker.js',import.meta.url),{type:'module'});
            worker.onerror=event => { if(token===generation) fail(new Error(event.message || 'Conversion worker failed')); };
            worker.onmessage=async event => {
                if(token!==generation) return;
                const result=event.data;
                if(result.type==='progress') {
                    const p=result.progress;
                    progress(`${p.stage} · ${p.completed.toLocaleString()} / ${p.total.toLocaleString()}`,0.15+0.65*(p.total ? p.completed/p.total : 0));
                } else if(result.type==='error') fail(new Error(result.message));
                else if(result.type==='complete') {
                    worker.terminate(); worker=undefined;
                    try {
                        report=result.report;
                        if(JSON.parse(runtime.briccraftState()).worldId!==data.world_id) throw new Error('The active world changed during conversion; import again in the intended world');
                        if(!data.append) storage.beginImportedWorld(result.name);
                        if(!runtime.briccraftImportPlan(result.source,result.name,!!data.append)) throw new Error(JSON.parse(runtime.briccraftState()).status);
                        progress(`Placing ${report.parts.toLocaleString()} bricks across streamed regions…`,0.82);
                    } catch(error) { fail(error); }
                }
            };
            delete data.snapshot;
            worker.postMessage({...data,bytes},[bytes]);
        } catch(error) { if(token===generation) fail(error); }
    };
    globalThis.briccraftImportSource=(action,payload) => {
        const data=JSON.parse(payload); data.kind=action==='model'?'glb':'osm';
        if((action==='model' && !data.sample) || data.local) {
            // A native file chooser needs the original user gesture, before awaiting storage.
            const input=document.createElement('input'); input.type='file';
            input.accept=data.kind==='osm'?'.json,application/json':'.glb,.gltf,.obj';
            input.addEventListener('change',()=>{ if(input.files?.[0]) convert(data,input.files[0]); },{once:true});
            input.click();
        } else queueMicrotask(()=>convert(data));
    };
    globalThis.briccraftCancelImport=() => { stop(); queueMicrotask(()=>progress('Import cancelled. No partial construction was added.',0,false)); };
    globalThis.briccraftImportFinished=(success,summary) => queueMicrotask(async () => {
        progress(summary,success?1:0,false);
        if(success) {
            try { await storage.finishImport(); } catch(error) { runtime.briccraftMessage(`Imported world is ready; automatic save failed: ${error.message}`); }
        }
    });
    globalThis.briccraftExportReport=source => download(source,'briccraft-import-report.json');
}
