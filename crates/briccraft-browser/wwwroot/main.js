import { FerroExports, WebRenderTarget } from './ferroui.js';
import { loadModule } from './ferroui-loader.js';
import { installStorage } from './storage-host.js';
import { installInput } from './input-host.js';
import { installImports } from './import-host.js';
import { installAudio } from './audio-host.js';
import { installGraphics } from './graphics-host.js';
import { installCanvasSizing } from './canvas-size.js';
import { installGraphicsPreferences } from './graphics-preferences.js';
import { installDesignFiles } from './design-host.js';
import { installPlaces } from './places-host.js';

export let canvasSizing;

const mount = document.getElementById('out');
const loading = document.getElementById('loading');
const message = document.getElementById('load-message');
try {
    const loaded = await loadModule({ script: './briccraft-browser.js', element: mount });
    if (loaded) {
        const runtime = await loaded.createRuntime();
        FerroExports.attach(runtime);
        globalThis.briccraft = runtime;
        const storage = installStorage(runtime);
        installImports(runtime,storage);
        installDesignFiles(runtime);
        const places = installPlaces(runtime, storage);
        installAudio();
        installInput(runtime, mount);
        installGraphics(mount);
        canvasSizing=installCanvasSizing(mount,WebRenderTarget);
        const graphicsPreferences=installGraphicsPreferences(runtime);
        message.textContent = 'Snapping the last pieces into place…';
        runtime.runMain(location.search);
        storage.ready().finally(() => runtime.briccraftWorldReady());
        let frames = 0;
        let graphicsVerified = false;
        let readyAnnounced = false;
        let preferencesRestored = false;
        const frame = time => {
            try {
                if (!preferencesRestored) {
                    const state=JSON.parse(runtime.briccraftState());
                    if (typeof state.parts==='number') {
                        graphicsPreferences.restore();
                        preferencesRestored=true;
                    }
                }
                // Read the first presented target before preparing another
                // frame: CPU Skia cannot run this world's shader interactively.
                if (!graphicsVerified) {
                    const state = JSON.parse(runtime.briccraftState());
                    if (state.rendering.frames > 0 && state.rendering.frameKind !== 0) {
                        console.info('[BricCraft] rendering', JSON.stringify({
                            rendering: state.rendering,
                            effectFrames: state.renderer.effectFrames,
                            uniformFrames: state.renderer.uniformFrames,
                        }));
                        if (state.rendering.kind !== 'webgl' || state.rendering.glMajorVersion < 3) {
                            throw new Error('WebGL 2 is required to render this world. Enable graphics acceleration or try another browser that supports WebGL 2');
                        }
                        graphicsVerified = true;
                    }
                }
                runtime.briccraftFrame(time, mount.clientWidth, mount.clientHeight);
                places.tick(time);
                if (++frames >= 3 && graphicsVerified && !readyAnnounced) {
                    readyAnnounced = true;
                    loading.classList.add('ready');
                    setTimeout(() => loading.remove(), 500);
                }
                requestAnimationFrame(frame);
            } catch (error) { showError(error); }
        };
        requestAnimationFrame(frame);
    }
} catch (error) { showError(error); }

function showError(error) {
    console.error('BricCraft startup/render failure:', error);
    if (loading.isConnected) {
        message.textContent = `The world could not start: ${error.message || error}. Reload to retry.`;
        loading.classList.remove('ready');
    } else {
        const banner = document.createElement('div');
        banner.setAttribute('role', 'alert');
        banner.style.cssText = 'position:fixed;inset:auto 20px 20px;padding:18px;background:#371e1e;color:white;border:1px solid #f99;z-index:100;font:14px system-ui';
        banner.textContent = `Rendering stopped: ${error.message || error}. Your saved world is retained. Reload to retry.`;
        document.body.append(banner);
    }
}
