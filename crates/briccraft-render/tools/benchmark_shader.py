"""Compare exact production SkSL against a Git revision at identical settings.

The default CPU raster backend needs skia-python. --backend egl also needs
PyOpenGL/EGL and reports its actual native OpenGL driver. This is not a browser
benchmark. First draws are retained but excluded from the steady medians.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import numpy as np
from smoke_shader import render, source_for_quality


ROOT = Path(__file__).resolve().parents[3]


def baseline_source(ref, quality):
    def read(path):
        return subprocess.check_output(['git', 'show', f'{ref}:{path}'], cwd=ROOT, text=True)
    directory = 'crates/briccraft-render/src/'
    rust = read(directory + 'effect.rs')
    name = ('FAST_WORLD_SKSL', 'BALANCED_WORLD_SKSL', 'WORLD_SKSL')[quality]
    declaration = re.search(rf'pub const {name}: &str = concat!\((.*?)\n\);', rust, re.S)
    if declaration is None:
        raise ValueError(f'{name} is absent from {ref}')
    return ''.join(read(directory + path)
        for path in re.findall(r'include_str!\("([^"]+)"\)', declaration.group(1)))


def traversal_counts(source, scene, quality, width, height, output):
    start, end = source.index('Hit traceVolume('), source.index('\nfloat3 vehicleLocal(')
    trace = source[start:end]
    loop = 'for(int iteration=0;iteration<384;++iteration) {'
    if loop not in trace:
        raise ValueError('Traversal loop changed; update the diagnostic counter adapter')
    trace = trace.replace(loop, 'int count=0; ' + loop + ' count=iteration+1;', 1)
    trace = trace.replace('if(best.t<FAR) return best;',
        'if(best.t<FAR) { best.id=float(count);return best; }', 1)
    position = trace.rindex('return best;')
    trace = trace[:position] + 'best.id=float(count);' + trace[position:]
    source = source[:start] + trace + source[end:]
    # Count the center ray independently of the sampling/display resolve.
    # The camera basis and opaque-water limit match both production revisions.
    source = source[:source.index('half4 main(')] + '''
half4 main(float2 coord) {
    float2 uv=coord/layer_size*2.0-1.0;
    float aspect=layer_size.x/max(layer_size.y,1.0);
    float3 ray=normalize(camera_forward.xyz+camera_right.xyz*uv.x*aspect*camera_forward.w
        -camera_up.xyz*uv.y*camera_forward.w);
    float3 origin=camera_position.xyz;
    float waterT=FAR;
    if(abs(ray.y)>0.000001) {
        float candidate=(sun_direction.w-origin.y)/ray.y;
        if(candidate>0.0) waterT=candidate;
    }
    Hit hit=traceScene(origin,ray,min(360.0,waterT+8.0));
    return half4(mod(hit.id,256.0)/255.0,floor(hit.id/256.0)/255.0,0.0,1.0);
}
'''
    _, pixels = render(output, scene, quality=quality, width=width, height=height,
        source=source, validate=False)
    count = pixels[..., 0].astype(np.uint16) + pixels[..., 1].astype(np.uint16) * 256
    return {'mean': float(count.mean()), 'p95': float(np.percentile(count, 95)),
        'maximum': int(count.max()), 'limit_exhausted_pixels': int(np.count_nonzero(count == 384))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-ref', default='HEAD')
    parser.add_argument('--scene', type=Path)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--backend', choices=('raster', 'egl'), default='raster')
    parser.add_argument('--frames', type=int, default=12)
    parser.add_argument('--samples', type=int, choices=(1, 2, 4), default=1)
    parser.add_argument('--width', type=int, default=320)
    parser.add_argument('--height', type=int, default=200)
    parser.add_argument('--qualities', type=int, choices=(0, 1, 2), nargs='+', default=[0, 1, 2])
    parser.add_argument('--count-traversal', action='store_true')
    args = parser.parse_args()
    if args.frames < 2 or min(args.width, args.height) < 1:
        parser.error('At least two frames and positive dimensions are required')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    results = []
    for quality in args.qualities:
        sources = {'baseline': baseline_source(args.baseline_ref, quality), 'current': source_for_quality(quality)}
        samples, images = {}, {}
        # Alternate order across quality levels to expose rather than hide
        # shared-runner scheduling/frequency variance.
        for name in (('baseline', 'current') if quality % 2 == 0 else ('current', 'baseline')):
            image = args.output.with_name(f'{args.output.stem}.q{quality}.{name}.png')
            samples[name], images[name] = render(image, args.scene, quality=quality,
                width=args.width, height=args.height, frames=args.frames, samples=args.samples, source=sources[name],
                backend=args.backend, animate_uniforms=True)
            samples[name]['source_sha256'] = hashlib.sha256(sources[name].encode()).hexdigest()
            if args.count_traversal:
                samples[name]['primary_iterations'] = traversal_counts(sources[name], args.scene,
                    quality, args.width, args.height, image.with_suffix('.steps.png'))
        difference = np.abs(images['current'].astype(np.int16)-images['baseline'].astype(np.int16))
        results.append({'quality': quality, **samples, 'difference': {
            'mean_absolute_channel': float(difference.mean()),
            'maximum_channel': int(difference.max()),
            'fraction_pixels_over_8': float((difference.max(axis=2) > 8).mean())}})
    report = {'baseline_ref': args.baseline_ref, 'scene': str(args.scene) if args.scene else 'mixed fixture',
        'samples': args.samples, 'measurements': results}
    args.output.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
