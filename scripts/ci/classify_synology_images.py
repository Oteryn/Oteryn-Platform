#!/usr/bin/env python3
from __future__ import annotations

import argparse
import fnmatch
import json
import subprocess
from pathlib import Path
from typing import Iterable

PLATFORM_PATTERNS = (
    'composer.json',
    'composer.lock',
    'artisan',
    'app/**',
    'bootstrap/**',
    'config/**',
    'database/**',
    'lang/**',
    'public/**',
    'resources/**',
    'routes/**',
    'storage/**',
    'docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json',
    'deploy/synology/docker/platform.Dockerfile',
    'deploy/synology/docker/platform-entrypoint.sh',
    'deploy/synology/docker/platform-media.ini',
)

GATEWAY_PATTERNS = (
    'services/game-gateway/**',
    'deploy/synology/docker/gateway.Dockerfile',
)

DEPLOY_RUNNER_PATTERNS = (
    'deploy/synology/runner/**',
)

CONTROL_PLANE_PATTERNS = (
    '.dockerignore',
    '.github/workflows/build-synology-staging-images.yml',
    'scripts/ci/classify_synology_images.py',
    'deploy/synology/scripts/repository-ghcr-image.sh',
)

SYNOLOGY_CONTRACT_PATTERNS = (
    '.github/workflows/deploy-synology-staging.yml',
    'tests/ci/test_synology_rollback_contract.py',
    'tests/ci/test_synology_deploy_release_identity.py',
    'tests/ci/test_synology_auto_staging_deploy.py',
    'tests/ci/test_synology_rollback_recovery_contract.py',
    'tests/ci/test_synology_image_build_routing.py',
)


def _matches(path: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns)


def _normalize(paths: Iterable[str]) -> list[str]:
    return sorted({
        path.strip().replace('\\', '/').removeprefix('./')
        for path in paths
        if path.strip()
    })


def classify_paths(paths: Iterable[str]) -> dict[str, object]:
    normalized = _normalize(paths)
    control = any(_matches(path, CONTROL_PLANE_PATTERNS) for path in normalized)
    platform = control or any(_matches(path, PLATFORM_PATTERNS) for path in normalized)
    gateway = control or any(_matches(path, GATEWAY_PATTERNS) for path in normalized)
    deploy_runner = control or any(_matches(path, DEPLOY_RUNNER_PATTERNS) for path in normalized)

    non_runner_synology = any(
        path.startswith('deploy/synology/')
        and not _matches(path, DEPLOY_RUNNER_PATTERNS)
        for path in normalized
    )
    synology_contract = any(_matches(path, SYNOLOGY_CONTRACT_PATTERNS) for path in normalized)
    deployment_package = control or non_runner_synology or synology_contract

    return {
        'paths': normalized,
        'platform': platform,
        'gateway': gateway,
        'deploy_runner': deploy_runner,
        'deployment_package': deployment_package,
        'control_plane': control,
    }


def plan_builds(mode: str, paths: Iterable[str], *, force_all: bool = False) -> dict[str, object]:
    if force_all:
        classification = {
            'paths': _normalize(paths),
            'platform': True,
            'gateway': True,
            'deploy_runner': True,
            'deployment_package': True,
            'control_plane': True,
        }
    else:
        classification = classify_paths(paths)

    platform = bool(classification['platform'])
    gateway = bool(classification['gateway'])
    deploy_runner = bool(classification['deploy_runner'])
    deployment_package = bool(classification['deployment_package'])
    control = bool(classification['control_plane'])

    if mode == 'workflow_dispatch':
        build_platform = True
        build_gateway = True
        build_deploy_runner = True
        deploy_main = False
    elif mode == 'pull_request':
        build_platform = platform
        build_gateway = gateway
        build_deploy_runner = deploy_runner
        deploy_main = False
    elif mode == 'push':
        # The current protected-main release contract still requires Platform and
        # Gateway OCI revisions to match one overall release SHA. Until Phase 2
        # introduces per-component immutable provenance, any runtime or non-runner
        # deployment-package change therefore rebuilds both runtime images.
        main_release = platform or gateway or deployment_package or control
        build_platform = main_release
        build_gateway = main_release
        # Ordinary main pushes never publish the privileged runner. A runner-only
        # change is validation/infrastructure work, not a new staging runtime release.
        build_deploy_runner = False
        deploy_main = main_release
    else:
        raise ValueError(f'unsupported mode: {mode}')

    return {
        **classification,
        'mode': mode,
        'build_platform': build_platform,
        'build_gateway': build_gateway,
        'build_deploy_runner': build_deploy_runner,
        'deploy_main': deploy_main,
    }


def changed_paths(base: str, head: str) -> list[str]:
    if not base or not head:
        raise ValueError('both base and head are required')
    output = subprocess.check_output(
        ['git', 'diff', '--name-only', '--diff-filter=ACMRD', base, head],
        text=True,
    )
    return [line for line in output.splitlines() if line.strip()]


def write_github_output(path: Path, plan: dict[str, object]) -> None:
    keys = (
        'build_platform',
        'build_gateway',
        'build_deploy_runner',
        'deploy_main',
        'platform',
        'gateway',
        'deploy_runner',
        'deployment_package',
        'control_plane',
    )
    with path.open('a', encoding='utf-8') as handle:
        for key in keys:
            handle.write(f"{key}={'true' if plan[key] else 'false'}\n")
        handle.write(f"paths_json={json.dumps(plan['paths'], separators=(',', ':'))}\n")


def write_summary(path: Path, plan: dict[str, object]) -> None:
    rows = [
        ('Platform image', 'build_platform'),
        ('Gateway image', 'build_gateway'),
        ('Deploy-runner image', 'build_deploy_runner'),
        ('Dispatch main staging', 'deploy_main'),
    ]
    lines = [
        '### Synology image build routing',
        '',
        f"- mode: `{plan['mode']}`",
        f"- changed paths: `{len(plan['paths'])}`",
        f"- deployment package affected: `{'YES' if plan['deployment_package'] else 'NO'}`",
        f"- build control plane affected: `{'YES' if plan['control_plane'] else 'NO'}`",
        '',
        '| Action | Decision |',
        '|---|---|',
    ]
    for label, key in rows:
        lines.append(f"| {label} | `{'RUN' if plan[key] else 'SKIP'}` |")
    lines.append('')
    with path.open('a', encoding='utf-8') as handle:
        handle.write('\n'.join(lines))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description='Classify Synology image build inputs.')
    parser.add_argument('--mode', required=True, choices=('pull_request', 'push', 'workflow_dispatch'))
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--all', action='store_true')
    source.add_argument('--paths', nargs='*')
    source.add_argument('--base')
    parser.add_argument('--head')
    parser.add_argument('--github-output', type=Path)
    parser.add_argument('--summary', type=Path)
    parser.add_argument('--json', action='store_true')
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.all:
        paths: list[str] = []
        force_all = True
    elif args.paths is not None:
        paths = list(args.paths)
        force_all = False
    else:
        paths = changed_paths(args.base, args.head)
        force_all = False

    plan = plan_builds(args.mode, paths, force_all=force_all)
    if args.github_output:
        write_github_output(args.github_output, plan)
    if args.summary:
        write_summary(args.summary, plan)
    if args.json or (not args.github_output and not args.summary):
        print(json.dumps(plan, indent=2, sort_keys=True))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
