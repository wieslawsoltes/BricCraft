/** Keep the owned WebGL drawing buffer when FerroUI repeats its current size.
 * The pinned platform calls setSize at every begin_draw. Actual resize, foreign
 * targets, software targets and worker canvases retain the framework behavior.
 */
export function installCanvasSizing(mount, RenderTarget) {
    const Canvas=mount.ownerDocument.defaultView.HTMLCanvasElement;
    const original=RenderTarget.setSize;
    const counts={calls:0,skipped:0,resizes:0};
    let enabled=true;
    function setSize(target,width,height) {
        const canvas=target?.canvas;
        const owned=target?.renderTargetType==='webgl' && canvas instanceof Canvas && mount.contains(canvas);
        if(owned) {
            counts.calls++;
            const same=canvas.width===width && canvas.height===height;
            if(enabled && same) { counts.skipped++; return; }
            if(!same) counts.resizes++;
        }
        return Reflect.apply(original,this,arguments);
    }
    RenderTarget.setSize=setSize;
    return {
        statistics:()=>({...counts,enabled}),
        // Used for a same-page A/B/A presentation diagnostic; this is not UI state.
        setEnabled(value) { enabled=!!value; },
        dispose() { if(RenderTarget.setSize===setSize) RenderTarget.setSize=original; },
    };
}
