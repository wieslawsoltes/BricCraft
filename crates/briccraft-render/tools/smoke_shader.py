"""Compile and render the production SkSL in native Skia (pip install skia-python).

This is a graphics regression fixture, not a second game renderer. The shader,
packing format, camera, and material path are the ones used by the browser app.
Run: python crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-shader.png
Optional second argument: a scene JSON written by the export_atlas example.
"""
from pathlib import Path
import argparse
import json
import math
import re
import statistics
import time
import functools
import importlib.util
import skia
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
def source_for_quality(quality):
    """Read the exact include_str sequence of the production Rust effect."""
    name = ("FAST_WORLD_SKSL", "BALANCED_WORLD_SKSL", "WORLD_SKSL")[quality]
    rust = (ROOT / "src/effect.rs").read_text()
    declaration = re.search(rf"pub const {name}: &str = concat!\((.*?)\n\);", rust, re.S)
    assert declaration, f"Production source declaration missing: {name}"
    files = re.findall(r'include_str!\("([^"]+)"\)', declaration.group(1))
    assert files, f"Production source has no shader files: {name}"
    return "".join((ROOT / "src" / filename).read_text() for filename in files)


SOURCE = source_for_quality(2)


@functools.lru_cache(maxsize=1)
def native_gl_backend():
    # Lazy file-local import also supports loading this tool with importlib.
    spec = importlib.util.spec_from_file_location('briccraft_native_gl', ROOT / 'tools/native_gl.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.NativeGlBackend(skia)


def fixture():
    pieces = []
    def add(origin, size, color, shape=0, rotation=0):
        pieces.append((origin, size, color, shape, rotation))
    for x in range(24, 72, 4):
        for z in range(24, 72, 4):
            add([x, 3, z], [4, 3, 4], [73 + (x % 3) * 5, 139, 57])
    for x in range(37, 49, 2):
        for y in range(6, 15, 3):
            add([x, y, 38], [2, 3, 2], [238, 204, 130])
            add([x, y, 48], [2, 3, 2], [238, 204, 130])
    for z in range(40, 48, 2):
        for y in range(6, 15, 3):
            add([37, y, z], [2, 3, 2], [238, 204, 130])
            add([47, y, z], [2, 3, 2], [238, 204, 130])
    for x in range(35, 51, 2):
        add([x, 15, 36], [2, 6, 7], [177, 56, 35], 3)
        add([x, 15, 43], [2, 6, 7], [177, 56, 35], 3, 2)
    for x, z in [(28, 31), (58, 33), (60, 61), (31, 63)]:
        add([x, 6, z], [2, 9, 2], [102, 67, 40], 4)
        for tier in range(3):
            width = 8 - tier * 2
            add([x + 1 - width // 2, 15 + tier * 3, z + 1 - width // 2],
                [width, 3, width], [44, 110 + tier * 15, 71])
    # A mixed part inspection row beside the house.
    for i, shape in enumerate([0, 1, 2, 3, 4, 5, 6, 8, 9, 11, 12, 13]):
        add([28 + i * 3, 6, 55], [2, 6 if shape > 2 else 3, 2],
            [233, 70 + i * 7, 50 + i * 11], shape)
    add([49, 6, 63], [6, 2, 3], [29, 125, 177])
    add([50, 8, 63], [3, 4, 3], [238, 173, 47])
    return pieces


def images(pieces, dimensions, moving=()):
    nx, ny, nz = dimensions
    occupancy = np.zeros((ny, nz, nx), dtype=np.uint32)
    for i, (origin, size, *_rest) in enumerate(pieces, 1):
        x, y, z = origin
        sx, sy, sz = size
        occupancy[y:y+sy, z:z+sz, x:x+sx] = i
    coarse_size = [math.ceil(nx/4), math.ceil((ny+1)/6), math.ceil(nz/4)]
    coarse = np.zeros((coarse_size[1], coarse_size[2], coarse_size[0]), dtype=np.uint32)
    ys, zs, xs = np.where(occupancy > 0)
    coarse[ys//6, zs//4, xs//4] = 1
    coarse[(ys+1)//6, zs//4, xs//4] = 1
    vehicle_min, vehicle_size = np.zeros(3, dtype=int), np.zeros(3, dtype=int)
    local = np.zeros(0, dtype=np.uint32)
    if moving:
        origins = np.array([piece[0] for piece in moving], dtype=int)
        sizes = np.array([piece[1] for piece in moving], dtype=int)
        vehicle_min = origins.min(axis=0)
        vehicle_size = (origins+sizes).max(axis=0)-vehicle_min
        vx, vy, vz = vehicle_size
        local = np.zeros((vy, vz, vx), dtype=np.uint32)
        for i, (origin, size, *_rest) in enumerate(moving, len(pieces)+1):
            x, y, z = np.array(origin)-vehicle_min
            sx, sy, sz = size
            local[y:y+sy, z:z+sz, x:x+sx] = i
    coarse_offset = occupancy.size+local.size
    ids = np.concatenate((occupancy.ravel(), local.ravel(), coarse.ravel()))
    height_offset = len(ids)
    heights = np.zeros((coarse_size[2], coarse_size[0], 3), dtype=np.uint8)
    for origin, size, *_rest in pieces:
        x, y, z = origin
        sx, sy, sz = size
        columns = heights[z//4:math.ceil((z+sz)/4), x//4:math.ceil((x+sx)/4), 0]
        np.maximum(columns, y+sy, out=columns)
    for channel, stride in ((1, 4), (2, 16)):
        for z in range(0, coarse_size[2], stride):
            for x in range(0, coarse_size[0], stride):
                heights[z:z+stride, x:x+stride, channel] = heights[z:z+stride, x:x+stride, 0].max()
    pixels = np.zeros((math.ceil((len(ids)+heights.shape[0]*heights.shape[1])/1024), 1024, 4), dtype=np.uint8)
    pixels[..., 3] = 255
    flat = pixels.reshape(-1, 4)
    flat[:len(ids), 0] = ids & 255
    flat[:len(ids), 1] = (ids >> 8) & 255
    flat[:len(ids), 2] = (ids >> 16) & 255
    flat[height_offset:height_offset+heights.shape[0]*heights.shape[1], :3] = heights.reshape(-1, 3)
    all_parts = list(pieces)+list(moving)
    records = np.zeros((math.ceil((len(all_parts)+1)*4/1024), 1024, 4), dtype=np.uint8)
    records[..., 3] = 255
    data = records.reshape(-1, 4)
    for i, (origin, size, color, shape, rotation) in enumerate(all_parts, 1):
        is_moving = i > len(pieces)
        data[i*4, :3] = np.array(origin)-vehicle_min if is_moving else origin
        data[i*4+1, :3] = size
        data[i*4+2, :3] = color
        data[i*4+3, :3] = [shape, rotation, int(is_moving)]
    return (pixels, records), {
        'vehicle_min': vehicle_min, 'vehicle_size': vehicle_size,
        'coarse_offset': coarse_offset, 'vehicle_offset': occupancy.size,
        'height_offset': height_offset,
    }


def render(output, scene=None, *, quality=1, samples=1, width=800, height=500, frames=1, source=None,
           backend='raster', animate_uniforms=False, validate=True):
    if samples not in (1, 2, 4):
        raise ValueError('samples must be 1, 2 or 4')
    if source is None:
        source = source_for_quality(quality)
    dimensions = [96, 72, 96]
    pieces = fixture()
    position = np.array([75.0, 30.0, 87.0])
    target = np.array([46.0, 5.0, 45.0])
    content = {}
    if scene is not None:
        content = json.loads(Path(scene).read_text())
        dimensions = content['dimensions']
        pieces = content['pieces']
        position = np.array(content.get('position', position), dtype=float)
        target = np.array(content.get('target', target), dtype=float)
    vehicle = content.get('vehicle', {})
    moving = vehicle.get('pieces', [])
    pixel_images, atlas = images(pieces, dimensions, moving)
    started = time.perf_counter()
    effect = skia.RuntimeEffect.MakeForShader(source)
    assert effect, 'Production SkSL failed to compile'
    compile_ms = (time.perf_counter()-started)*1000
    builder = skia.RuntimeShaderBuilder(effect)
    forward = target-position
    forward /= np.linalg.norm(forward)
    right = np.array([forward[2], 0.0, -forward[0]])
    right /= np.linalg.norm(right)
    up = np.cross(forward, right)
    sun = np.array([-0.45, 0.83, -0.32])
    sun /= np.linalg.norm(sun)
    lower, upper = np.zeros(3), np.zeros(3)
    if pieces:
        origins = np.array([piece[0] for piece in pieces])
        sizes = np.array([piece[1] for piece in pieces])
        lower = origins.min(axis=0)*[1.0, 0.4, 1.0]
        upper = (origins+sizes).max(axis=0)*[1.0, 0.4, 1.0]+[0.0, 0.18, 0.0]
    values = {
        'layer_size': [width, height], 'layer_scale': 1.0,
        'world_dimensions': dimensions+[float(atlas['coarse_offset'])],
        'scene_bounds_min': list(lower)+[float(atlas['height_offset'])], 'scene_bounds_max': list(upper)+[float(bool(pieces))],
        'cell_texture_size': [pixel_images[0].shape[1], pixel_images[0].shape[0]],
        'part_texture_size': [pixel_images[1].shape[1], pixel_images[1].shape[0]],
        'camera_position': list(position)+[content.get('time', 4.0)],
        'camera_forward': list(forward)+[math.tan(content.get('vertical_fov', math.radians(52.0))*0.5)],
        'camera_right': list(right)+[content.get('window_fade', 0.0)], 'camera_up': list(up)+[content.get('daylight', 1.0)],
        'sun_direction': list(sun)+[content.get('water_level', 2.4 if scene else 1.2)], 'preview_origin': [0.0]*4,
        'preview_size': [0.0]*4, 'selection': content.get('selection', 0), 'quality': quality, 'sample_count': samples,
        'construction_grid': int(content.get('grid', False)),
        'vehicle_dimensions': [0.0]*4, 'vehicle_position': [0.0]*4,
        'vehicle_axis_x': [0.0]*4, 'vehicle_axis_y': [0.0]*4, 'vehicle_axis_z': [0.0]*4,
        'vehicle_motion': [0.0]*2,
    }
    if moving:
        pivot = np.array(vehicle.get('pivot', atlas['vehicle_min']*[1.0, 0.4, 1.0]), dtype=float)
        local_pivot = pivot-atlas['vehicle_min']*[1.0, 0.4, 1.0]
        yaw, pitch, roll = vehicle.get('yaw', 0.0), -vehicle.get('pitch', 0.0), vehicle.get('roll', 0.0)
        sy, cy, sp, cp, sr, cr = math.sin(yaw), math.cos(yaw), math.sin(pitch), math.cos(pitch), math.sin(roll), math.cos(roll)
        values.update({
            'vehicle_dimensions': [float(v) for v in atlas['vehicle_size']]+[float(atlas['vehicle_offset'])],
            'vehicle_position': list(vehicle.get('position', pivot))+[1.0],
            'vehicle_axis_x': [cy*cr+sy*sp*sr, cp*sr, -sy*cr+cy*sp*sr, local_pivot[0]],
            'vehicle_axis_y': [-cy*sr+sy*sp*cr, cp*cr, sy*sr+cy*sp*cr, local_pivot[1]],
            'vehicle_axis_z': [sy*cp, -sp, cy*cp, local_pivot[2]],
            'vehicle_motion': [vehicle.get('wheel_angle', 0.0), vehicle.get('rotor_angle', 0.0)],
        })
    uniforms = {uniform.name for uniform in effect.uniforms()}
    assert uniforms <= values.keys(), f'Unset shader uniforms: {uniforms-values.keys()}'
    for name in uniforms:
        builder.setUniform(name, values[name])
    for name, pixels in zip(('cells', 'parts'), pixel_images):
        image = skia.Image.fromarray(pixels, skia.ColorType.kRGBA_8888_ColorType, skia.AlphaType.kOpaque_AlphaType)
        matrix = skia.Matrix.Scale(width/pixels.shape[1], height/pixels.shape[0])
        builder.setChild(name, image.makeShader(skia.TileMode.kDecal, skia.TileMode.kDecal,
            skia.SamplingOptions(skia.FilterMode.kNearest), matrix))
    # EGL measures synchronized native Ganesh draws; the default remains CPU
    # raster. Neither backend measures browser application presentation.
    gpu = native_gl_backend() if backend == 'egl' else None
    surface = gpu.surface(width, height) if gpu else skia.Surface(width, height)
    paint = skia.Paint(Shader=builder.makeShader())
    draw_ms = []
    draw_cpu_ms = []
    for frame in range(frames):
        if animate_uniforms:
            builder.setUniform('camera_position', list(position)+[content.get('time', 4.0)+frame/60.0])
            paint = skia.Paint(Shader=builder.makeShader())
        started = time.perf_counter()
        cpu_started = time.process_time()
        surface.getCanvas().drawRect(skia.Rect.MakeWH(width, height), paint)
        if gpu:
            gpu.synchronize()
        draw_cpu_ms.append((time.process_time()-cpu_started)*1000)
        draw_ms.append((time.perf_counter()-started)*1000)
    snapshot = surface.makeImageSnapshot()
    if gpu:
        # Readback is deliberately outside the measured draw interval.
        snapshot = snapshot.makeRasterImage()
    assert snapshot is not None, 'Skia snapshot/readback failed'
    Path(output).write_bytes(bytes(snapshot.encodeToData()))
    rgb = snapshot.toarray(colorType=skia.ColorType.kRGBA_8888_ColorType)[..., :3]
    if validate:
        assert rgb.std() > 22, 'Rendered fixture is unexpectedly flat or blank'
    warm_pixels = None
    if validate and scene is None:
        # The fixed inspection fixture contains a red roof and warm-colored
        # bricks. Blue sky/water alone must not pass the visibility gate.
        red, green, blue = (rgb[..., channel].astype(float) for channel in range(3))
        warm_pixels = int(np.count_nonzero((red > green*1.15) & (red > blue*1.30)))
        assert warm_pixels >= max(3, width*height*0.0005), 'Fixture brick/roof geometry is missing'
    result = {
        'shader': 'compiled', 'backend': gpu.name if gpu else 'native Skia CPU raster',
        'context': gpu.context_info if gpu else None, 'uniform_updates': animate_uniforms,
        'parts': len(pieces)+len(moving),
        'output': str(output), 'channel_std': float(rgb.std()), 'warm_fixture_pixels': warm_pixels, 'size': [width, height],
        'quality': quality, 'samples': samples if 'sample_count' in uniforms else 1,
        'requested_samples': samples, 'variant': ('analytic-fast', 'analytic-balanced', 'full-geometry')[quality], 'compile_ms': compile_ms, 'draw_ms': draw_ms,
        'draw_cpu_ms': draw_cpu_ms,
        'steady_draw_ms_median': statistics.median(draw_ms[1:]) if frames > 1 else None,
        'steady_draw_cpu_ms_median': statistics.median(draw_cpu_ms[1:]) if frames > 1 else None,
        'texture_bytes': sum(pixels.nbytes for pixels in pixel_images),
    }
    return result, rgb


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', nargs='?', default='/tmp/briccraft-shader.png')
    parser.add_argument('scene', nargs='?')
    parser.add_argument('--width', type=int, default=800)
    parser.add_argument('--height', type=int, default=500)
    parser.add_argument('--quality', type=int, choices=(0, 1, 2), default=1)
    parser.add_argument('--samples', type=int, choices=(1, 2, 4), default=1)
    parser.add_argument('--frames', type=int, default=1)
    parser.add_argument('--backend', choices=('raster', 'egl'), default='raster',
                        help='Native CPU raster or synchronized Ganesh OpenGL (requires PyOpenGL/EGL)')
    parser.add_argument('--animate-uniforms', action='store_true',
                        help='Build a new shader snapshot with changing time uniforms each frame')
    parser.add_argument('--all-qualities', action='store_true', help='Compile and render all three actual production shader variants')
    parser.add_argument('--source', type=Path, help='Alternate concatenated SkSL, for a baseline benchmark')
    parser.add_argument('--compare-source', type=Path, help='Also render a baseline and report pixel differences')
    args = parser.parse_args()
    if min(args.width, args.height, args.frames) < 1:
        parser.error('width, height, and frames must be positive')
    results = []
    for quality in ((0, 1, 2) if args.all_qualities else (args.quality,)):
        output = Path(args.output)
        if args.all_qualities:
            output = output.with_name(f"{output.stem}.{('fast', 'balanced', 'full')[quality]}{output.suffix}")
        settings = dict(quality=quality, samples=args.samples, width=args.width, height=args.height, frames=args.frames,
                        backend=args.backend, animate_uniforms=args.animate_uniforms)
        result, rgb = render(output, args.scene, source=args.source.read_text() if args.source else None, **settings)
        if args.compare_source:
            baseline, original = render(str(output.with_suffix('.baseline.png')), args.scene,
                                        source=args.compare_source.read_text(), **settings)
            delta = np.abs(rgb.astype(np.int16)-original.astype(np.int16))
            result['baseline'] = baseline
            result['difference'] = {'mean_absolute_channel': float(delta.mean()), 'maximum_channel': int(delta.max()),
                                    'fraction_pixels_over_8': float((delta.max(axis=2) > 8).mean())}
        results.append(result)
    print(json.dumps(results if args.all_qualities else results[0]))
