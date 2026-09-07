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

PLATFORM_EXACT = {
    "composer.json",
    "composer.lock",
    "artisan",
    "deploy/synology/docker/platform.Dockerfile",
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
PLATFORM_SOURCE_PATHS = (
    "composer.json",
    "composer.lock",
    "artisan",
    "app",
    "bootstrap",
    "config",
    "database",
    "public",
    "resources",
    "routes",
    "lang",
    "storage",
    "deploy/synology/docker/platform.Dockerfile",
    "deploy/synology/docker/platform-media.ini",
    "deploy/synology/docker/platform-entrypoint.sh",
    "deploy/synology/release-contract.env",
    *WIKI_RUNTIME_PROVENANCE,
)

GATEWAY_EXACT = {"deploy/synology/docker/gateway.Dockerfile"}
GATEWAY_PREFIXES = ("services/game-gateway/",)
GATEWAY_SOURCE_PATHS = (
    "services/game-gateway",
    "deploy/synology/docker/gateway.Dockerfile",
)

RUNNER_PREFIXES = ("deploy/synology/runner/",)
RUNNER_SOURCE_PATHS = ("deploy/synology/runner",)

CONTROL_PLANE_EXACT = {
    ".github/workflows/build-synology-staging-images.yml",
    ".github/workflows/deploy-synology-staging.yml",
    "scripts/ci/classify_synology_builds.py",
}
DEPLOYMENT_PREFIXES = ("deploy/synology/",)


def _matches(path: str, exact: set[str], prefixes: Iterable[str]) -> bool:
    return path in exact or any(path.startswith(prefix) for prefix in prefixes)


def _is_runtime_deployment_path(path: str) -> bool:
    return path.startswith(DEPLOYMENT_PREFIXES) and not path.startswith(RUNNER_PREFIXES)


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


def _component_source_sha(head: str, paths: tuple[str, ...]) -> str:
    result = subprocess.run(
        ["git", "log", "-1", "--format=%H", head, "--", *paths],
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr.strip() or f"failed to resolve component source SHA at {head}")
    candidate = result.stdout.strip()
    return candidate if len(candidate) == 40 else head


def classify(paths: Iterable[str], event_name: str, head: str) -> dict[str, object]:
    changed = sorted(set(paths))
    control_plane = any(path in CONTROL_PLANE_EXACT for path in changed)
    platform_changed = any(_matches(path, PLATFORM_EXACT, PLATFORM_PREFIXES) for path in changed)
    gateway_changed = any(_matches(path, GATEWAY_EXACT, GATEWAY_PREFIXES) for path in changed)
    runner_changed = any(path.startswith(RUNNER_PREFIXES) for path in changed)
    deployment_changed = any(path.startswith(DEPLOYMENT_PREFIXES) for path in changed)
    runtime_deployment_changed = any(_is_runtime_deployment_path(path) for path in changed)
    release_relevant = control_plane or platform_changed or gateway_changed or runtime_deployment_changed

    source_shas = {
        "platform": _component_source_sha(head, PLATFORM_SOURCE_PATHS) if head else "",
        "game-gateway": _component_source_sha(head, GATEWAY_SOURCE_PATHS) if head else "",
        "deploy-runner": _component_source_sha(head, RUNNER_SOURCE_PATHS) if head else "",
    }

    components = {
        "platform": {
            "package": "oteryn-platform",
            "dockerfile": "deploy/synology/docker/platform.Dockerfile",
        },
        "game-gateway": {
            "package": "oteryn-game-gateway",
            "dockerfile": "deploy/synology/docker/gateway.Dockerfile",
        },
        "deploy-runner": {
            "package": "oteryn-deploy-runner",
            "dockerfile": "deploy/synology/runner/Dockerfile",
        },
    }

    include: list[dict[str, str]] = []

    def add(name: str, mode: str) -> None:
        component = components[name]
        include.append(
            {
                "name": name,
                "package": str(component["package"]),
                "dockerfile": str(component["dockerfile"]),
                "mode": mode,
                "source_sha": source_shas[name] or head,
            }
        )

    if event_name == "workflow_dispatch":
        for name in ("platform", "game-gateway", "deploy-runner"):
            add(name, "full")
    elif event_name == "pull_request":
        if control_plane:
            for name in ("platform", "game-gateway", "deploy-runner"):
                add(name, "full")
        else:
            if platform_changed:
                add("platform", "full")
            if gateway_changed:
                add("game-gateway", "full")
            if runner_changed:
                add("deploy-runner", "full")
    elif event_name == "push":
        if release_relevant:
            add("platform", "full" if control_plane or platform_changed else "reuse")
            add("game-gateway", "full" if control_plane or gateway_changed else "reuse")
    else:
        raise ValueError(f"unsupported event: {event_name}")

    if not include:
        include = [
            {
                "name": "noop",
                "package": "noop",
                "dockerfile": "noop",
                "mode": "noop",
                "source_sha": head,
            }
        ]

    return {
        "platform_changed": platform_changed,
        "gateway_changed": gateway_changed,
        "runner_changed": runner_changed,
        "deployment_changed": deployment_changed,
        "runtime_deployment_changed": runtime_deployment_changed,
        "control_plane": control_plane,
        "release_relevant": release_relevant,
        "platform_source_sha": source_shas["platform"],
        "gateway_source_sha": source_shas["game-gateway"],
        "runner_source_sha": source_shas["deploy-runner"],
        "changed_paths": changed,
        "matrix": {"include": include},
    }


def _write_output(path: Path, result: dict[str, object]) -> None:
    bool_keys = (
        "platform_changed",
        "gateway_changed",
        "runner_changed",
        "deployment_changed",
        "runtime_deployment_changed",
        "control_plane",
        "release_relevant",
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
        f"- control-plane change: `{str(result['control_plane']).lower()}`\n"
        f"- deployment package change: `{str(result['deployment_changed']).lower()}`\n"
        f"- release-relevant deployment change: `{str(result['runtime_deployment_changed']).lower()}`\n"
        f"- Platform source SHA: `{result['platform_source_sha']}`\n"
        f"- Gateway source SHA: `{result['gateway_source_sha']}`\n"
        f"- changed paths: `{len(result['changed_paths'])}`\n\n"
        "| component | mode | component source SHA |\n"
        "|---|---|---|\n"
        f"{rows}\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", required=True, choices=("pull_request", "push", "workflow_dispatch"))
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
