/**
 * Compatibility adapter for the pinned FerroUI browser backend. Its WebGL
 * target hardcodes failIfMajorPerformanceCaveat=true without a platform option;
 * that can reject a browser-approved software GPU and fall back to CPU Skia.
 * Permit WebGL 2 for this app while leaving the browser's context policy intact.
 *
 * CanvasSurface.create attaches the canvas before creating its render target,
 * so containment scopes this to our main-thread canvas. Other canvases and all
 * other context types receive their original arguments. Remove this adapter
 * when FerroUI exposes the corresponding browser platform setting.
 */
export function installGraphics(mount) {
    const Canvas = mount.ownerDocument.defaultView.HTMLCanvasElement;
    const original = Canvas.prototype.getContext;
    const getContext = function (type, attributes, ...rest) {
        if (type === 'webgl2' && this instanceof Canvas && mount.contains(this)
            && attributes && typeof attributes === 'object'
            && attributes.failIfMajorPerformanceCaveat) {
            return original.call(this, type, {
                ...attributes,
                failIfMajorPerformanceCaveat: false,
            }, ...rest);
        }
        return Reflect.apply(original, this, arguments);
    };
    Canvas.prototype.getContext = getContext;
    return () => {
        if (Canvas.prototype.getContext === getContext) {
            Canvas.prototype.getContext = original;
        }
    };
}
