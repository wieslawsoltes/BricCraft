// Real touch contacts exercise FerroUI hit testing and the production gesture path.
// The caller supplies the already running official browser harness; this module
// neither starts a second browser nor replaces native controls with test DOM.
import path from 'node:path';
import {assert} from '../FerroUI/scripts/browser/harness.mjs';

const current = 'JSON.parse(briccraft.briccraftState())';
const distance = (a, b) => Math.hypot(...a.map((value, index) => value - b[index]));

export async function checkMobile(page, out) {
    const state = async () => JSON.parse(await page.evaluate('briccraft.briccraftState()'));
    const command = (name, value = '') => page.evaluate(
        `briccraft.briccraftCommand(${JSON.stringify(name)},${JSON.stringify(value)})`);
    const wait = expression => page.waitFor(expression, 90000);
    const send = async (method, params) => {
        const result = await page.send(method, params);
        assert(!result.error, `${method}: ${result.error?.message}`);
    };
    const touch = (type, points = []) => send('Input.dispatchTouchEvent', {
        type, touchPoints: points.map(([id, x, y]) => ({id, x, y, radiusX: 5, radiusY: 5, force: 1})),
    });
    const paint = async before => {
        before ??= (await state()).rendering.frames;
        await wait(`${current}.rendering.frames > ${before} && !${current}.worldBusy`);
    };
    const closePanel = async () => {
        if ((await state()).interface.modal) await page.press('Escape');
        await wait(`!${current}.interface.modal`);
    };
    // Sparse construction is independent of the camera, floating origin, and
    // resident chunks. Comparing it exactly detects edits without mistaking a
    // streamed window refresh for a construction change.
    const construction = () => page.evaluate(`(() => {
        const saved = JSON.parse(briccraft.briccraftSerializeWorld());
        return JSON.stringify(saved.stream ? {
            edits: saved.stream.edits, nextUserId: saved.stream.next_user_id,
            seed: saved.stream.seed, preset: saved.stream.preset,
        } : saved.world);
    })()`);
    const original = await page.evaluate('({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})');
    const result = {};
    await closePanel();
    const worldId = (await state()).worldId;
    const unchanged = await construction();
    try {
        await send('Emulation.setTouchEmulationEnabled', {enabled: true, maxTouchPoints: 5});
        for (const layout of [
            {name: 'portrait', width: 390, height: 844, aim: [195, 360], modal: [205, 450]},
            {name: 'landscape', width: 844, height: 390, aim: [300, 170], modal: [620, 260]},
        ]) {
            const {name, width, height, aim: [x, y]} = layout;
            await send('Emulation.setDeviceMetricsOverride', {
                width, height, deviceScaleFactor: 1, mobile: true,
            });
            await wait(`${current}.interface.compact && Math.abs(${current}.interface.width-${width}) < 2 && Math.abs(${current}.interface.height-${height}) < 2`);
            await paint();
            const initial = await state();
            assert(initial.interface.landscape === (name === 'landscape'), `${name}: native orientation must match the viewport`);
            assert(initial.interface.panelWidth + 24 <= initial.interface.width, `${name}: panels must fit their logical viewport`);

            // A touch aims at an actual world pixel; the separate Place control
            // owns editing. It must not synthesize a mouse construction click.
            await touch('touchStart', [[1, x, y]]);
            await touch('touchEnd');
            await paint();
            const aimed = await state();
            assert(aimed.interface.touch, `${name}: native touch mode must activate`);
            assert(distance(aimed.cursor, [x / width * 2 - 1, 1 - y / height * 2]) < 0.005,
                `${name}: touch must reach native world picking`);
            assert(distance(aimed.globalCamera, initial.globalCamera) < 0.0001,
                `${name}: tapping to aim must not move the camera`);
            assert(await construction() === unchanged, `${name}: tapping to aim must not place or remove bricks`);

            await touch('touchStart', [[2, x, y]]);
            await touch('touchMove', [[2, x + 22, y + 9]]);
            await touch('touchMove', [[2, x + 40, y + 18]]);
            await touch('touchEnd');
            const dragged = await state();
            const dragDistance = distance(dragged.globalCamera, aimed.globalCamera);
            assert(dragDistance > 0.01, `${name}: one-finger dragging must orbit the actual camera`);

            const left = [3, x - 55, y];
            const right = [4, x + 55, y];
            await touch('touchStart', [left, right]);
            const spread = [[3, x - 75, y - 3], [4, x + 75, y + 3]];
            await touch('touchMove', spread);
            const pinched = await state();
            assert(distance(pinched.globalCamera, dragged.globalCamera) > 0.01,
                `${name}: two-finger pan and pinch must change the actual camera`);
            // CDP takes the complete active contact set. Removing one point
            // from touchMove emits that point's release; touchEnd ends all.
            await touch('touchMove', [spread[1]]);
            const lifted = await state();
            assert(distance(lifted.globalCamera, pinched.globalCamera) < 0.0001,
                `${name}: lifting one finger must not jump the camera`);
            await touch('touchMove', [[4, x + 86, y + 12]]);
            await touch('touchEnd');
            await paint();
            const resumed = await state();
            assert(distance(resumed.globalCamera, lifted.globalCamera) > 0.01,
                `${name}: the remaining finger must continue a camera drag`);
            assert(await construction() === unchanged, `${name}: drag, pinch, and contact release must not edit geometry`);

            // A real native catalog scroll is isolated from the full-bleed
            // world underneath it, even at the phone layout's tight bounds.
            const catalogFrame = (await state()).rendering.frames;
            await command('catalog');
            await wait(`${current}.interface.catalogOpen && ${current}.interface.modal`);
            await paint(catalogFrame);
            const blocked = await state();
            const [mx, my] = layout.modal;
            await touch('touchStart', [[5, mx, my]]);
            await touch('touchMove', [[5, mx, my - 40]]);
            await touch('touchEnd');
            // An idle modal retains its world bitmap. Observe the frame caused
            // by this gesture, including one already presented before release.
            await paint(blocked.rendering.frames);
            assert((await state()).interface.catalogOpen, `${name}: scrolling inside the catalog must keep it open`);
            assert(distance((await state()).globalCamera, blocked.globalCamera) < 0.0001,
                `${name}: a modal interaction must not move the world camera`);
            assert(await construction() === unchanged, `${name}: a modal interaction must not edit world geometry`);
            await closePanel();
            await paint();
            await page.screenshot(path.join(out, name === 'portrait' ? 'mobile-terrain.png' : 'mobile-landscape.png'));

            if (name === 'portrait') {
                const studioFrame = (await state()).rendering.frames;
                await command('studio');
                await wait(`${current}.interface.studioOpen && ${current}.interface.modal`);
                await paint(studioFrame);
                await page.screenshot(path.join(out, 'mobile-studio.png'));
                await closePanel();
            }
            const observed = await state();
            result[name] = {width, height, dragDistance, interface: observed.interface,
                presented: observed.rendering, worldRendering: observed.worldRendering,
                constructionUnchanged: true};
        }
        assert((await state()).worldId === worldId, 'Mobile gestures must preserve the active terrain world');
        return result;
    } finally {
        await touch('touchCancel').catch(() => {});
        await send('Emulation.setTouchEmulationEnabled', {enabled: false}).catch(() => {});
        await send('Emulation.setDeviceMetricsOverride', {
            width: original.width, height: original.height, deviceScaleFactor: original.dpr, mobile: false,
        }).catch(() => {});
    }
}
