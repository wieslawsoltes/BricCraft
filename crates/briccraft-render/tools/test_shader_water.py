"""Production water visibility: disabled ocean, enabled sea and mapped geometry.

The actual shadeScene entry path and all three production variants are used.
Only the final display entry point is replaced to expose exact ray outcomes.
"""
import json
from pathlib import Path
import tempfile
import numpy as np
from smoke_shader import render, source_for_quality


PROBES = '''half4 main(float2 coord) {
    int probe=int(floor(coord.x));
    float3 origin=camera_position.xyz;
    float3 down=float3(0.0,-1.0,0.0);
    float distance=waterSurfaceDistance(origin,down);
    bool correct=false;
    if(probe==0) {
        // Negative is disabled, while exactly zero is an enabled sea level.
        correct=sun_direction.w<0.0?distance>=FAR:
            abs(distance-(origin.y-sun_direction.w))<0.0001;
    } else if(probe==1) {
        correct=waterSurfaceDistance(origin,float3(1.0,0.0,0.0))>=FAR;
    } else if(probe==2) {
        correct=waterSurfaceDistance(origin,float3(0.0,1.0,0.0))>=FAR;
    } else if(probe==3) {
        // An empty mapped area must show sky, never an unrequested ocean.
        // Enabled island water must still reach the water shading path.
        float3 ray=normalize(float3(3.0,-1.0,0.0));
        float difference=length(shadeScene(ray)-skyColor(ray));
        correct=sun_direction.w<0.0?difference<0.0001:difference>0.01;
    } else {
        // The blue plate represents a mapped river. Turning off the infinite
        // plane must preserve its geometry and its normal material shading.
        Hit hit=traceScene(origin,down,360.0);
        float3 expected=shadeBrick(hit,origin+down*hit.t,down);
        correct=hit.id==1.0 && length(shadeScene(down)-expected)<0.0001;
    }
    return correct?half4(0.0,1.0,0.0,1.0):half4(1.0,0.0,0.0,1.0);
}
'''


def main():
    # A water-colored plate at elevation 4.4, above every enabled ocean tested.
    # The slanted probe leaves its footprint before reaching that elevation.
    scene = {'dimensions': [96, 72, 96], 'position': [48.5, 10.0, 48.5],
        'target': [48.5, 4.4, 48.0], 'pieces': [
            [[48, 10, 48], [2, 1, 2], [29, 125, 177], 2, 0],
        ]}
    with tempfile.TemporaryDirectory(prefix='briccraft-water-regressions-') as directory:
        path = Path(directory)
        for quality in range(3):
            source = source_for_quality(quality).split('half4 main(')[0] + PROBES
            for level in (-1.0, 0.0, 2.4):
                scene['water_level'] = level
                (path / 'scene.json').write_text(json.dumps(scene))
                _, pixels = render(path / 'probes.png', path / 'scene.json',
                    quality=quality, width=5, height=1, source=source, validate=False)
                failed = np.flatnonzero(pixels[0, :, 1] != 255).tolist()
                assert not failed, f'quality={quality}, water_level={level}, failed rays={failed}'
    print(json.dumps({'production_variants': 3, 'water_levels': 3,
        'rays_per_level': 5, 'passed': 45}))


if __name__ == '__main__':
    main()
