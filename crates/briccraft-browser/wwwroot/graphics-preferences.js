// Graphics choices belong to this browser, independently of the current world.
export const GRAPHICS_KEY = 'briccraft.graphics.v1';
export function graphicsPreferences(value) {
    if (!value || value.version !== 1) return {version:1,mode:0,samples:2};
    return {
        version:1,
        mode:[0,1,2].includes(value.mode) ? value.mode : 0,
        samples:[1,2,4].includes(value.samples) ? value.samples : 2,
    };
}
export function installGraphicsPreferences(runtime, host=globalThis) {
    let preference=graphicsPreferences(null), restoring=false;
    try { preference=graphicsPreferences(JSON.parse(host.localStorage.getItem(GRAPHICS_KEY))); } catch {}
    const save=(mode,samples)=>{
        preference=graphicsPreferences({version:1,mode,samples});
        if (!restoring) {
            try { host.localStorage.setItem(GRAPHICS_KEY,JSON.stringify(preference)); } catch {}
        }
    };
    host.briccraftSaveGraphics=save;
    return {
        restore() {
            restoring=true;
            const saved={...preference};
            try {
                runtime.briccraftCommand('resolution',String(saved.mode));
                runtime.briccraftCommand('antialiasing',String(saved.samples));
            } finally { restoring=false; }
        },
        current:()=>({...preference}),
        dispose() { if(host.briccraftSaveGraphics===save) delete host.briccraftSaveGraphics; },
    };
}
