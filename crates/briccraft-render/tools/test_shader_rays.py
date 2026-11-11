"""Native production-traversal regressions, independent of camera sampling.

Run after installing skia-python. Only the entry point is replaced: all actual
volume bounds, hierarchy, DDA, part intersections and vehicle code are retained.
"""
import json
from pathlib import Path
import tempfile
import numpy as np
from smoke_shader import render, source_for_quality


PROBES = '''half4 main(float2 coord) {
    int probe=int(floor(coord.x));
    float3 origin=float3(56.0,21.0,86.0);
    float x=-0.005;
    if(probe==1) x=-0.00001;
    if(probe==2) x=-0.00000001;
    if(probe==3) x=0.005;
    if(probe==4) x=0.0;
    float3 ray=normalize(float3(x,-0.2,-1.0));
    if(probe==5) { origin.z=60.0;ray=float3(0.0,-1.0,0.0); }
    if(probe==6) { origin=float3(56.0,10.0,60.0);ray=normalize(float3(0.00001,0.2,0.0)); }
    if(probe==7) { origin=float3(-1.0,21.0,60.0);ray=float3(0.0,-1.0,0.0); }
    if(probe==8) { origin.z=60.0;ray=float3(0.0,1.0,0.0); }
    Hit hit=traceScene(origin,ray,360.0);
    bool correct=probe<7 ? hit.id==1.0 : hit.t>=FAR;
    return correct ? half4(0.0,1.0,0.0,1.0) : half4(1.0,0.0,0.0,1.0);
}
'''


def main():
    # A high distant tower expands the global bounds around an otherwise
    # floating platform. The starting camera is inside those bounds, exactly
    # on X=56: the original DDA failed the tiny-negative-X probes here.
    scene = {'dimensions': [96, 72, 96], 'pieces': [
        [[48, 36, 24], [16, 3, 48], [200, 40, 30], 0, 0],
        [[0, 0, 95], [1, 72, 1], [220, 220, 220], 0, 0],
    ]}
    with tempfile.TemporaryDirectory(prefix='briccraft-ray-regressions-') as directory:
        path = Path(directory)
        (path / 'scene.json').write_text(json.dumps(scene))
        for quality in range(3):
            source = source_for_quality(quality).split('half4 main(')[0] + PROBES
            for hierarchy in (False, True):
                actual = source if hierarchy else source.replace('scene_bounds_min.w>0.0', 'false')
                _, pixels = render(path / 'probes.png', path / 'scene.json', quality=quality,
                    width=9, height=1, source=actual, validate=False)
                failed = np.flatnonzero(pixels[0, :, 1] != 255).tolist()
                assert not failed, f'quality={quality}, hierarchy={hierarchy}, failed rays={failed}'
    print(json.dumps({'production_variants': 3, 'rays_per_variant': 18, 'passed': 54}))


if __name__ == '__main__':
    main()
