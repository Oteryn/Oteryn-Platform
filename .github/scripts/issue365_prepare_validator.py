#!/usr/bin/env python3
'Prepare the ephemeral Issue #365 validator from the retired frozen harness.'

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

    old_build = '''tar --exclude=.git -C "$GITHUB_WORKSPACE" -cf - . \\
  | docker build \\
      -t "$base_image" \\
      -f deploy/synology/docker/platform.Dockerfile \\
      -'''
    new_build = '''echo "::notice::ISSUE365_STAGE=build-platform-image"
docker build --progress=plain \\
  -t "$base_image" \\
  -f "$GITHUB_WORKSPACE/deploy/synology/docker/platform.Dockerfile" \\
  "$GITHUB_WORKSPACE"'''
    script = replace_once(
        script,
        old_build,
        new_build,
        "platform image build",
    )

    script = replace_once(
        script,
        "docker exec \"$app_container\" python3 - <<'PY'",
        "docker exec -i \"$app_container\" python3 - <<'PY'",
        "python heredoc stdin",
    )

    matrix_anchor = '''set -euo pipefail
cd /workspace

: > "$ISSUE365_SERVER_TRACE"'''
    matrix_replacement = '''set -euo pipefail
cd /workspace
START_SESSION="$(php -r 'require "vendor/autoload.php"; echo (new ReflectionClass(Illuminate\\Session\\Middleware\\StartSession::class))->getFileName();')"
export START_SESSION

echo "matrix-start" > "$RUN_ROOT/LAST_STAGE"
: > "$ISSUE365_SERVER_TRACE"'''
    script = replace_once(
        script,
        matrix_anchor,
        matrix_replacement,
        "matrix framework path",
    )

    mariadb_run_old = '''docker run -d \\
  --name "$mariadb_container" \\
  --network "$network" \\
  --network-alias mariadb \\
  -e MARIADB_ROOT_PASSWORD=acceptance-ci-root-not-a-secret \\
  -e MARIADB_ROOT_HOST=% \\
  mariadb:11.8 >/dev/null'''
    mariadb_run_new = '''docker run -d \\
  --name "$mariadb_container" \\
  --network "$network" \\
  --network-alias mariadb \\
  --tmpfs /var/lib/mysql:rw,size=1g \\
  -e MARIADB_ROOT_PASSWORD=acceptance-ci-root-not-a-secret \\
  -e MARIADB_ROOT_HOST=% \\
  mariadb:11.8 >/dev/null'''
    script = replace_once(
        script,
        mariadb_run_old,
        mariadb_run_new,
        "MariaDB isolated tmpfs",
    )

    script = replace_once(
        script,
        r"Illuminate\\Contracts\\Console\\Kernel",
        r"Illuminate\Contracts\Console\Kernel",
        "runtime metadata Laravel kernel namespace",
    )
    composer_namespace = r"Composer\\InstalledVersions"
    if script.count(composer_namespace) != 2:
        raise SystemExit(
            "runtime metadata Composer namespace: expected two matches, "
            f"found {script.count(composer_namespace)}"
        )
    script = script.replace(
        composer_namespace,
        r"Composer\InstalledVersions",
    )

    old_selectors = '''selectors = {
    '01-observer-create.sh': 'mkdir -p app/Support',
    '02-observer-patch.sh': 'export START_SESSION=',
    '03-browser-helper.sh': 'issue365-trace-helper.mjs',
    '04-probe-test.sh': 'admin-wiki-issue365-probe.spec.mjs',
    '05-media-snapshot.sh': 'media-snapshot.php',
}'''
    new_selectors = '''selectors = {
    '01-observer-create.sh': 'cat > app/Support/Issue365Trace.php',
    '02-observer-patch.sh': 'export START_SESSION=vendor/laravel/framework/src/Illuminate/Session/Middleware/StartSession.php',
    '03-browser-helper.sh': "cat > scripts/acceptance/tests/issue365-trace-helper.mjs <<'JS'",
    '04-probe-test.sh': "path = Path('scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs')",
    '05-media-snapshot.sh': "cat > \"$RUN_ROOT/media-snapshot.php\" <<'PHP'",
}'''
    script = replace_once(
        script,
        old_selectors,
        new_selectors,
        "runbook observer selectors",
    )

    old_hashes = '''  git hash-object \\
    scripts/acceptance/tests/admin-wiki-administration.spec.mjs \\
    scripts/acceptance/playwright.config.mjs \\
    scripts/acceptance/seed-browser-editorial-media.php \\
    routes/modules/wiki.php \\
    composer.lock > "$RUN_ROOT/source-blob-hashes.txt"'''
    new_hashes = '''  for source_path in \\
    scripts/acceptance/tests/admin-wiki-administration.spec.mjs \\
    scripts/acceptance/playwright.config.mjs \\
    scripts/acceptance/seed-browser-editorial-media.php \\
    routes/modules/wiki.php \\
    composer.lock; do
    printf '%s  %s\\n' "$(git hash-object "$source_path")" "$source_path"
  done > "$RUN_ROOT/source-blob-hashes.txt"'''
    script = replace_once(
        script,
        old_hashes,
        new_hashes,
        "source hashes with paths",
    )

    validator_build_end = '''EOF

app_key="base64:$(python3 - <<'PY' '''.rstrip()
    validator_build_replacement = """EOF

docker run --rm \\
  -v "$work_volume:/workspace" \\
  -v "$evidence_volume:/evidence" \\
  -w /workspace \\
  "$validator_image" \\
  bash -lc '
    set -euo pipefail
    git config --global --add safe.directory /workspace
    git status --porcelain=v1 > /evidence/issue365-run/git-status-frozen-initial.txt
    test ! -s /evidence/issue365-run/git-status-frozen-initial.txt
    echo frozen-clean > /evidence/issue365-run/LAST_STAGE
  '

app_key="base64:$(python3 - <<'PY' """.rstrip()
    script = replace_once(
        script,
        validator_build_end,
        validator_build_replacement,
        "initial frozen git status evidence",
    )

    metadata_anchor = '''docker exec "$app_container" bash -lc '
  set -euo pipefail
  git show --no-patch --format=fuller HEAD > "$RUN_ROOT/target-commit.txt"'''
    metadata_replacement = '''docker exec "$app_container" sh -c 'echo runtime-metadata > "$RUN_ROOT/LAST_STAGE"'
docker exec "$app_container" bash -lc '
  set -euo pipefail
  git show --no-patch --format=fuller HEAD > "$RUN_ROOT/target-commit.txt"'''
    script = replace_once(
        script,
        metadata_anchor,
        metadata_replacement,
        "runtime metadata stage",
    )

    observer_generation_anchor = '''docker exec -i "$app_container" python3 - <<'PY'
from pathlib import Path
import re'''
    observer_generation_replacement = '''docker exec "$app_container" sh -c 'echo observer-generation > "$RUN_ROOT/LAST_STAGE"'
docker exec -i "$app_container" python3 - <<'PY'
from pathlib import Path
import re'''
    script = replace_once(
        script,
        observer_generation_anchor,
        observer_generation_replacement,
        "observer generation stage",
    )

    observer_install_anchor = '''PY

for script in \\
  01-observer-create.sh'''
    observer_install_replacement = '''PY

docker exec "$app_container" bash -lc 'sha256sum "$RUN_ROOT"/runtime/*.sh > "$RUN_ROOT/runtime/SHA256SUMS"'

for script in \\
  01-observer-create.sh'''
    script = replace_once(
        script,
        observer_install_anchor,
        observer_install_replacement,
        "observer generated hashes",
    )

    matrix_launch_anchor = """done

docker exec -i "$app_container" bash -s <<'MATRIX'""".rstrip()
    matrix_launch_replacement = """done

docker exec "$app_container" bash -lc '
  set -euo pipefail
  test -f app/Support/Issue365Trace.php
  test -f scripts/acceptance/tests/issue365-trace-helper.mjs
  test -f scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs
  test -f "$RUN_ROOT/media-snapshot.php"
  test -s "$RUN_ROOT/StartSession.sha256.instrumented"
  printf "%s\\n" \\
    app/Support/Issue365Trace.php \\
    scripts/acceptance/tests/issue365-trace-helper.mjs \\
    scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs \\
    "$RUN_ROOT/media-snapshot.php" \\
    "$START_SESSION" \\
    > "$RUN_ROOT/observers-installed.txt"
  echo observers-installed > "$RUN_ROOT/LAST_STAGE"
'

docker exec -i "$app_container" bash -s <<'MATRIX'""".rstrip()
    script = replace_once(
        script,
        matrix_launch_anchor,
        matrix_launch_replacement,
        "observer installation evidence",
    )

    sample_anchor = '''sample_dir="$RUN_ROOT/samples/$sample"
        mkdir -p "$sample_dir"'''
    sample_replacement = '''sample_dir="$RUN_ROOT/samples/$sample"
        echo "sample-${sample}-start" > "$RUN_ROOT/LAST_STAGE"
        mkdir -p "$sample_dir"'''
    script = replace_once(
        script,
        sample_anchor,
        sample_replacement,
        "sample stage checkpoint",
    )

    cleanup_old = '''sha256sum -c "$RUN_ROOT/StartSession.sha256.before"
php -l "$START_SESSION" > "$RUN_ROOT/StartSession.restore-lint.txt"
test -z "$(git status --porcelain=v1)"
git diff --exit-code
git diff --cached --exit-code
git status --porcelain=v1 > "$RUN_ROOT/git-status-after.txt"'''
    cleanup_new = '''echo cleanup-restore > "$RUN_ROOT/LAST_STAGE"
sha256sum -c "$RUN_ROOT/StartSession.sha256.before" > "$RUN_ROOT/StartSession.restore-check.txt"
sha256sum "$START_SESSION" > "$RUN_ROOT/StartSession.sha256.after"
cmp -s "$RUN_ROOT/StartSession.sha256.before" "$RUN_ROOT/StartSession.sha256.after"
php -l "$START_SESSION" > "$RUN_ROOT/StartSession.restore-lint.txt"
git diff --exit-code
git diff --cached --exit-code
git clean -ffdqx
git status --porcelain=v1 > "$RUN_ROOT/git-status-after.txt"
test ! -s "$RUN_ROOT/git-status-after.txt"
echo matrix-complete > "$RUN_ROOT/LAST_STAGE"'''
    script = replace_once(
        script,
        cleanup_old,
        cleanup_new,
        "framework restore and git cleanup evidence",
    )

    bootstrap_old = (
        'docker exec "$app_container" bash '
        "scripts/acceptance/bootstrap-production-like.sh"
    )
    bootstrap_new = r'''echo "::notice::ISSUE365_STAGE=dependency-readiness"
stable=0
for attempt in $(seq 1 450); do
  db_direct=0
  db_via_app=0
  redis_direct=0
  redis_via_app=0

  if docker exec "$mariadb_container" \
      mariadb-admin \
      -uroot \
      -pacceptance-ci-root-not-a-secret \
      ping \
      --silent >/dev/null 2>&1; then
    db_direct=1
  fi

  if docker exec "$app_container" bash -lc \
      'mariadb --protocol=TCP --connect-timeout=5 -h127.0.0.1 -P3306 -uroot -p"$MARIADB_ROOT_PASSWORD" -Nse "SELECT 1"' \
      2>/dev/null | grep -qx 1; then
    db_via_app=1
  fi

  if docker exec "$redis_container" redis-cli ping 2>/dev/null | grep -qx PONG; then
    redis_direct=1
  fi

  if docker exec "$app_container" redis-cli -h 127.0.0.1 -p 6379 ping \
      2>/dev/null | grep -qx PONG; then
    redis_via_app=1
  fi

  if [[ "$db_direct" -eq 1 \
      && "$db_via_app" -eq 1 \
      && "$redis_direct" -eq 1 \
      && "$redis_via_app" -eq 1 ]]; then
    stable=$((stable + 1))
    if [[ "$stable" -ge 3 ]]; then
      break
    fi
  else
    stable=0
  fi

  for dependency in "$mariadb_container" "$redis_container" "$app_container"; do
    if [[ "$(docker inspect -f '{{.State.Running}}' "$dependency" 2>/dev/null || true)" != "true" ]]; then
      echo "Dependency container stopped: $dependency" >&2
      docker inspect "$dependency" || true
      docker logs "$dependency" || true
      exit 1
    fi
  done

  if (( attempt % 30 == 0 )); then
    echo "Dependency readiness attempt ${attempt}/450: db_direct=${db_direct}, db_via_app=${db_via_app}, redis_direct=${redis_direct}, redis_via_app=${redis_via_app}"
    docker logs --tail 40 "$mariadb_container" || true
  fi

  sleep 2
done

if [[ "$stable" -lt 3 ]]; then
  echo "MariaDB/Redis readiness did not stabilize." >&2
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
fi'''
    script = replace_once(
        script,
        bootstrap_old,
        bootstrap_new,
        "dependency readiness and acceptance bootstrap",
    )

    stage_replacements = {
        'cat <<EOF | docker build -t "$validator_image" -': (
            'echo "::notice::ISSUE365_STAGE=build-validator-image"\n'
            'cat <<EOF | docker build -t "$validator_image" -'
        ),
        'docker exec "$app_container" composer install': (
            'echo "::notice::ISSUE365_STAGE=composer-install"\n'
            'docker exec "$app_container" composer install'
        ),
        "for script in \\": (
            'echo "::notice::ISSUE365_STAGE=install-observers"\n'
            "docker exec \"$app_container\" sh -c 'echo observer-install > \"$RUN_ROOT/LAST_STAGE\"'\n"
            "for script in \\"
        ),
        "docker exec -i \"$app_container\" bash -s <<'MATRIX'": (
            'echo "::notice::ISSUE365_STAGE=matrix"\n'
            "docker exec -i \"$app_container\" bash -s <<'MATRIX'"
        ),
    }
    for old, new in stage_replacements.items():
        script = replace_once(script, old, new.rstrip(), f"stage anchor {old!r}")

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
