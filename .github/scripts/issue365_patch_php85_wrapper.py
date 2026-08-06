#!/usr/bin/env python3
"""Patch the proven Issue #365 validator to use Platform-container PHP 8.5.

The Playwright image receives Docker CLI and a tiny `php` wrapper. The wrapper
executes PHP inside the already running Platform validator container, whose
image is built from the frozen target's `php:8.5-cli-alpine` Dockerfile.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def regex_replace_once(
    text: str,
    pattern: str,
    replacement: str,
    label: str,
) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.S)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return updated


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()

    text = args.source.read_text(encoding="utf-8")

    validator_identity = (
        'validator_image="oteryn-issue365-validator:'
        '${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"'
    )
    text = replace_once(
        text,
        validator_identity,
        validator_identity
        + '\nplaywright_php_image="oteryn-issue365-playwright-php85:'
        + '${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"',
        "derived Playwright image identity",
    )

    first_validator_probe = '''docker run --rm \\
  -v "$work_volume:/workspace" \\
  -v "$evidence_volume:/evidence" \\
  -w /workspace \\
  "$validator_image" \\
  bash -lc '''

    wrapper_build = r'''echo "::notice::ISSUE365_STAGE=build-playwright-php85-wrapper-image"
cat <<EOF | docker build --progress=plain -t "$playwright_php_image" -
FROM ${PLAYWRIGHT_IMAGE}
USER root
RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends docker.io ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN printf '%s\n' \
      '#!/usr/bin/env bash' \
      'set -euo pipefail' \
      ': "${ISSUE365_APP_CONTAINER:?ISSUE365_APP_CONTAINER is required}"' \
      'exec docker exec -w /workspace "$ISSUE365_APP_CONTAINER" php "$@"' \
      > /usr/local/bin/php \
    && chmod 0755 /usr/local/bin/php
EOF

docker run --rm \
  -v "$work_volume:/workspace" \
  -v "$evidence_volume:/evidence" \
  -w /workspace \
  "$validator_image" \
  bash -lc '''

    text = replace_once(
        text,
        first_validator_probe,
        wrapper_build,
        "one-time Playwright wrapper image build",
    )

    text = replace_once(
        text,
        '-e PLAYWRIGHT_IMAGE="$PLAYWRIGHT_IMAGE" \\',
        '-e PLAYWRIGHT_IMAGE="$playwright_php_image" \\',
        "nested Playwright image selection",
    )

    text = replace_once(
        text,
        '''-e PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \\
                  -v "$WORK_VOLUME:/workspace" \\''',
        '''-e PLAYWRIGHT_BROWSERS_PATH=/ms-playwright \\
                  -e ISSUE365_APP_CONTAINER="$APP_CONTAINER_NAME" \\
                  -v /var/run/docker.sock:/var/run/docker.sock \\
                  -v "$WORK_VOLUME:/workspace" \\''',
        "Playwright Docker socket and app identity",
    )

    text = regex_replace_once(
        text,
        r"bash -lc 'apt-get update >/dev/null && "
        r"DEBIAN_FRONTEND=noninteractive apt-get install -y "
        r"--no-install-recommends php-cli php-mysql php-mbstring php-xml "
        r"php-curl php-redis >/dev/null && command -v php && php -v && "
        r"npx playwright test tests/admin-wiki-issue365-probe\.spec\.mjs "
        r"--project=responsive-mobile --workers=1 --retries=0 --reporter=line'",
        "bash -lc 'command -v php && php -v && "
        "php -r \"exit(PHP_VERSION_ID >= 80500 ? 0 : 1);\" && "
        "npx playwright test tests/admin-wiki-issue365-probe.spec.mjs "
        "--project=responsive-mobile --workers=1 --retries=0 --reporter=line'",
        "replace per-sample PHP install with PHP 8.5 wrapper preflight",
    )

    required = [
        "playwright_php_image=",
        "ISSUE365_APP_CONTAINER",
        "/var/run/docker.sock:/var/run/docker.sock",
        "PHP_VERSION_ID >= 80500",
        "--workers=1 --retries=0",
    ]
    for marker in required:
        if marker not in text:
            raise SystemExit(f"patched validator missing required marker: {marker}")

    forbidden = [
        "--no-install-recommends php-cli php-mysql",
        "apt-get install -y --no-install-recommends php-cli",
    ]
    for marker in forbidden:
        if marker in text:
            raise SystemExit(f"patched validator retains forbidden marker: {marker}")

    args.target.write_text(text, encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
