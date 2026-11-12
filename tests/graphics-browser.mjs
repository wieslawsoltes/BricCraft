// A/B/A uses the same four native UI changes in each phase. Retained idle UI
// need not draw during a fixed timer, especially on CI's software GPU.
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkCanvasPresentation(page) {
    const read=()=>page.evaluate(`import('./main.js').then(m=>{
        const value=JSON.parse(briccraft.briccraftState());
        return {timestamp:performance.now(),sizing:m.canvasSizing.statistics(),
            rendering:value.rendering,renderer:value.renderer,worldRendering:value.worldRendering};
    })`);
    const set=value=>page.evaluate(`import('./main.js').then(m=>m.canvasSizing.setEnabled(${value}))`);
    const initial=await read();
    assert(initial.sizing.enabled && initial.sizing.skipped>0,
        `The app must skip its unchanged WebGL dimensions: ${JSON.stringify(initial.sizing)}`);
    const samples=[];
    let rotations=0;
    try {
        for(const [name,enabled] of [['original-1',false],['retained',true],['original-2',false]]) {
            await set(enabled);
            const before=await read();
            for(let step=0;step<4;step++) {
                const frame=await page.evaluate(`(() => {
                    const frame=JSON.parse(briccraft.briccraftState()).rendering.frames;
                    briccraft.briccraftCommand('rotate','');
                    return frame;
                })()`);
                rotations=(rotations+1)%4;
                try {
                    await page.waitFor(`JSON.parse(briccraft.briccraftState()).rendering.frames > ${frame}`,90000);
                } catch(error) {
                    const stalled=await read();
                    throw new Error(`${name} rotation ${step+1} must present a native frame; `+
                        `startFrame=${frame}, currentFrame=${stalled.rendering.frames}, `+
                        `before=${JSON.stringify(before.sizing)}, current=${JSON.stringify(stalled.sizing)}; `+
                        String(error.message).split('\n')[0]);
                }
            }
            const after=await read();
            const seconds=(after.timestamp-before.timestamp)/1000;
            const frames=after.rendering.frames-before.rendering.frames;
            const presentation={seconds,frames,fps:frames/Math.max(seconds,0.001),
                effectFrames:after.renderer.effectFrames-before.renderer.effectFrames,
                atlasUploads:after.renderer.uploads-before.renderer.uploads,
                start:before.rendering,end:after.rendering,
                worldStart:before.worldRendering,worldEnd:after.worldRendering};
            const sizeCalls=after.sizing.calls-before.sizing.calls;
            const skipped=after.sizing.skipped-before.sizing.skipped;
            const diagnostic=`${name}: frames=${frames}, before=${JSON.stringify(before.sizing)}, after=${JSON.stringify(after.sizing)}`;
            assert(sizeCalls>0,`The diagnostic must observe actual framework size requests; ${diagnostic}`);
            assert(enabled?skipped>0:skipped===0,`The A/B toggle must select the intended sizing behavior; ${diagnostic}`);
            assert(after.sizing.resizes===before.sizing.resizes,`Fixed-view presentation must not need a real resize; ${diagnostic}`);
            samples.push({name,enabled,presentation,sizeCalls,skipped});
        }
    } finally {
        await set(true);
        // Restore selection orientation even when a presented-frame wait fails.
        if(rotations) await page.evaluate(`(() => {
            for(let turn=${rotations};turn<4;turn++) briccraft.briccraftCommand('rotate','');
        })()`);
    }
    const configs=samples.flatMap(s=>[s.presentation.worldStart,s.presentation.worldEnd])
        .map(s=>JSON.stringify([s.effectWidth,s.effectHeight,s.effectiveQuality,s.samples,s.resolutionMode]));
    return {activity:'controlled-ui',rotationsPerPhase:4,samples,
        comparable:configs.every(s=>s===configs[0]),final:(await read()).sizing};
}
