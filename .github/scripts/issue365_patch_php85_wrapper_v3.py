#!/usr/bin/env python3
"""Structurally patch the exact Issue #365 validator for PHP 8.5."""

from __future__ import annotations

import argparse
import base64
from pathlib import Path


def require_one(indices: list[int], label: str) -> int:
    if len(indices) != 1:
        raise SystemExit(f"{label}: expected one match, found {len(indices)}")
    return indices[0]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()

    lines = args.source.read_text(encoding="utf-8").splitlines()

    validator_declarations = [
        index
        for index, line in enumerate(lines)
        if line.startswith('validator_image="oteryn-issue365-validator:')
    ]
    declaration = require_one(
        validator_declarations,
        "derived Playwright image declaration anchor",
    )
    lines.insert(
        declaration + 1,
        'playwright_php_image="oteryn-issue365-playwright-php85:'
        '${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"',
    )

    build_starts = [
        index
        for index, line in enumerate(lines)
        if line.strip() == 'cat <<EOF | docker build -t "$validator_image" -'
    ]
    build_start = require_one(build_starts, "validator image build anchor")
    build_ends = [
        index
        for index in range(build_start + 1, len(lines))
        if lines[index].strip() == "EOF"
    ]
    if not build_ends:
        raise SystemExit("validator image build terminator not found")
    build_end = build_ends[0]

    wrapper = """#!/usr/bin/env bash
set -euo pipefail
: "${ISSUE365_APP_CONTAINER:?ISSUE365_APP_CONTAINER is required}"
case "$PWD" in
  /workspace*) workdir="$PWD" ;;
  *) workdir=/workspace/scripts/acceptance ;;
esac
env_args=()
for name in ACCEPTANCE_RUN_ID ACCEPTANCE_OUTPUT_DIR ISSUE365_SAMPLE_ID ISSUE365_ACTION_MODE ISSUE365_FIXTURE_MODE; do
  if [[ -n "${!name+x}" ]]; then
    env_args+=(-e "$name=${!name}")
  fi
done
exec docker exec -w "$workdir" "${env_args[@]}" "$ISSUE365_APP_CONTAINER" php "$@"
"""
    wrapper_b64 = base64.b64encode(wrapper.encode("utf-8")).decode("ascii")
    derived_build = [
        "",
        'echo "::notice::ISSUE365_STAGE=build-playwright-php85-wrapper-image"',
        'cat <<EOF | docker build --progress=plain -t "$playwright_php_image" -',
        "FROM ${PLAYWRIGHT_IMAGE}",
        "USER root",
        "RUN apt-get update \\",
        "    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends docker.io ca-certificates \\",
        "    && rm -rf /var/lib/apt/lists/*",
        f"RUN echo {wrapper_b64} | base64 -d > /usr/local/bin/php \\",
        "    && chmod 0755 /usr/local/bin/php",
        "EOF",
    ]
    lines[build_end + 1 : build_end + 1] = derived_build

    app_image_env = [
        index
        for index, line in enumerate(lines)
        if line.strip() == '-e PLAYWRIGHT_IMAGE="$PLAYWRIGHT_IMAGE" \\'
    ]
    app_image_index = require_one(
        app_image_env,
        "application Playwright image environment anchor",
    )
    app_indent = lines[app_image_index][: -len(lines[app_image_index].lstrip())]
    lines[app_image_index] = (
        app_indent + '-e PLAYWRIGHT_IMAGE="$playwright_php_image" \\'
    )

    browser_env = [
        index
        for index, line in enumerate(lines)
        if line.strip() == "-e PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \\"
    ]
    browser_env_index = require_one(
        browser_env,
        "sample Playwright browser environment anchor",
    )
    if lines[browser_env_index + 1].strip() != '-v "$WORK_VOLUME:/workspace" \\':
        raise SystemExit(
            "sample Playwright work-volume anchor does not immediately follow browser environment"
        )
    sample_indent = lines[browser_env_index][: -len(lines[browser_env_index].lstrip())]
    lines[browser_env_index + 1 : browser_env_index + 1] = [
        sample_indent + '-e ISSUE365_APP_CONTAINER="$APP_CONTAINER_NAME" \\',
        sample_indent + "-v /var/run/docker.sock:/var/run/docker.sock \\",
    ]

    install_lines = [
        index
        for index, line in enumerate(lines)
        if "apt-get update >/dev/null" in line
        and "npx playwright test tests/admin-wiki-issue365-probe.spec.mjs" in line
    ]
    install_index = require_one(
        install_lines,
        "per-sample Playwright PHP installation anchor",
    )
    install_indent = lines[install_index][: -len(lines[install_index].lstrip())]
    lines[install_index] = (
        install_indent
        + "bash -lc 'command -v php && php -v && "
        + 'php -r "exit(PHP_VERSION_ID >= 80500 ? 0 : 1);" && '
        + "npx playwright test tests/admin-wiki-issue365-probe.spec.mjs "
        + "--project=responsive-mobile --workers=1 --retries=0 --reporter=line' \\\n"
    ).rstrip("\n")

    text = "\n".join(lines) + "\n"

    required_markers = {
        "derived image identity": "playwright_php_image=",
        "wrapper app identity": "ISSUE365_APP_CONTAINER",
        "wrapper Docker socket": "/var/run/docker.sock:/var/run/docker.sock",
        "PHP 8.5 preflight": "PHP_VERSION_ID >= 80500",
        "zero-retry single worker": "--workers=1 --retries=0",
        "derived app image": '-e PLAYWRIGHT_IMAGE="$playwright_php_image"',
    }
    for label, marker in required_markers.items():
        if marker not in text:
            raise SystemExit(f"generated validator missing {label}: {marker}")

    forbidden_markers = [
        "--no-install-recommends php-cli php-mysql",
        "apt-get install -y --no-install-recommends php-cli",
    ]
    for marker in forbidden_markers:
        if marker in text:
            raise SystemExit(f"generated validator retains forbidden marker: {marker}")

    if text.count("PHP_VERSION_ID >= 80500") != 1:
        raise SystemExit("generated validator must contain exactly one PHP 8.5 preflight")
    if text.count('-e ISSUE365_APP_CONTAINER="$APP_CONTAINER_NAME"') != 1:
        raise SystemExit("generated validator must pass exactly one app-container identity")
    if text.count("-v /var/run/docker.sock:/var/run/docker.sock") < 2:
        raise SystemExit(
            "generated validator must mount the Docker socket in both app and Playwright containers"
        )

    args.target.write_text(text, encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
