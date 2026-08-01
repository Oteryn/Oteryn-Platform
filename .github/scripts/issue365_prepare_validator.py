#!/usr/bin/env python3
"""Prepare the ephemeral Issue #365 validator from the retired frozen harness."""

from __future__ import annotations

import argparse
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match, found {count}")
    return text.replace(old, new, 1)


def extract_run_block(source: str) -> str:
    lines = source.splitlines()
    start: int | None = None

    for index, line in enumerate(lines):
        context = lines[max(0, index - 5) : index]
        if line == "        run: |" and any(
            "Execute isolated exact-frozen 12-sample matrix" in item
            for item in context
        ):
            start = index + 1
            break

    if start is None:
        raise SystemExit("main validator run block not found")

    body: list[str] = []
    for line in lines[start:]:
        if line.startswith("      - name: "):
            break
        if line.startswith("          "):
            body.append(line[10:])
        elif line == "":
            body.append("")
        else:
            raise SystemExit(
                f"unexpected indentation in validator block: {line!r}"
            )

    return "\n".join(body) + "\n"


def prepare(source: str) -> str:
    script = extract_run_block(source)

    old_build = """tar --exclude=.git -C "$GITHUB_WORKSPACE" -cf - . \\
  | docker build \\
      -t "$base_image" \\
      -f deploy/synology/docker/platform.Dockerfile \\
      -"""
    new_build = """echo "::notice::ISSUE365_STAGE=build-platform-image"
docker build --progress=plain \\
  -t "$base_image" \\
  -f "$GITHUB_WORKSPACE/deploy/synology/docker/platform.Dockerfile" \\
  "$GITHUB_WORKSPACE""""
    script = replace_once(
        script,
        old_build,
        new_build,
        "platform image build",
    )

    script = replace_once(
        script,
        'docker exec "$app_container" python3 - <<\'PY\'',
        'docker exec -i "$app_container" python3 - <<\'PY\'',
        "python heredoc stdin",
    )

    matrix_anchor = """set -euo pipefail
cd /workspace

: > "$ISSUE365_SERVER_TRACE""""
    matrix_replacement = """set -euo pipefail
cd /workspace
START_SESSION="$(php -r 'require "vendor/autoload.php"; echo (new ReflectionClass(Illuminate\\Session\\Middleware\\StartSession::class))->getFileName();')"
export START_SESSION

echo "matrix-start" > "$RUN_ROOT/LAST_STAGE"
: > "$ISSUE365_SERVER_TRACE""""
    script = replace_once(
        script,
        matrix_anchor,
        matrix_replacement,
        "matrix framework path",
    )

    stage_replacements = {
        'cat <<EOF | docker build -t "$validator_image" -': """echo "::notice::ISSUE365_STAGE=build-validator-image"
cat <<EOF | docker build -t "$validator_image" -""",
        'docker exec "$app_container" composer install': """echo "::notice::ISSUE365_STAGE=composer-install"
docker exec "$app_container" composer install""",
        "for script in \\": """echo "::notice::ISSUE365_STAGE=install-observers"
for script in \\""",
        'docker exec -i "$app_container" bash -s <<\'MATRIX\'': """echo "::notice::ISSUE365_STAGE=matrix"
docker exec -i "$app_container" bash -s <<'MATRIX'""",
    }
    for old, new in stage_replacements.items():
        script = replace_once(script, old, new, f"stage anchor {old!r}")

    bootstrap_old = (
        'docker exec "$app_container" bash '
        "scripts/acceptance/bootstrap-production-like.sh"
    )
    bootstrap_new = """echo "::notice::ISSUE365_STAGE=dependency-readiness"
if ! docker exec -i "$app_container" bash -s <<'READINESS'
set -euo pipefail
stable=0
for attempt in $(seq 1 90); do
  db_ok=0
  redis_ok=0
  if mariadb \\
      --protocol=TCP \\
      --connect-timeout=5 \\
      -h127.0.0.1 \\
      -P3306 \\
      -uroot \\
      -p"$MARIADB_ROOT_PASSWORD" \\
      -Nse 'SELECT 1' 2>/dev/null | grep -qx 1; then
    db_ok=1
  fi
  if [[ "$(redis-cli -h 127.0.0.1 -p 6379 ping 2>/dev/null || true)" == "PONG" ]]; then
    redis_ok=1
  fi
  if [[ "$db_ok" -eq 1 && "$redis_ok" -eq 1 ]]; then
    stable=$((stable + 1))
    if [[ "$stable" -ge 3 ]]; then
      exit 0
    fi
  else
    stable=0
  fi
  sleep 2
done
echo "MariaDB/Redis readiness did not stabilize." >&2
exit 1
READINESS
then
  echo "Dependency readiness failed; collecting isolated container diagnostics." >&2
  docker inspect "$mariadb_container" "$redis_container" "$app_container" || true
  docker logs "$mariadb_container" || true
  docker logs "$redis_container" || true
  docker logs "$app_container" || true
  exit 1
fi

echo "::notice::ISSUE365_STAGE=acceptance-bootstrap"
if ! docker exec "$app_container" bash scripts/acceptance/bootstrap-production-like.sh; then
  echo "Acceptance bootstrap failed; collecting isolated container diagnostics." >&2
  docker inspect "$mariadb_container" "$redis_container" "$app_container" || true
  docker logs "$mariadb_container" || true
  docker logs "$redis_container" || true
  docker logs "$app_container" || true
  exit 1
fi"""
    script = replace_once(
        script,
        bootstrap_old,
        bootstrap_new,
        "dependency readiness and acceptance bootstrap",
    )

    return script


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("source", type=Path)
    parser.add_argument("target", type=Path)
    args = parser.parse_args()

    source = args.source.read_text(encoding="utf-8")
    target = prepare(source)
    args.target.write_text(target, encoding="utf-8")
    args.target.chmod(0o700)


if __name__ == "__main__":
    main()
