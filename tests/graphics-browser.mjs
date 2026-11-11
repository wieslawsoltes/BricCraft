// A/B/A uses one warmed application and records actual presentation, not rAF.
import {assert,sleep} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkCanvasPresentation(page,presentedSample) {
    const read=()=>page.evaluate(`import('./main.js').then(m=>m.canvasSizing.statistics())`);
    const set=value=>page.evaluate(`import('./main.js').then(m=>m.canvasSizing.setEnabled(${value}))`);
    const initial=await read();
    assert(initial.enabled && initial.skipped>0,'The app must skip its unchanged WebGL dimensions');
    const samples=[];
    try {
        for(const [name,enabled] of [['original-1',false],['retained',true],['original-2',false]]) {
            await set(enabled);await sleep(500);
            const before=await read(),presentation=await presentedSample(2500),after=await read();
            assert(after.calls>before.calls,'The diagnostic must observe actual framework size requests');
            const skipped=after.skipped-before.skipped;
            assert(enabled?skipped>0:skipped===0,'The A/B toggle must select the intended sizing behavior');
            assert(after.resizes===before.resizes,'Fixed-view presentation must not need a real resize');
            samples.push({name,enabled,presentation,sizeCalls:after.calls-before.calls,skipped});
        }
    } finally { await set(true); }
    const configs=samples.flatMap(s=>[s.presentation.worldStart,s.presentation.worldEnd])
        .map(s=>JSON.stringify([s.effectWidth,s.effectHeight,s.effectiveQuality]));
    return {samples,comparable:configs.every(s=>s===configs[0]),final:await read()};
}
