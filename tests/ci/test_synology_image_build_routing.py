from __future__ import annotations

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
