"""Verify production spatial AA and material filters in native Skia.

The coverage adapter replaces lighting/display only. Actual primary traversal,
part intersections, rotating assemblies, sample offsets and resolve stay intact.
An eight-times denser center-sampled image is the independent area reference.
This is a native graphics correctness gate, not browser performance evidence.
"""
import argparse
import json
import math
from pathlib import Path
import re
import tempfile
import numpy as np
from smoke_shader import render, source_for_quality


def replace_body(source, name, body):
    match = re.search(r'\bfloat3\s+' + re.escape(name) + r'\s*\([^)]*\)\s*\{', source)
    assert match, f'Production function missing: {name}'
    begin, depth, at = match.end(), 1, match.end()
    while depth:
        if source[at] == '{':
            depth += 1
        elif source[at] == '}':
            depth -= 1
        at += 1
    return source[:begin] + '\n    ' + body + '\n' + source[at-1:]


def coverage_source(source):
    for name, body in (
        ('shadeBrick', 'return float3(0.0);'),
        ('skyColor', 'return float3(1.0);'),
        ('waterColor', 'return float3(1.0);'),
        ('displayColor', 'return color;'),
    ):
        source = replace_body(source, name, body)
    return source


def coverage_fixture():
    # Isolated silhouettes expose stud caps, slopes, holes, thin members,
    # quarter-turned wheels and an independently rotated moving assembly.
    pieces = [
        [[35, 10, 35], [12, 3, 4], [210, 50, 30], 0, 0],
        [[51, 10, 35], [4, 9, 7], [210, 50, 30], 3, 0],
        [[36, 10, 43], [4, 15, 4], [210, 50, 30], 4, 0],
        [[43, 10, 43], [3, 10, 4], [210, 50, 30], 5, 0],
        [[48, 10, 43], [4, 10, 3], [210, 50, 30], 5, 1],
        [[54, 10, 43], [1, 18, 6], [210, 50, 30], 9, 0],
        [[35, 10, 53], [1, 3, 12], [210, 50, 30], 8, 0],
        [[41, 10, 52], [1, 15, 6], [210, 50, 30], 14, 0],
        [[47, 10, 53], [1, 9, 4], [210, 50, 30], 6, 0],
        [[54, 10, 53], [8, 1, 5], [210, 50, 30], 12, 0],
        [[40, 10, 62], [4, 12, 4], [210, 50, 30], 13, 0],
        [[47, 12, 61], [8, 1, 1], [210, 50, 30], 7, 0],
    ]
    return {
        'dimensions': [96, 72, 96], 'pieces': pieces,
        'position': [70.0, 30.0, 78.0], 'target': [47.0, 6.0, 48.0],
        'water_level': -100.0,
        'vehicle': {
            'pieces': [
                [[0, 0, 0], [6, 2, 3], [240, 180, 40], 0, 0],
                [[1, 2, 0], [3, 3, 3], [240, 180, 40], 3, 1],
                [[0, 0, 3], [3, 8, 2], [40, 40, 50], 5, 1],
            ],
            'pivot': [3.0, 0.4, 2.0], 'position': [64.0, 6.0, 46.0],
            'yaw': 0.55, 'pitch': 0.19, 'roll': -0.12, 'wheel_angle': 0.73,
        },
    }


def numerical_filters(source, output):
    # Midpoint quadrature is deliberately independent of the shader's closed
    # forms. Include negative coordinates and footprints spanning many periods.
    cases, expected = [], []
    taps = (np.arange(65536, dtype=float) + 0.5) / 65536 - 0.5
    for value, spacing, radius, width in (
        (-2.993, 1.0, 0.018, 0.004), (-0.015, 1.0, 0.018, 0.12),
        (0.49, 1.0, 0.018, 1.0), (0.93, 1.0, 0.018, 3.4),
        (52.001, 8.0, 0.035, 0.005), (64.2, 8.0, 0.035, 32.0),
        (-17.4, 4.0, 0.018, 7.3), (3.98, 4.0, 0.018, 0.06),
    ):
        x = value + taps * width
        distance = np.abs(np.remainder(x + spacing * 0.5, spacing) - spacing * 0.5)
        cases.append(f'periodicLine({value},{spacing},{radius},{width})')
        expected.append(float(np.mean(distance < radius)))
    for value, lo, hi, width in (
        (0.0, 0.0, 0.04, 0.08), (0.24, 0.228, 0.251, 0.08),
        (-0.15, -0.2, -0.1, 0.3), (0.5, 0.0, 1.0, 4.0),
    ):
        x = value + taps * width
        cases.append(f'intervalCoverage({value},{lo},{hi},{width})')
        expected.append(float(np.mean((x >= lo) & (x <= hi))))
    for phase, width in ((0.7, 0.0001), (-1.2, 0.8), (3.7, 9.2), (0.4, 40.0)):
        cases.append(f'0.5+0.5*filteredSine({phase},{width})')
        expected.append(float(0.5 + 0.5 * np.sin(phase + taps * width).mean()))
    branches = '\n'.join(f'    if(index=={i}) value={expression};'
                         for i, expression in enumerate(cases))
    entry = ('half4 main(float2 coord) {\n    int index=int(floor(coord.x));\n'
             '    float value=0.0;\n' + branches +
             '\n    return half4(value,value,value,1.0);\n}\n')
    probe_source = source.split('half4 main(')[0] + entry
    _, pixels = render(output / 'filter-probes.png', quality=0,
        width=len(cases), height=1, source=probe_source, validate=False)
    actual = pixels[0, :, 0].astype(float) / 255.0
    error = np.abs(actual - np.array(expected))
    assert error.max() < 0.007, f'Material coverage integral differs: {error.tolist()}'
    return {'probes': len(cases), 'maximum_error': float(error.max()),
            'comparison': '65536-tap independent midpoint quadrature; RGBA8 output'}


def footprint_probes(source, output):
    # Compare the shader's analytic surface differential against independent
    # finite differences of ray/plane intersections, including a rotated body.
    width, height, distance, epsilon = 8, 1, 0.30, 0.00001
    tangent = math.tan(math.radians(52.0) * 0.5)
    normals = np.array([[0.0, 0.0, -1.0], [0.3, 0.4, -1.0],
                        [-0.5, 0.1, -1.0], [0.2, -0.6, -1.0]])
    normals /= np.linalg.norm(normals, axis=1, keepdims=True)
    yaw, pitch, roll = 0.7, 0.2, -0.3
    cy, sy, cp, sp, cr, sr = math.cos(yaw), math.sin(yaw), math.cos(-pitch), math.sin(-pitch), math.cos(roll), math.sin(roll)
    rotation = (np.array([[cy, 0, sy], [0, 1, 0], [-sy, 0, cy]])
                @ np.array([[1, 0, 0], [0, cp, -sp], [0, sp, cp]])
                @ np.array([[cr, -sr, 0], [sr, cr, 0], [0, 0, 1]]))
    scene = {
        'dimensions': [96, 72, 96], 'pieces': [],
        'position': [0.0, 0.0, 0.0], 'target': [0.0, 0.0, 1.0],
        'vehicle': {'pieces': [[[0, 0, 0], [1, 1, 1], [255, 0, 0], 0, 0]],
                    'yaw': yaw, 'pitch': pitch, 'roll': roll},
    }
    scene_path = output / 'footprint-scene.json'
    scene_path.write_text(json.dumps(scene))
    branches = '\n'.join(
        '    if(index==' + str(i) + ' || index==' + str(i+4) + ') normal=float3(' +
        ','.join(format(v, '.10f') for v in normal) + ');'
        for i, normal in enumerate(normals))
    entry = ('half4 main(float2 coord) {\n    int index=int(floor(coord.x));\n'
             '    float3 normal=float3(0.0,0.0,-1.0);\n' + branches +
             '\n    float moving=index>=4?1.0:0.0;\n'
             '    if(moving>0.5) normal=vehicleDirection(normal);\n'
             '    float3 footprint=surfaceFootprint(0.30,float3(0.0,0.0,1.0),normal,moving);\n'
             '    return half4(half3(footprint),1.0);\n}\n')
    probe_source = source.split('half4 main(')[0] + entry

    def ray(x, y):
        v = np.array([x*2*tangent/height, -y*2*tangent/height, 1.0])
        return v / np.linalg.norm(v)

    records = []
    for samples in (1, 2, 4):
        expected = []
        for moving in (False, True):
            for normal in normals:
                def intersect(x, y):
                    direction = ray(x, y)
                    return direction * (distance * normal[2] / np.dot(direction, normal))
                dx = (intersect(epsilon, 0) - intersect(-epsilon, 0)) / (2*epsilon)
                dy = (intersect(0, epsilon) - intersect(0, -epsilon)) / (2*epsilon)
                if moving:
                    dx, dy = rotation.T @ dx, rotation.T @ dy
                expected.append((np.abs(dx) + np.abs(dy)) / math.sqrt(samples))
        _, pixels = render(output / f'footprints-{samples}.png', scene_path,
            quality=0, samples=samples, width=width, height=height,
            source=probe_source, validate=False)
        actual = pixels[0].astype(float) / 255.0
        error = np.abs(actual - np.array(expected))
        assert error.max() < 0.007, f'Footprint differential mismatch at {samples} samples: {error.tolist()}'
        records.append({'samples': samples, 'probes': len(expected),
                        'maximum_error': float(error.max())})
    return records


def run(output):
    output.mkdir(parents=True, exist_ok=True)
    scene_path = output / 'coverage-scene.json'
    scene_path.write_text(json.dumps(coverage_fixture()))
    report = {
        'backend': 'native Skia CPU raster',
        'material_filters': numerical_filters(source_for_quality(0), output),
        'surface_differentials': footprint_probes(source_for_quality(0), output),
        'coverage': [],
    }
    width, height, factor = 72, 48, 8
    for quality in range(3):
        source = coverage_source(source_for_quality(quality))
        _, reference = render(output / f'q{quality}-dense-reference.png', scene_path,
            quality=quality, samples=1, width=width*factor, height=height*factor,
            source=source, validate=False)
        reference = reference[..., 0].astype(float).reshape(
            height, factor, width, factor).mean(axis=(1, 3)) / 255.0
        edge_pixels = (reference > 0.01) & (reference < 0.99)
        assert edge_pixels.sum() >= 30, 'Coverage fixture has too few resolved edges'
        assert (reference < 0.5).sum() > width*height*0.02, 'Coverage fixture geometry is missing'
        errors, measurements = {}, []
        for samples in (1, 2, 4):
            measurement, pixels = render(output / f'q{quality}-samples-{samples}.png', scene_path,
                quality=quality, samples=samples, width=width, height=height,
                source=source, validate=False)
            actual = pixels[..., 0].astype(float) / 255.0
            error = np.abs(actual-reference)
            errors[samples] = float(error.mean())
            measurements.append({
                'samples': samples, 'mean_absolute_coverage_error': errors[samples],
                'edge_mean_absolute_error': float(error[edge_pixels].mean()),
                'draw_ms': measurement['draw_ms'], 'compile_ms': measurement['compile_ms'],
            })
        assert errors[4] < errors[1]*0.95, f'Four-sample coverage did not improve: q{quality} {errors}'
        assert errors[2] < errors[1]*1.05, f'Two-sample coverage regressed: q{quality} {errors}'
        report['coverage'].append({
            'quality': quality, 'size': [width, height], 'reference_factor': factor,
            'edge_pixels': int(edge_pixels.sum()), 'measurements': measurements,
        })
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, help='Keep native probe images and numeric report')
    args = parser.parse_args()
    if args.output:
        run(args.output)
    else:
        with tempfile.TemporaryDirectory(prefix='briccraft-aa-') as directory:
            run(Path(directory))
