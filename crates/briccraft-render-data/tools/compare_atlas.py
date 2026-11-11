"""Compare the current pure atlas packer with a pre-extraction Git revision.

Runs only parts/world/render-data Rust code, never FerroUI. Both builds use
release optimization with thin LTO, two warmups and twenty measured samples.
Example: python3 crates/briccraft-render-data/tools/compare_atlas.py --baseline-ref 2c14d12
"""
from pathlib import Path
import argparse
import json
import os
import re
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]


def run(command, **kwargs):
    result = subprocess.run(command, cwd=ROOT, check=True, capture_output=True, text=True, **kwargs)
    return result.stdout


def records(output):
    return [json.loads(line) for line in output.splitlines() if line.startswith('{')]


def from_existing_libraries(folder):
    dependencies = ROOT/'target/release/deps'
    externs = []
    for name in ('briccraft_parts', 'briccraft_world'):
        library = max(dependencies.glob(f'lib{name}-*.rlib'), key=lambda p: p.stat().st_mtime)
        externs.extend(['--extern', f'{name}={library}'])
    common = ['rustc', '--edition=2024', '-C', 'opt-level=3', '-C', 'panic=abort',
              '-C', 'embed-bitcode=yes', '-L', f'dependency={dependencies}']+externs
    library = folder/'libbriccraft_render_data.rlib'
    run(common+['--crate-type', 'rlib', '--crate-name', 'briccraft_render_data',
                str(ROOT/'crates/briccraft-render-data/src/lib.rs'), '-o', str(library)])
    run(common+['-C', 'lto=thin', '--extern', f'briccraft_render_data={library}',
                str(ROOT/'crates/briccraft-render-data/examples/benchmark.rs'), '-o', str(folder/'current')])
    run(common+['-C', 'lto=thin', str(folder/'src/main.rs'), '-o', str(folder/'baseline')])
    # Both compilers finish before either timed run begins.
    return records(run([str(folder/'current')])), records(run([str(folder/'baseline')]))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline-ref', required=True)
    parser.add_argument('--baseline-path', default='crates/briccraft-render/src/atlas.rs')
    parser.add_argument('--reuse-libraries', action='store_true',
                        help='Use existing release domain libraries, avoiding Cargo target locks')
    args = parser.parse_args()
    baseline_source = run(['git', 'show', f'{args.baseline_ref}:{args.baseline_path}'])
    if not args.reuse_libraries:
        current = records(run(['cargo', 'run', '--quiet', '--release', '-j', '1', '-p',
                               'briccraft-render-data', '--example', 'benchmark']))
    with tempfile.TemporaryDirectory(prefix='briccraft-atlas-baseline-') as temporary:
        folder = Path(temporary)
        (folder/'src').mkdir()
        (folder/'src/legacy.rs').write_text(baseline_source)
        program = (ROOT/'crates/briccraft-render-data/examples/benchmark.rs').read_text()
        program = program.replace('use briccraft_render_data::SceneAtlas;',
                                  '#[allow(dead_code)]\nmod legacy;\nuse legacy::SceneAtlas;')
        program, count = re.subn(r'SceneAtlas::build\(world,\s*&\[\],\s*None\)',
                                 'SceneAtlas::build(world, &[])', program)
        if count != 1:
            raise RuntimeError('Benchmark constructor changed; update the baseline adapter')
        (folder/'src/main.rs').write_text(program)
        (folder/'Cargo.toml').write_text(
            '[workspace]\n[package]\nname="briccraft-atlas-baseline"\nversion="0.0.0"\nedition="2024"\n'
            '[profile.release]\nlto="thin"\npanic="abort"\n[dependencies]\n'
            f'briccraft-parts={{path={json.dumps(str(ROOT/"crates/briccraft-parts"))}}}\n'
            f'briccraft-world={{path={json.dumps(str(ROOT/"crates/briccraft-world"))}}}\n')
        if args.reuse_libraries:
            current, baseline = from_existing_libraries(folder)
        else:
            environment = dict(os.environ, CARGO_TARGET_DIR=str(ROOT/'target'))
            baseline = records(run(['cargo', 'run', '--quiet', '--release', '-j', '1',
                                    '--manifest-path', str(folder/'Cargo.toml')], env=environment))
    comparison = []
    for before, after in zip(baseline, current):
        assert before['benchmark'] == after['benchmark']
        assert before['atlas_bytes'] == after['atlas_bytes']
        comparison.append({'benchmark': after['benchmark'],
                           'median_time_reduction_percent': 100*(1-after['median_ms']/before['median_ms']),
                           'median_cpu_work_reduction_percent':
                               100*(1-after['median_cpu_ms']/before['median_cpu_ms'])
                               if after['median_cpu_ms'] is not None and before['median_cpu_ms'] is not None else None})
    print(json.dumps({'baseline_ref': args.baseline_ref, 'current': current,
                      'baseline': baseline, 'comparison': comparison}, indent=2))


if __name__ == '__main__':
    main()
