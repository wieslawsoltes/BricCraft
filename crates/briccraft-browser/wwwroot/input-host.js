// Browser lifecycle and key codes only. FerroUI owns pointer hit testing,
// focus, construction gestures, touch contacts, and the entire visible UI.
export function installInput(runtime, mount) {
    const subscriptions = new AbortController();
    const options = { signal: subscriptions.signal };
    const captures = { ...options, capture: true };
    const touches = new Set();
    const position = { x: mount.clientWidth / 2, y: mount.clientHeight / 2 };
    let gestureScale = 1;
    let gestureFromTouch = false;
    let lastControlWheel = -Infinity;
    let touchUsed = false;
    let environmentFrame = 0;
    const coarse = window.matchMedia('(pointer: coarse)');
    const syncEnvironment = () => {
        environmentFrame = 0;
        if (subscriptions.signal.aborted) return;
        const bounds = mount.getBoundingClientRect();
        const visible = window.visualViewport;
        // The native InputPane handles browsers with VirtualKeyboard support.
        // Safari's visible viewport supplies the same occlusion when that API
        // is absent. Ignore page zoom, which is not a keyboard rectangle.
        const keyboard = visible && Math.abs(visible.scale - 1) < 0.01 &&
            bounds.height - visible.height > 80;
        const top = keyboard ? Math.max(0, visible.offsetTop - bounds.top) : 0;
        const bottom = keyboard ? Math.max(0, bounds.bottom - visible.offsetTop - visible.height) : 0;
        runtime.briccraftInputEnvironment(coarse.matches || touchUsed, top, bottom);
    };
    const scheduleEnvironment = () => {
        if (!environmentFrame) environmentFrame = requestAnimationFrame(syncEnvironment);
    };
    const resize = new ResizeObserver(scheduleEnvironment);
    resize.observe(mount);
    coarse.addEventListener('change', scheduleEnvironment, options);
    window.addEventListener('resize', scheduleEnvironment, options);
    window.visualViewport?.addEventListener('resize', scheduleEnvironment, options);
    window.visualViewport?.addEventListener('scroll', scheduleEnvironment, options);
    scheduleEnvironment();
    const isTextInput = target => target instanceof HTMLElement &&
        (target.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(target.tagName));
    const isFrameworkInput = target => target instanceof HTMLElement &&
        (target.classList.contains('ferroui-input-element') || target.classList.contains('ferroui-text-input-element'));
    const isComposition = event => event.isComposing || ['Process', 'Dead'].includes(event.key) || event.keyCode === 229;

    window.addEventListener('keydown', event => {
        if (isComposition(event) || event.altKey) return;
        // The framework's hidden input also carries non-text keys. Let its
        // native focus manager distinguish a TextBox from the game viewport.
        if (isTextInput(event.target) && !isFrameworkInput(event.target)) return;
        const command = event.ctrlKey || event.metaKey;
        const code = command && event.shiftKey && event.code === 'KeyZ' ? 'KeyY' : event.code;
        if (runtime.briccraftKey(code, true, command, event.repeat)) {
            event.preventDefault();
            event.stopPropagation();
        }
    }, captures);
    window.addEventListener('keyup', event => {
        runtime.briccraftKey(event.code, false, event.ctrlKey || event.metaKey, false);
    }, captures);

    const reset = () => { touches.clear(); gestureScale = 1; gestureFromTouch = false; runtime.briccraftBlur(); };
    window.addEventListener('blur', reset, options);
    document.addEventListener('visibilitychange', () => { if (document.hidden) reset(); }, options);
    mount.addEventListener('contextmenu', event => event.preventDefault(), options);
    mount.addEventListener('pointermove', event => {
        const bounds = mount.getBoundingClientRect();
        position.x = event.clientX - bounds.left;
        position.y = event.clientY - bounds.top;
    }, captures);
    mount.addEventListener('pointerdown', event => {
        if (event.pointerType === 'touch') {
            touches.add(event.pointerId);
            gestureFromTouch = true;
            touchUsed = true;
            scheduleEnvironment();
        }
    }, captures);
    const release = event => touches.delete(event.pointerId);
    window.addEventListener('pointerup', release, captures);
    window.addEventListener('pointercancel', release, captures);
    mount.addEventListener('wheel', event => {
        if (event.ctrlKey) { lastControlWheel = performance.now(); event.preventDefault(); }
    }, { ...options, passive: false });

    // Safari on macOS may emit gesture events instead of Ctrl+wheel. On touch
    // screens native pointer gestures already apply the pinch, so never double it.
    mount.addEventListener('gesturestart', event => {
        gestureScale = Number.isFinite(event.scale) && event.scale > 0 ? event.scale : 1;
        gestureFromTouch = touches.size > 0;
        event.preventDefault();
    }, { ...options, passive: false });
    mount.addEventListener('gesturechange', event => {
        event.preventDefault();
        const scale = event.scale;
        if (!Number.isFinite(scale) || scale <= 0) return;
        if (!gestureFromTouch && touches.size === 0 && performance.now() - lastControlWheel > 100) {
            runtime.briccraftGesture(position.x, position.y, -Math.log(scale / gestureScale) / 0.0015);
        }
        gestureScale = scale;
    }, { ...options, passive: false });
    mount.addEventListener('gestureend', () => { gestureScale = 1; gestureFromTouch = false; }, options);
    return () => {
        subscriptions.abort();
        resize.disconnect();
        if (environmentFrame) cancelAnimationFrame(environmentFrame);
        reset();
    };
}
