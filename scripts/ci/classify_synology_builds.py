#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Iterable

WIKI_RUNTIME_PROVENANCE = (
    "docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json",
    "docs/architecture/adr/0004-authoritative-platform-account-ownership.md",
    "docs/architecture/adr/0005-character-creation-product-policy.md",
    "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md",
    "docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md",
    "docs/agents/PROJECT_STATE.md",
    "docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md",
    "docs/architecture/SECURITY_ARCHITECTURE.md",
    "docs/architecture/adr/0013-wiki-administration.md",
)

PLATFORM_DOCKERIGNORE = "deploy/synology/docker/platform.Dockerfile.dockerignore"
GATEWAY_DOCKERIGNORE = "deploy/synology/docker/gateway.Dockerfile.dockerignore"
RUNNER_DOCKERIGNORE = "deploy/synology/runner/Dockerfile.dockerignore"

PLATFORM_EXACT = {
    "composer.json",
    "composer.lock",
    "artisan",
    "deploy/synology/docker/platform.Dockerfile",
    PLATFORM_DOCKERIGNORE,
    "deploy/synology/docker/platform-media.ini",
    "deploy/synology/docker/platform-entrypoint.sh",
    "deploy/synology/release-contract.env",
    *WIKI_RUNTIME_PROVENANCE,
}
PLATFORM_PREFIXES = (
    "app/",
    "bootstrap/",
    "config/",
    "database/",
    "public/",
    "resources/",
    "routes/",
    "lang/",
    "storage/",
)

GATEWAY_EXACT = {
    "deploy/synology/docker/gateway.Dockerfile",
    GATEWAY_DOCKERIGNORE,
    "services/game-gateway/go.mod",
}
GATEWAY_PREFIXES = (
    "services/game-gateway/cmd/",
    "services/game-gateway/internal/",
)

# The deploy-runner image only copies these files. Runner Compose/environment
# files are operator-package inputs, not deploy-runner image inputs.
RUNNER_EXACT = {
    "deploy/synology/runner/Dockerfile",
    RUNNER_DOCKERIGNORE,
    "deploy/synology/runner/entrypoint.sh",
    "deploy/synology/runner/test-entrypoint.sh",
}

# If the decision machinery or a Docker build-context boundary changes, prove
# every relevant image path rather than allowing the classifier to skip itself.
CONTROL_PLANE_EXACT = {
    ".github/workflows/build-synology-staging-images.yml",
    ".github/workflows/deploy-synology-staging.yml",
    "scripts/ci/classify_synology_builds.py",
    "deploy/synology/scripts/repository-ghcr-image.sh",
    PLATFORM_DOCKERIGNORE,
    GATEWAY_DOCKERIGNORE,
    RUNNER_DOCKERIGNORE,
}

DEPLOYMENT_ROOT = "deploy/synology/"
RUNNER_ROOT = "deploy/synology/runner/"


def _matches(path: str, exact: set[str], prefixes: Iterable[str] = ()) -> bool:
    return path in exact or any(path.startswith(prefix) for prefix in prefixes)


def _platform_input(path: str) -> bool:
    return _matches(path, PLATFORM_EXACT, PLATFORM_PREFIXES)


def _gateway_input(path: str) -> bool:
    return _matches(path, GATEWAY_EXACT, GATEWAY_PREFIXES)


def _runner_input(path: str) -> bool:
    return path in RUNNER_EXACT


def _deployment_package_input(path: str) -> bool:
    if not path.startswith(DEPLOYMENT_ROOT):
        return False
    return not (_platform_input(path) or _gateway_input(path) or _runner_input(path))


def _runtime_deployment_package_input(path: str) -> bool:
    # Runner operator configuration is validated as deployment-package state but
    # does not reconcile the staging runtime by itself.
    return _deployment_package_input(path) and not path.startswith(RUNNER_ROOT)


def _changed_paths(base: str, head: str) -> list[str]:
    if not head:
        raise SystemExit("--head is required outside workflow_dispatch")
    if not base or set(base) == {"0"}:
        command = ["git", "ls-tree", "-r", "--name-only", head]
    else:
        command = [
            "git",
            "diff",
            "--name-only",
            "--diff-filter=ACMRTUXB",
            base,
            head,
        ]
    result = subprocess.run(command, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or f"failed to classify Git range {base}..{head}")
    return sorted({line.strip() for line in result.stdout.splitlines() if line.strip()})


def _require_sha(head: str, *, event_name: str) -> str:
    if event_name == "workflow_dispatch" and not head:
        return ""
    if len(head) != 40 or any(ch not in "0123456789abcdef" for ch in head):
        raise SystemExit("--head must be an exact lower-case 40-character Git SHA")
    return head


def classify(paths: Iterable[str], event_name: str, head: str) -> dict[str, object]:
    if event_name not in {"pull_request", "merge_group", "push", "workflow_dispatch"}:
        raise ValueError(f"unsupported event: {event_name}")

    changed = sorted(set(paths))
    exact_head = _require_sha(head, event_name=event_name)

    control_plane_changed = any(path in CONTROL_PLANE_EXACT for path in changed)
    platform_changed = any(_platform_input(path) for path in changed)
    gateway_changed = any(_gateway_input(path) for path in changed)
    deploy_runner_changed = any(_runner_input(path) for path in changed)
    deployment_package_changed = any(_deployment_package_input(path) for path in changed)
    runtime_deployment_changed = any(_runtime_deployment_package_input(path) for path in changed)

    if event_name == "workflow_dispatch":
        build_platform = True
        build_gateway = True
        build_deploy_runner = True
        release_relevant = False
    elif event_name in {"pull_request", "merge_group"}:
        build_platform = control_plane_changed or platform_changed
        build_gateway = control_plane_changed or gateway_changed
        build_deploy_runner = control_plane_changed or deploy_runner_changed
        release_relevant = False
    else:  # protected-main push
        build_platform = control_plane_changed or platform_changed
        build_gateway = control_plane_changed or gateway_changed
        # Ordinary main is never allowed to publish the privileged deploy-runner.
        build_deploy_runner = False
        release_relevant = (
            control_plane_changed
            or platform_changed
            or gateway_changed
            or runtime_deployment_changed
        )

    components = (
        (
            "platform",
            build_platform,
            "oteryn-platform",
            "deploy/synology/docker/platform.Dockerfile",
        ),
        (
            "game-gateway",
            build_gateway,
            "oteryn-game-gateway",
            "deploy/synology/docker/gateway.Dockerfile",
        ),
        (
            "deploy-runner",
            build_deploy_runner,
            "oteryn-deploy-runner",
            "deploy/synology/runner/Dockerfile",
        ),
    )

    include = [
        {
            "name": name,
            "package": package,
            "dockerfile": dockerfile,
            "mode": "full",
            "source_sha": exact_head,
        }
        for name, enabled, package, dockerfile in components
        if enabled
    ]
    has_image_builds = bool(include)
    if not include:
        # Keep a syntactically non-empty matrix object. The workflow gates the
        # entire build job on has_image_builds, so this placeholder never gets a
        # runner and therefore is not a hidden no-op image job.
        include = [
            {
                "name": "noop",
                "package": "noop",
                "dockerfile": "noop",
                "mode": "noop",
                "source_sha": exact_head,
            }
        ]

    return {
        "platform_changed": platform_changed,
        "gateway_changed": gateway_changed,
        "deploy_runner_changed": deploy_runner_changed,
        "deployment_package_changed": deployment_package_changed,
        "control_plane_changed": control_plane_changed,
        "runtime_deployment_changed": runtime_deployment_changed,
        "build_platform": build_platform,
        "build_gateway": build_gateway,
        "build_deploy_runner": build_deploy_runner,
        "has_image_builds": has_image_builds,
        "release_relevant": release_relevant,
        # Compatibility aliases for existing contracts while #1328 lands.
        "runner_changed": deploy_runner_changed,
        "deployment_changed": deployment_package_changed,
        "control_plane": control_plane_changed,
        # A newly built component is produced from the exact candidate/head. A
        # reused component source SHA is deliberately resolved from trusted
        # persisted release state on the Synology deploy runner instead.
        "platform_source_sha": exact_head,
        "gateway_source_sha": exact_head,
        "runner_source_sha": exact_head,
        "changed_paths": changed,
        "matrix": {"include": include},
    }


def _write_output(path: Path, result: dict[str, object]) -> None:
    bool_keys = (
        "platform_changed",
        "gateway_changed",
        "deploy_runner_changed",
        "deployment_package_changed",
        "control_plane_changed",
        "runtime_deployment_changed",
        "build_platform",
        "build_gateway",
        "build_deploy_runner",
        "has_image_builds",
        "release_relevant",
        "runner_changed",
        "deployment_changed",
        "control_plane",
    )
    lines = [f"{key}={'true' if result[key] else 'false'}" for key in bool_keys]
    for key in ("platform_source_sha", "gateway_source_sha", "runner_source_sha"):
        lines.append(f"{key}={result[key]}")
    lines.append(f"matrix={json.dumps(result['matrix'], separators=(',', ':'))}")
    lines.append(f"changed_paths={json.dumps(result['changed_paths'], separators=(',', ':'))}")
    with path.open("a", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def _write_summary(path: Path, result: dict[str, object]) -> None:
    matrix = result["matrix"]
    assert isinstance(matrix, dict)
    include = matrix["include"]
    assert isinstance(include, list)
    rows = "\n".join(
        f"| {entry['name']} | {entry['mode']} | `{entry['source_sha']}` |"
        for entry in include
    )
    path.write_text(
        "### Synology staging build classification\n\n"
        f"- control-plane change: `{str(result['control_plane_changed']).lower()}`\n"
        f"- deployment-package change: `{str(result['deployment_package_changed']).lower()}`\n"
        f"- runtime deployment reconciliation: `{str(result['runtime_deployment_changed']).lower()}`\n"
        f"- image jobs allocated: `{str(result['has_image_builds']).lower()}`\n"
        f"- changed paths: `{len(result['changed_paths'])}`\n\n"
        "| component | mode | build source SHA |\n"
        "|---|---|---|\n"
        f"{rows}\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--event",
        required=True,
        choices=("pull_request", "merge_group", "push", "workflow_dispatch"),
    )
    parser.add_argument("--base", default="")
    parser.add_argument("--head", default="")
    parser.add_argument("--github-output")
    parser.add_argument("--summary")
    parser.add_argument("--paths-json")
    args = parser.parse_args()

    if args.event == "workflow_dispatch":
        paths: list[str] = []
    elif args.paths_json:
        loaded = json.loads(args.paths_json)
        if not isinstance(loaded, list) or not all(isinstance(item, str) for item in loaded):
            raise SystemExit("--paths-json must be a JSON list of path strings")
        paths = loaded
    else:
        paths = _changed_paths(args.base, args.head)

    result = classify(paths, args.event, args.head)
    if args.github_output:
        _write_output(Path(args.github_output), result)
    if args.summary:
        _write_summary(Path(args.summary), result)
    if not args.github_output:
        print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
