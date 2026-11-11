import { FerroExports } from './ferroui.js';
import { loadModule } from './ferroui-loader.js';
import { installStorage } from './storage-host.js';
import { installInput } from './input-host.js';
import { installAudio } from './audio-host.js';

const mount = document.getElementById('out');
const loading = document.getElementById('loading');
const message = document.getElementById('load-message');
try {
    const loaded = await loadModule({ script: './briccraft-browser.js', element: mount });
    if (loaded) {
        const runtime = await loaded.createRuntime();
        FerroExports.attach(runtime);
        globalThis.briccraft = runtime;
        installStorage(runtime);
        installAudio();
        installInput(runtime, mount);
        message.textContent = 'Snapping the last pieces into place…';
        runtime.runMain(location.search);
        let frames = 0;
        const frame = time => {
            try {
                runtime.briccraftFrame(time, mount.clientWidth, mount.clientHeight);
                if (++frames === 3) {
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
