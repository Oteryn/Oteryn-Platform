#!/usr/bin/env python3
"""Compile design tests with the pinned workspace dependency, without changing its layout."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--update', action='store_true', help='regenerate the static gallery from partials and local i18n catalogues')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    result = subprocess.run(
        ['cargo', 'build', '--locked', '-p', 'oteryn-web', '--message-format=json'],
        cwd=root, text=True, stdout=subprocess.PIPE, check=True,
    )
    libraries = []
    for line in result.stdout.splitlines():
        artifact = json.loads(line)
        if artifact.get('reason') == 'compiler-artifact' and artifact.get('target', {}).get('name') == 'askama':
            libraries.extend(Path(name) for name in artifact['filenames'] if name.endswith('.rlib'))
    if len(libraries) != 1:
        raise SystemExit('Expected one Askama library from the locked workspace build')
    library = libraries[0]
    environment = dict(os.environ, CARGO_MANIFEST_DIR=str(root / 'crates/web'))
    if args.update:
        environment['UPDATE_DESIGN_GALLERY'] = '1'
    with tempfile.TemporaryDirectory(prefix='portal-design-') as temporary:
        binary = str(Path(temporary) / 'design-tests')
        arguments = [
            '--edition=2024', '--test', str(root / 'static/design/tests.rs'),
            '--extern', f'askama={library}', '-L', f'dependency={library.parent}', '-o', binary,
        ]
        subprocess.run(['rustc', *arguments, '-D', 'warnings'], cwd=root, env=environment, check=True)
        subprocess.run([binary, '--nocapture'], cwd=root, env=environment, check=True)
        driver = subprocess.check_output(['rustup', 'which', 'clippy-driver'], cwd=root, text=True).strip()
        subprocess.run([driver, *arguments, '-D', 'warnings', '-D', 'clippy::unwrap_used', '-D', 'clippy::expect_used', '-D', 'clippy::panic'], cwd=root, env=environment, check=True)
    size = (root / 'static/design/drawer.js').stat().st_size
    if size >= 5000:
        raise SystemExit(f'Drawer script exceeds 5 KB: {size} bytes')
    print(f'Design checks passed; drawer script: {size} bytes. Database tests: 0 (presentation only).')


if __name__ == '__main__':
    main()
