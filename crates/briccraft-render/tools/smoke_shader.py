"""Compile and render the production SkSL in native Skia (pip install skia-python).

This is a graphics regression fixture, not a second game renderer. The shader,
packing format, camera, and material path are the ones used by the browser app.
Run: python crates/briccraft-render/tools/smoke_shader.py /tmp/briccraft-shader.png
Optional second argument: a scene JSON written by the export_atlas example.
"""
from pathlib import Path
import json
import math
import sys
import skia
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "\n".join((ROOT / "src/shaders" / f"{name}.sksl").read_text()
                    for name in ("uniforms", "geometry", "trace", "lighting", "main"))


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


def images(pieces, dimensions):
    nx, ny, nz = dimensions
    occupancy = np.zeros((ny, nz, nx), dtype=np.uint32)
    for i, (origin, size, *_rest) in enumerate(pieces, 1):
        x, y, z = origin
        sx, sy, sz = size
        occupancy[y:y+sy, z:z+sz, x:x+sx] = i
    coarse_size = [math.ceil(nx/4), math.ceil((ny+1)/6), math.ceil(nz/4)]
    coarse = np.zeros((coarse_size[1],coarse_size[2],coarse_size[0]),dtype=np.uint32)
    ys,zs,xs = np.where(occupancy>0)
    coarse[ys//6,zs//4,xs//4] = 1
    coarse[(ys+1)//6,zs//4,xs//4] = 1
    ids = np.concatenate((occupancy.ravel(),coarse.ravel()))
    pixels = np.zeros((math.ceil(len(ids) / 1024), 1024, 4), dtype=np.uint8)
    pixels[..., 3] = 255
    flat = pixels.reshape(-1, 4)
    flat[:len(ids), 0] = ids & 255
    flat[:len(ids), 1] = (ids >> 8) & 255
    flat[:len(ids), 2] = (ids >> 16) & 255
    records = np.zeros((math.ceil((len(pieces) + 1) * 4 / 1024), 1024, 4), dtype=np.uint8)
    records[..., 3] = 255
    data = records.reshape(-1, 4)
    for i, (origin, size, color, shape, rotation) in enumerate(pieces, 1):
        data[i*4, :3] = origin
        data[i*4+1, :3] = size
        data[i*4+2, :3] = color
        data[i*4+3, :3] = [shape, rotation, 0]
    return pixels, records


def render(output, scene=None):
    width, height = 800, 500
    dimensions = [96, 72, 96]
    pieces = fixture()
    position = np.array([75.0, 30.0, 87.0])
    target = np.array([46.0, 5.0, 45.0])
    if scene is not None:
        content = json.loads(Path(scene).read_text())
        dimensions = content['dimensions']
        pieces = content['pieces']
        if 'position' in content:
            position = np.array(content['position'], dtype=float)
            target = np.array(content['target'], dtype=float)
    pixel_images = images(pieces, dimensions)
    effect = skia.RuntimeEffect.MakeForShader(SOURCE)
    assert effect, "Production SkSL failed to compile"
    builder = skia.RuntimeShaderBuilder(effect)
    forward = target - position
    forward /= np.linalg.norm(forward)
    right = np.array([forward[2], 0.0, -forward[0]])
    right /= np.linalg.norm(right)
    up = np.cross(forward, right)
    sun = np.array([-0.45, 0.83, -0.32])
    sun /= np.linalg.norm(sun)
    values = {
        'layer_size': [width, height], 'layer_scale': 1.0,
        'world_dimensions': dimensions + [float(math.prod(dimensions))],
        'cell_texture_size': [pixel_images[0].shape[1], pixel_images[0].shape[0]],
        'part_texture_size': [pixel_images[1].shape[1], pixel_images[1].shape[0]],
        'camera_position': list(position) + [4.0],
        'camera_forward': list(forward) + [math.tan(math.radians(26.0))],
        'camera_right': list(right) + [0.0], 'camera_up': list(up) + [1.0],
        'sun_direction': list(sun) + [1.2], 'preview_origin': [0.0]*4,
        'preview_size': [0.0]*4, 'selection': 0, 'quality': 1, 'construction_grid': 0,
        'vehicle_dimensions': [0.0]*4, 'vehicle_position': [0.0]*4,
        'vehicle_axis_x': [0.0]*4, 'vehicle_axis_y': [0.0]*4, 'vehicle_axis_z': [0.0]*4,
        'vehicle_motion': [0.0]*2,
    }
    if scene is not None:
        values['sun_direction'][3] = 2.4
    for name, value in values.items():
        builder.setUniform(name, value)
    for name, pixels in zip(('cells', 'parts'), pixel_images):
        image = skia.Image.fromarray(pixels, skia.ColorType.kRGBA_8888_ColorType, skia.AlphaType.kOpaque_AlphaType)
        matrix = skia.Matrix.Scale(width / pixels.shape[1], height / pixels.shape[0])
        builder.setChild(name, image.makeShader(skia.TileMode.kDecal, skia.TileMode.kDecal,
            skia.SamplingOptions(skia.FilterMode.kNearest), matrix))
    surface = skia.Surface(width, height)
    surface.getCanvas().drawRect(skia.Rect.MakeWH(width, height), skia.Paint(Shader=builder.makeShader()))
    snapshot = surface.makeImageSnapshot()
    Path(output).write_bytes(bytes(snapshot.encodeToData()))
    rgb = snapshot.toarray()[..., :3]
    assert rgb.std() > 22, 'Rendered fixture is unexpectedly flat or blank'
    print(json.dumps({'shader': 'compiled', 'parts': len(pieces), 'output': str(output),
                      'channel_std': float(rgb.std()), 'size': [width, height]}))


if __name__ == '__main__':
    render(sys.argv[1] if len(sys.argv) > 1 else '/tmp/briccraft-shader.png',
           sys.argv[2] if len(sys.argv) > 2 else None)
