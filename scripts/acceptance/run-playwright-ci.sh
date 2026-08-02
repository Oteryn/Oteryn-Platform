#!/usr/bin/env bash
set -euo pipefail

repo_root="${OTERYN_REPO_ROOT:-/workspace}"
acceptance_dir="$repo_root/scripts/acceptance"
toolchain_dir="/opt/oteryn-playwright"
linked_node_modules=0
child_pid=''

cleanup() {
  if [[ "$linked_node_modules" -eq 1 && -L "$acceptance_dir/node_modules" ]]; then
    rm -f "$acceptance_dir/node_modules"
  fi
}
trap cleanup EXIT

fail() {
  printf 'Playwright CI runtime error: %s\n' "$*" >&2
  exit 1
}

image_playwright="$(
  node -e '
    const packageJson = require(process.argv[1]);
    process.stdout.write(packageJson.version);
  ' "$toolchain_dir/node_modules/@playwright/test/package.json"
)"

read_expected_playwright() {
  node -e '
    const packageJson = require(process.argv[1]);
    const version = packageJson.devDependencies?.["@playwright/test"];
    if (!version || !/^\d+\.\d+\.\d+$/u.test(version)) process.exit(1);
    process.stdout.write(version);
  ' "$acceptance_dir/package.json"
}

prepare_workspace_node_modules() {
  local expected_playwright installed_playwright

  [[ -f "$acceptance_dir/package.json" ]] || return 0

  expected_playwright="$(read_expected_playwright)" || fail \
    'repository @playwright/test version must be an exact semantic version'
  [[ "$image_playwright" == "$expected_playwright" ]] || fail \
    "image Playwright $image_playwright does not match repository $expected_playwright"

  if [[ -e "$acceptance_dir/node_modules" ]]; then
    [[ -d "$acceptance_dir/node_modules" ]] || fail \
      "$acceptance_dir/node_modules exists but is not a directory"
    installed_playwright="$(
      node -e '
        const packageJson = require(process.argv[1]);
        process.stdout.write(packageJson.version);
      ' "$acceptance_dir/node_modules/@playwright/test/package.json" 2>/dev/null || true
    )"
    [[ "$installed_playwright" == "$expected_playwright" ]] || fail \
      "workspace Playwright ${installed_playwright:-missing} does not match repository $expected_playwright"
  else
    ln -s "$toolchain_dir/node_modules" "$acceptance_dir/node_modules"
    linked_node_modules=1
  fi
}

export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-/ms-playwright}"
export PATH="$toolchain_dir/node_modules/.bin:$PATH"
export NODE_PATH="${NODE_PATH:-$toolchain_dir/node_modules}"

case "${1:-}" in
  bash|/bin/bash|sh|/bin/sh)
    prepare_workspace_node_modules
    set +e
    "$@" &
    child_pid=$!
    trap 'kill -INT "$child_pid" 2>/dev/null || true' INT
    trap 'kill -TERM "$child_pid" 2>/dev/null || true' TERM
    wait "$child_pid"
    status=$?
    set -e
    exit "$status"
    ;;
esac

trap 'exit 130' INT
trap 'exit 143' TERM

for required in \
  "$repo_root/composer.json" \
  "$repo_root/composer.lock" \
  "$repo_root/artisan" \
  "$acceptance_dir/package.json" \
  "$acceptance_dir/tests/helpers.mjs"; do
  [[ -f "$required" ]] || fail "required repository file is missing: $required"
done

php -r '
if (PHP_VERSION_ID < 80500) {
    fwrite(STDERR, "PHP 8.5 or newer is required; found ".PHP_VERSION."\n");
    exit(1);
}
printf("php=%s\n", PHP_VERSION);
'

prepare_workspace_node_modules
export PATH="$acceptance_dir/node_modules/.bin:$PATH"

if [[ "${OTERYN_REQUIRE_VENDOR:-1}" == '1' ]]; then
  [[ -f "$repo_root/vendor/autoload.php" ]] || fail \
    'vendor/autoload.php is missing; install Composer dependencies before Playwright execution'
  (
    cd "$repo_root"
    composer check-platform-reqs
    php -l scripts/acceptance/assert-platform-state.php >/dev/null
    php artisan cache:clear --no-interaction >/dev/null
  )
fi

printf 'node=%s\n' "$(node --version)"
printf 'playwright=%s\n' "$image_playwright"
printf 'browsers_path=%s\n' "$PLAYWRIGHT_BROWSERS_PATH"

case "${1:-}" in
  --verify-only)
    exit 0
    ;;
  --runtime-smoke)
    node - "$toolchain_dir" <<'NODE'
const path = require('node:path');
const toolchainDir = process.argv[2];
const { chromium, firefox, webkit } = require(path.join(
  toolchainDir,
  'node_modules',
  '@playwright',
  'test',
));

(async () => {
  for (const [name, engine] of Object.entries({ chromium, firefox, webkit })) {
    const browser = await engine.launch({ headless: true });
    const page = await browser.newPage();
    await page.setContent(`<main><h1>${name} ready</h1></main>`);
    const text = await page.locator('h1').textContent();
    if (text !== `${name} ready`) {
      throw new Error(`${name} rendered unexpected smoke content: ${text}`);
    }
    await browser.close();
    process.stdout.write(`${name}=PASS\n`);
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE
    exit 0
    ;;
  '')
    fail 'no command supplied; use --verify-only, --runtime-smoke or Playwright CLI arguments'
    ;;
esac

set +e
"$acceptance_dir/node_modules/.bin/playwright" "$@"
status=$?
set -e
exit "$status"
