// These checks exercise full-density world surfaces separately from the long
// functional journey, which explicitly selects Performance on CI's software GPU.
import path from 'node:path';
import {assert,differing,sleep} from '../FerroUI/scripts/browser/harness.mjs';

export async function checkImageQuality({page,state,command,wait,current,out}) {
    const original=await page.evaluate('briccraft.briccraftSerializeWorld()');
    const prior=await state();
    const result={};
    const configure=commands=>page.evaluate(`(() => {
        for(const [name,value] of ${JSON.stringify(commands)}) briccraft.briccraftCommand(name,value);
    })()`);
    // A cold, full-density shader draw can take much longer on SwiftShader than
    // the functional journey. Require an actual presented frame, not a timer.
    const presented=async(before,condition)=>page.waitFor(
        `${current}.rendering.frames > ${before} && (${condition})`,240000);
    const density=(value,width,height)=>{
        assert(value.worldRendering.effectWidth===width && value.worldRendering.effectHeight===height,
            `Expected a native ${width} × ${height} world, got ${JSON.stringify(value.worldRendering)}`);
        assert(value.rendering.width===width && value.rendering.height===height,
            'The native UI and the presented target must retain full display resolution');
        assert(value.worldRendering.pixels<=value.worldRendering.pixelBudget,
            'A sharp world must still respect its bounded allocation');
    };
    try {
        await command('settings','false');
        await command('create-world','flat');
        await wait(`${current}.preset === 'flat' && !${current}.worldBusy`);
        await command('blueprint','house');
        assert((await state()).parts>0,'The quality scene must contain actual construction geometry');
        // Freeze simulation and water so the AA comparison has identical inputs.
        let before=(await state()).rendering.frames;
        await configure([['paused','true'],['quality','1'],['antialiasing','2'],['resolution','auto']]);
        await presented(before,`${current}.worldRendering.resolutionMode === 'auto' && ${current}.renderer.samples === 2`);
        const auto=await state();
        density(auto,1280,800);
        assert(auto.worldRendering.minimumPixels===576000,
            'Auto must retain the documented 0.75-per-axis CSS density floor');
        result.auto={world:auto.worldRendering,renderer:auto.renderer,presented:auto.rendering};
        const clip={x:490,y:190,width:500,height:370};
        const twoSamples=await page.screenshot(undefined,clip);

        before=auto.rendering.frames;
        await configure([['resolution','quality'],['antialiasing','4']]);
        await presented(before,`${current}.worldRendering.resolutionMode === 'quality' && ${current}.renderer.samples === 4`);
        const quality=await state();
        density(quality,1280,800);
        assert(quality.worldRendering.requestedSamples===4 && quality.worldRendering.samples===4,
            'Quality must honor the requested AA sample count');
        assert(quality.worldRendering.effectiveQuality===1,
            'Quality must honor the requested shader detail even on a slow software GPU');
        const fourSamples=await page.screenshot(undefined,clip);
        const difference=differing(twoSamples,fourSamples,2,{tolerance:1});
        assert(difference.different>0,'Changing AA must change actual frozen world pixels');
        await page.screenshot(path.join(out,'quality-desktop.png'));
        result.desktop={world:quality.worldRendering,renderer:quality.renderer,
            presented:quality.rendering,aaPixels:difference};

        before=(await state()).rendering.frames;
        await command('settings');
        await presented(before,`${current}.interface.settingsOpen`);
        await page.screenshot(path.join(out,'settings.png'));
        // UI animation/FPS updates can present while the same world snapshot is
        // reused. No new atlas publication or effect is needed for an idle menu.
        const frozen=await state();
        await page.waitFor(`${current}.renderer.effectReuses > ${frozen.renderer.effectReuses}+2`,30000);
        await sleep(350);
        const retained=await state();
        assert(retained.renderer.effectFrames===frozen.renderer.effectFrames,
            'An unchanged paused menu must retain its immutable world effect');
        assert(retained.renderer.uploads===frozen.renderer.uploads,
            'An unchanged menu must not republish world atlas images');
        result.frozenMenu={newEffects:retained.renderer.effectFrames-frozen.renderer.effectFrames,
            atlasUploads:retained.renderer.uploads-frozen.renderer.uploads,
            effectReuses:retained.renderer.effectReuses-frozen.renderer.effectReuses};

        before=retained.rendering.frames;
        await page.resize(390,844);
        await presented(before,`${current}.interface.compact && ${current}.worldRendering.effectWidth === 390 && ${current}.worldRendering.effectHeight === 844`);
        const mobile=await state();
        density(mobile,390,844);
        assert(mobile.interface.panelWidth+24<=390,'The image-quality panel must fit a phone');
        await page.screenshot(path.join(out,'mobile-settings.png'));
        before=mobile.rendering.frames;
        await command('settings','false');
        await presented(before,`!${current}.interface.modal`);
        await page.screenshot(path.join(out,'quality-mobile.png'));
        result.mobile={world:mobile.worldRendering,renderer:mobile.renderer,presented:mobile.rendering};
        result.frozenResize=true;
        return result;
    } finally {
        // Return the functional journey to its explicit, recorded CI settings.
        await configure([['settings','false'],['resolution',prior.worldRendering.resolutionMode],
            ['antialiasing',String(prior.worldRendering.requestedSamples)],
            ['quality',String(prior.worldRendering.requestedQuality)],['paused','false']]);
        await page.resize(1280,800);
        assert(await page.evaluate(`briccraft.briccraftImportWorld(${JSON.stringify(original)})`),
            'The quality gate must restore the caller\'s world');
        await wait(`!${current}.worldBusy && !${current}.interface.modal`);
    }
}
