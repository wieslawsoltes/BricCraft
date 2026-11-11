// File services only; the visible design library and editors are native FerroUI.
const MAX_DESIGN_BYTES=256*1024;

export function installDesignFiles(runtime) {
    globalThis.briccraftDesignFile=(action,source)=>{
        if(action==='export') {
            try {
                const document=JSON.parse(source);
                const name=String(document.design?.name || 'Construction')
                    .replace(/[\\/:*?"<>|\u0000-\u001f]/g,'-').slice(0,64);
                const blob=new Blob([source],{type:'application/json'});
                const url=URL.createObjectURL(blob);
                const link=globalThis.document.createElement('a');
                link.href=url;link.download=`${name}.bricdesign`;
                link.hidden=true;globalThis.document.body.append(link);
                link.click();link.remove();
                setTimeout(()=>URL.revokeObjectURL(url),30000);
            } catch(error) { runtime.briccraftMessage(`Could not export the design: ${error.message || error}`); }
        } else if(action==='import') {
            const world=JSON.parse(runtime.briccraftState()).worldId;
            const input=document.createElement('input');
            input.type='file';input.accept='.bricdesign,application/json';input.hidden=true;
            document.body.append(input);
            const cleanup=()=>input.remove();
            input.addEventListener('cancel',cleanup,{once:true});
            input.addEventListener('change',async()=>{
                try {
                    const file=input.files?.[0];
                    if(!file)return;
                    if(file.size>MAX_DESIGN_BYTES)throw new Error('Design files must be at most 256 KiB');
                    const text=await file.text();
                    if(JSON.parse(runtime.briccraftState()).worldId!==world)
                        throw new Error('The world changed while opening the file. Import it again into the current world');
                    runtime.briccraftImportDesign(text);
                } catch(error) { runtime.briccraftMessage(`Could not import the design: ${error.message || error}`); }
                finally { cleanup(); }
            },{once:true});
            input.click();
        }
    };
}
