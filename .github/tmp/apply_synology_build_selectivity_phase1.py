from pathlib import Path

ROOT = Path('.')
WORKFLOW = ROOT / '.github/workflows/build-synology-staging-images.yml'
PLATFORM_DOCKERFILE = ROOT / 'deploy/synology/docker/platform.Dockerfile'
CLASSIFIER = ROOT / 'scripts/ci/classify_synology_images.py'
TEST = ROOT / 'tests/ci/test_synology_image_build_routing.py'
TASK = ROOT / 'docs/agents/tasks/active/OTERYN-20260907-synology-build-selectivity.md'
DOCKERIGNORE = ROOT / '.dockerignore'


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'{label}: expected exactly one match, found {count}')
    return text.replace(old, new, 1)


classifier = r'''#!/usr/bin/env python3
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

SYNLOGY_CONTRACT_PATTERNS = (
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
    synology_contract = any(_matches(path, SYNLOGY_CONTRACT_PATTERNS) for path in normalized)
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
'''

# Typo guard: retain the intentionally public constant name expected by tests.
classifier = classifier.replace('SYNLOGY_CONTRACT_PATTERNS', 'SYNOLOGY_CONTRACT_PATTERNS')
CLASSIFIER.parent.mkdir(parents=True, exist_ok=True)
CLASSIFIER.write_text(classifier)


test = r'''from __future__ import annotations

import importlib.util
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
CLASSIFIER = ROOT / 'scripts/ci/classify_synology_images.py'
WORKFLOW = ROOT / '.github/workflows/build-synology-staging-images.yml'
PLATFORM_DOCKERFILE = ROOT / 'deploy/synology/docker/platform.Dockerfile'
DOCKERIGNORE = ROOT / '.dockerignore'

spec = importlib.util.spec_from_file_location('classify_synology_images', CLASSIFIER)
assert spec is not None and spec.loader is not None
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


def assert_plan(mode: str, paths: list[str], *, platform: bool, gateway: bool, runner: bool, deploy: bool) -> None:
    plan = module.plan_builds(mode, paths)
    assert plan['build_platform'] is platform, plan
    assert plan['build_gateway'] is gateway, plan
    assert plan['build_deploy_runner'] is runner, plan
    assert plan['deploy_main'] is deploy, plan


def test_pull_request_component_routing() -> None:
    assert_plan('pull_request', ['app/Http/Controllers/HomeController.php'], platform=True, gateway=False, runner=False, deploy=False)
    assert_plan('pull_request', ['lang/pl/portal.php'], platform=True, gateway=False, runner=False, deploy=False)
    assert_plan('pull_request', ['services/game-gateway/internal/login.go'], platform=False, gateway=True, runner=False, deploy=False)
    assert_plan('pull_request', ['deploy/synology/runner/entrypoint.sh'], platform=False, gateway=False, runner=True, deploy=False)
    assert_plan('pull_request', ['deploy/synology/scripts/health-check.sh'], platform=False, gateway=False, runner=False, deploy=False)
    assert_plan('pull_request', ['tests/ci/test_synology_auto_staging_deploy.py'], platform=False, gateway=False, runner=False, deploy=False)


def test_control_plane_fails_closed_on_pull_request() -> None:
    for path in (
        '.dockerignore',
        '.github/workflows/build-synology-staging-images.yml',
        'scripts/ci/classify_synology_images.py',
        'deploy/synology/scripts/repository-ghcr-image.sh',
    ):
        assert_plan('pull_request', [path], platform=True, gateway=True, runner=True, deploy=False)


def test_push_preserves_current_exact_sha_runtime_contract() -> None:
    assert_plan('push', ['app/Identity/Login.php'], platform=True, gateway=True, runner=False, deploy=True)
    assert_plan('push', ['services/game-gateway/main.go'], platform=True, gateway=True, runner=False, deploy=True)
    assert_plan('push', ['deploy/synology/scripts/health-check.sh'], platform=True, gateway=True, runner=False, deploy=True)
    assert_plan('push', ['deploy/synology/runner/entrypoint.sh'], platform=False, gateway=False, runner=False, deploy=False)


def test_manual_dispatch_remains_explicit_full_build_without_auto_deploy() -> None:
    plan = module.plan_builds('workflow_dispatch', [], force_all=True)
    assert plan['build_platform'] is True
    assert plan['build_gateway'] is True
    assert plan['build_deploy_runner'] is True
    assert plan['deploy_main'] is False


def test_platform_dockerfile_copies_only_runtime_inputs() -> None:
    dockerfile = PLATFORM_DOCKERFILE.read_text()
    assert 'COPY . .' not in dockerfile
    for required in (
        'COPY app/ ./app/',
        'COPY bootstrap/ ./bootstrap/',
        'COPY config/ ./config/',
        'COPY database/ ./database/',
        'COPY lang/ ./lang/',
        'COPY public/ ./public/',
        'COPY resources/ ./resources/',
        'COPY routes/ ./routes/',
        'COPY storage/ ./storage/',
        'COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json ./docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json',
    ):
        assert required in dockerfile


def test_root_dockerignore_bounds_shared_context() -> None:
    ignore = DOCKERIGNORE.read_text()
    assert ignore.startswith('**\n')
    for required in (
        '!app/**',
        '!lang/**',
        '!services/game-gateway/**',
        '!deploy/synology/docker/**',
        '!deploy/synology/runner/**',
        '!docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json',
    ):
        assert required in ignore
    for forbidden in ('!tests/**', '!docs/agents/**', '!.github/**'):
        assert forbidden not in ignore


def test_workflow_uses_classifier_and_skips_unaffected_build_steps() -> None:
    workflow = WORKFLOW.read_text()
    assert 'Classify Synology image changes' in workflow
    assert 'python3 scripts/ci/classify_synology_images.py' in workflow
    assert "if: steps.selection.outputs.enabled == 'true'" in workflow
    assert 'needs.classify-images.outputs.deploy_main' in workflow
    assert '- lang/**' in workflow
    assert '- artisan' in workflow
    assert '- .dockerignore' in workflow
    assert '- scripts/ci/classify_synology_images.py' in workflow


def main() -> None:
    tests = [value for name, value in sorted(globals().items()) if name.startswith('test_') and callable(value)]
    for test in tests:
        test()
    print(f'synology image build routing contract: PASS ({len(tests)} tests)')


if __name__ == '__main__':
    main()
'''
TEST.write_text(test)


dockerignore = '''**
!composer.json
!composer.lock
!artisan
!app/
!app/**
!bootstrap/
!bootstrap/**
!config/
!config/**
!database/
!database/**
!lang/
!lang/**
!public/
!public/**
!resources/
!resources/**
!routes/
!routes/**
!storage/
!storage/**
!docs/
!docs/testing/
!docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
!services/
!services/game-gateway/
!services/game-gateway/**
!deploy/
!deploy/synology/
!deploy/synology/docker/
!deploy/synology/docker/**
!deploy/synology/runner/
!deploy/synology/runner/**
'''
DOCKERIGNORE.write_text(dockerignore)


dockerfile = PLATFORM_DOCKERFILE.read_text()
dockerfile = replace_once(
    dockerfile,
    'COPY . .\n',
    '''COPY artisan ./artisan
COPY app/ ./app/
COPY bootstrap/ ./bootstrap/
COPY config/ ./config/
COPY database/ ./database/
COPY lang/ ./lang/
COPY public/ ./public/
COPY resources/ ./resources/
COPY routes/ ./routes/
COPY storage/ ./storage/
COPY docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json ./docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
''',
    'platform repository-wide copy',
)
PLATFORM_DOCKERFILE.write_text(dockerfile)


workflow = WORKFLOW.read_text()
path_anchor = '      - composer.lock\n      - app/**\n'
path_insert = '''      - composer.lock
      - artisan
      - lang/**
      - storage/**
      - docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json
      - .dockerignore
      - scripts/ci/classify_synology_images.py
      - app/**
'''
if workflow.count(path_anchor) != 2:
    raise SystemExit(f'workflow path anchor: expected 2 matches, found {workflow.count(path_anchor)}')
workflow = workflow.replace(path_anchor, path_insert, 2)
workflow = replace_once(
    workflow,
    '      - tests/ci/test_synology_auto_staging_deploy.py\n      - .github/workflows/build-synology-staging-images.yml\n',
    '      - tests/ci/test_synology_auto_staging_deploy.py\n      - tests/ci/test_synology_image_build_routing.py\n      - .github/workflows/build-synology-staging-images.yml\n',
    'PR routing test trigger',
)

classify_job = r'''jobs:
  classify-images:
    name: Classify Synology image changes
    runs-on: ubuntu-latest
    outputs:
      build_platform: ${{ steps.classify.outputs.build_platform }}
      build_gateway: ${{ steps.classify.outputs.build_gateway }}
      build_deploy_runner: ${{ steps.classify.outputs.build_deploy_runner }}
      deploy_main: ${{ steps.classify.outputs.deploy_main }}
    steps:
      - name: Checkout exact classification range
        uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0

      - name: Classify affected image inputs
        id: classify
        env:
          EVENT_NAME: ${{ github.event_name }}
          PR_BASE_SHA: ${{ github.event.pull_request.base.sha }}
          PR_HEAD_SHA: ${{ github.event.pull_request.head.sha }}
          PUSH_BASE_SHA: ${{ github.event.before }}
        run: |
          set -euo pipefail
          common=(
            --mode "$EVENT_NAME"
            --github-output "$GITHUB_OUTPUT"
            --summary "$GITHUB_STEP_SUMMARY"
          )
          case "$EVENT_NAME" in
            workflow_dispatch)
              python3 scripts/ci/classify_synology_images.py "${common[@]}" --all
              ;;
            pull_request)
              python3 scripts/ci/classify_synology_images.py \
                "${common[@]}" \
                --base "$PR_BASE_SHA" \
                --head "$PR_HEAD_SHA"
              ;;
            push)
              if [[ ! "$PUSH_BASE_SHA" =~ ^[0-9a-f]{40}$ || "$PUSH_BASE_SHA" =~ ^0{40}$ ]]; then
                python3 scripts/ci/classify_synology_images.py "${common[@]}" --all
              else
                python3 scripts/ci/classify_synology_images.py \
                  "${common[@]}" \
                  --base "$PUSH_BASE_SHA" \
                  --head "$GITHUB_SHA"
              fi
              ;;
            *)
              echo "Unsupported Synology image routing event: $EVENT_NAME" >&2
              exit 1
              ;;
          esac

  validate-deployment:
'''
workflow = replace_once(workflow, 'jobs:\n  validate-deployment:\n', classify_job, 'classifier job insertion')

build_start = workflow.index('\n  build:\n')
deploy_start = workflow.index('\n  deploy-staging:\n', build_start)
build = workflow[build_start:deploy_start]
build = replace_once(
    build,
    '  build:\n    name: Build ${{ matrix.name }} image\n    runs-on: ubuntu-latest\n',
    '  build:\n    name: Build ${{ matrix.name }} image\n    needs: classify-images\n    runs-on: ubuntu-latest\n',
    'build needs classifier',
)
selection = r'''    steps:
      - name: Decide whether image is affected
        id: selection
        env:
          IMAGE_NAME: ${{ matrix.name }}
          BUILD_PLATFORM: ${{ needs.classify-images.outputs.build_platform }}
          BUILD_GATEWAY: ${{ needs.classify-images.outputs.build_gateway }}
          BUILD_DEPLOY_RUNNER: ${{ needs.classify-images.outputs.build_deploy_runner }}
        run: |
          set -euo pipefail
          case "$IMAGE_NAME" in
            platform) enabled="$BUILD_PLATFORM" ;;
            game-gateway) enabled="$BUILD_GATEWAY" ;;
            deploy-runner) enabled="$BUILD_DEPLOY_RUNNER" ;;
            *) echo "Unknown image matrix entry: $IMAGE_NAME" >&2; exit 1 ;;
          esac
          echo "enabled=$enabled" >> "$GITHUB_OUTPUT"
          if [[ "$enabled" != true ]]; then
            echo "Skipping unaffected $IMAGE_NAME image build."
          fi

      - name: Checkout exact source
'''
build = replace_once(build, '    steps:\n      - name: Checkout exact source\n', selection, 'build selection step')
build = replace_once(
    build,
    '      - name: Checkout exact source\n        uses:',
    "      - name: Checkout exact source\n        if: steps.selection.outputs.enabled == 'true'\n        uses:",
    'build checkout condition',
)
build = replace_once(
    build,
    '      - name: Set up Docker Buildx\n        uses:',
    "      - name: Set up Docker Buildx\n        if: steps.selection.outputs.enabled == 'true'\n        uses:",
    'buildx condition',
)
build = replace_once(
    build,
    '      - name: Resolve current repository GHCR image\n        id:',
    "      - name: Resolve current repository GHCR image\n        if: steps.selection.outputs.enabled == 'true'\n        id:",
    'image resolution condition',
)
build = replace_once(
    build,
    '      - name: Compute image tags\n        id:',
    "      - name: Compute image tags\n        if: steps.selection.outputs.enabled == 'true'\n        id:",
    'metadata condition',
)
build = replace_once(
    build,
    "      - name: Log in to GHCR\n        if: github.event_name != 'pull_request' && (github.event_name == 'workflow_dispatch' || matrix.name != 'deploy-runner')\n",
    "      - name: Log in to GHCR\n        if: steps.selection.outputs.enabled == 'true' && github.event_name != 'pull_request' && (github.event_name == 'workflow_dispatch' || matrix.name != 'deploy-runner')\n",
    'login condition',
)
build = replace_once(
    build,
    '      - name: Build and optionally publish\n        uses:',
    "      - name: Build and optionally publish\n        if: steps.selection.outputs.enabled == 'true'\n        uses:",
    'build action condition',
)
build = replace_once(
    build,
    "      - name: Validate locally loaded pull-request image\n        if: github.event_name == 'pull_request'\n",
    "      - name: Validate locally loaded pull-request image\n        if: steps.selection.outputs.enabled == 'true' && github.event_name == 'pull_request'\n",
    'local image validation condition',
)
workflow = workflow[:build_start] + build + workflow[deploy_start:]
workflow = replace_once(
    workflow,
    "    if: github.event_name == 'push' && github.ref == 'refs/heads/main'\n    needs:\n      - validate-deployment\n      - build\n",
    "    if: github.event_name == 'push' && github.ref == 'refs/heads/main' && needs.classify-images.outputs.deploy_main == 'true'\n    needs:\n      - classify-images\n      - validate-deployment\n      - build\n",
    'deploy routing condition',
)
WORKFLOW.write_text(workflow)


task = '''---
task_id: OTERYN-20260907-synology-build-selectivity
governing_issue: 1328
terminal_pr_policy: archive_pending
required_reads:
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
search_first:
  - active Synology deployment tasks and open PR ownership
  - current Synology image build and release identity contracts
optional_reads: []
---

# OTERYN-20260907-synology-build-selectivity

## Goal

Make Synology image verification proportional to actual image inputs without weakening protected-main release identity or deployment/recovery safety.

## Phase 1 acceptance

- Platform image no longer copies unrelated repository control/test/docs sources.
- Pull requests build only affected Platform, Gateway and deploy-runner images; deployment-package-only PRs retain package/Compose contract validation without unrelated image builds.
- Build-routing control-plane changes fail closed to all three PR images.
- Ordinary protected-main pushes do not spend a build on the non-published deploy-runner.
- Runner-only protected-main changes are not misrepresented as a new Platform/Gateway runtime release.
- Until Phase 2, any protected-main Platform/Gateway or non-runner Synology deployment-package change continues to build both exact-SHA runtime images and dispatch the guarded staging deploy.
- Full repository exact-head CI and Merge Queue remain mandatory.

## Phase 2 hold

Per-component main reuse is not authorized by Phase 1. It requires explicit persisted component source SHA + immutable digest provenance through deploy, candidate, last-good, rollback and recovery contracts before either Platform or Gateway may be reused across an overall protected-main SHA.

## Ownership

```yaml
owned_paths:
  - .dockerignore
  - .github/workflows/build-synology-staging-images.yml
  - deploy/synology/docker/platform.Dockerfile
  - scripts/ci/classify_synology_images.py
  - tests/ci/test_synology_image_build_routing.py
modules:
  - Synology image build routing
  - Platform Docker build context
  - CI economy
conflicts: []
```

## Checkpoint

```yaml
checkpoint_version: 1
base: c15493a1c4c38762b486893a56b40ee6004f0381
branch: ci/1328-synology-build-selectivity-phase1
status: implementing
proven:
  - The fixed Synology image matrix currently starts Platform, Gateway and deploy-runner builds for deployment-script-only PRs.
  - Platform Dockerfile currently uses repository-wide COPY . . and there is no root .dockerignore, so unrelated repository paths contaminate Platform image cache/content identity.
  - Current protected-main deploy contract requires both Platform and Gateway OCI revisions to equal one release SHA; Phase 1 therefore cannot safely reuse either runtime image on main.
  - Ordinary main pushes do not publish deploy-runner, but the existing matrix still builds it.
derived:
  - PR image selection can be made component-aware immediately once Docker inputs are truthful.
  - Main Platform/Gateway selectivity requires Phase 2 component provenance rather than a simple conditional skip.
unknown:
  - Post-merge live runner-time reduction until representative PR/main evidence exists.
next_action: Complete Phase 1 exact-head validation and protected integration, then gather routing evidence before Phase 2.
```
'''
TASK.parent.mkdir(parents=True, exist_ok=True)
TASK.write_text(task)
