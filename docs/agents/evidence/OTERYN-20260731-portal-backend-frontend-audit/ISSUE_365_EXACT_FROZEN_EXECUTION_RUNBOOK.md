# Issue #365 exact frozen-target execution runbook

Task: `OTERYN-20260731-portal-backend-frontend-audit`  
Frozen target: `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`  
Execution class: `EPHEMERAL_VALIDATOR_ONLY`  
Repository mutation authorization: none; every observer described below must be removed before the run ends and must never be committed.

## Purpose

This runbook executes the only remaining audit gate:

1. restore the historical publication-flash assertion ephemerally on the exact frozen SHA;
2. run clean responsive-mobile samples after an EditorialMedia reset;
3. compare immediate publication with explicit pre-scroll plus media-settle;
4. repeat the pair with exactly one intentionally corrupt EditorialMedia row;
5. correlate browser request start, Laravel request ID, session-lock acquisition/release, session load/save and flash aging;
6. return the checkout and installed framework files to their exact pre-run state.

It is deliberately independent of the configured session and cache drivers. The observer is inserted into the installed, lockfile-resolved `StartSession` middleware and records only sanitized state.

## Source facts that the validator must preserve

- The exact frozen test is `scripts/acceptance/tests/admin-wiki-administration.spec.mjs`.
- The historical observer to restore is:

  ```js
  await expect(page.getByRole('status')).toContainText('Wiki article published.');
  ```

- The mobile project is `responsive-mobile`, Chromium, viewport `390×844`, touch enabled and mobile emulation enabled.
- Playwright workers are one and the focused run must use retries zero.
- The exact frozen Wiki media index, thumbnail, article edit and publication routes all use `->block()`.
- `seed-browser-editorial-media.php reset` removes rows and stored files together.
- `seed-referenced` creates one valid media row after reset.
- `corrupt-files <id>` corrupts only the specified row's original and thumbnail objects.

Do not replace the original Wiki administration scenario with a synthetic application flow.

## Required environment

Run this inside a mutable checkout-capable validator that can already execute the repository's production-like acceptance environment. Reuse the same database, cache/session service, mail service, application environment and HTTP runtime shape used by the repository acceptance workflow. Do not substitute SQLite, an in-memory session driver, a mocked HTTP kernel or Playwright request-context calls.

The procedure below starts after those dependencies have been provisioned. If the validator starts the application manually, it must use the repository's existing acceptance environment and expose it at `ACCEPTANCE_BASE_URL`, normally `http://127.0.0.1:8080`.

Required executables:

- Git;
- PHP and extensions accepted by the frozen lockfile;
- Composer;
- Node/npm and the pinned acceptance package lock;
- Chromium installed through the pinned Playwright harness;
- Python 3;
- `sha256sum` and `tar`.

## 1. Fail-closed identity and cleanliness preflight

```bash
set -euo pipefail

export TARGET_SHA=b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
export RUN_ROOT="${RUN_ROOT:-/tmp/oteryn-issue365-${TARGET_SHA}}"
export ACCEPTANCE_BASE_URL="${ACCEPTANCE_BASE_URL:-http://127.0.0.1:8080}"
export ISSUE365_SERVER_TRACE="$RUN_ROOT/server-trace.jsonl"

mkdir -p "$RUN_ROOT"
chmod 700 "$RUN_ROOT"

test "$(git rev-parse HEAD)" = "$TARGET_SHA"
test -z "$(git status --porcelain=v1)"

git show --no-patch --format=fuller HEAD > "$RUN_ROOT/target-commit.txt"
git status --porcelain=v1 > "$RUN_ROOT/git-status-before.txt"
git hash-object \
  scripts/acceptance/tests/admin-wiki-administration.spec.mjs \
  scripts/acceptance/playwright.config.mjs \
  scripts/acceptance/seed-browser-editorial-media.php \
  routes/modules/wiki.php \
  composer.lock > "$RUN_ROOT/source-blob-hashes.txt"

php -r '
require "vendor/autoload.php";
$app = require "bootstrap/app.php";
$app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
echo json_encode([
    "framework" => Composer\InstalledVersions::getPrettyVersion("laravel/framework"),
    "framework_reference" => Composer\InstalledVersions::getReference("laravel/framework"),
    "session_driver" => config("session.driver"),
    "session_connection" => config("session.connection"),
    "session_store" => config("session.store"),
    "cache_default" => config("cache.default"),
], JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), PHP_EOL;
' > "$RUN_ROOT/runtime-config.json"

php artisan route:list --name=admin.wiki --json > "$RUN_ROOT/admin-wiki-routes.json"

grep -F 'admin.wiki.media.index' "$RUN_ROOT/admin-wiki-routes.json"
grep -F 'admin.wiki.media.thumbnail' "$RUN_ROOT/admin-wiki-routes.json"
grep -F 'admin.wiki.articles.publish' "$RUN_ROOT/admin-wiki-routes.json"
```

Abort if the SHA is different, the checkout is dirty, the routes are missing, Composer is not installed from the frozen lockfile or the existing acceptance environment is not healthy.

## 2. Install the ephemeral server observer

The observer writes no cookies, tokens, email addresses, session IDs or complete session payloads. It records a SHA-256 hash of the session ID, the exact non-secret publication status only when it matches the expected message, and boolean membership of `status` in `_flash.new` and `_flash.old`.

Create `app/Support/Issue365Trace.php`:

```bash
mkdir -p app/Support
cat > app/Support/Issue365Trace.php <<'PHP'
<?php

declare(strict_types=1);

namespace App\Support;

use Illuminate\Http\Request;
use Illuminate\Routing\Route;
use RuntimeException;
use Throwable;

final class Issue365Trace
{
    public static function framework(Request $request, mixed $session, string $event, array $extra = []): void
    {
        if (! ($request->is('admin/wiki') || $request->is('admin/wiki/*'))) {
            return;
        }

        $traceFile = getenv('ISSUE365_SERVER_TRACE');
        if (! is_string($traceFile) || $traceFile === '') {
            throw new RuntimeException('ISSUE365_SERVER_TRACE is required for the ephemeral observer.');
        }

        $route = $request->route();
        $referer = $request->headers->get('referer');
        $snapshot = [
            'session_attached' => is_object($session),
            'session_id_sha256' => null,
            'status_present' => false,
            'status_value' => null,
            'flash_new_has_status' => false,
            'flash_old_has_status' => false,
        ];

        if (is_object($session) && method_exists($session, 'getId')) {
            try {
                $snapshot['session_id_sha256'] = hash('sha256', (string) $session->getId());
                if (method_exists($session, 'exists') && method_exists($session, 'get')) {
                    $snapshot['status_present'] = (bool) $session->exists('status');
                    $status = $session->get('status');
                    $snapshot['status_value'] = $status === 'Wiki article published.'
                        ? 'Wiki article published.'
                        : ($snapshot['status_present'] ? '[present-other]' : null);
                    $new = $session->get('_flash.new', []);
                    $old = $session->get('_flash.old', []);
                    $snapshot['flash_new_has_status'] = is_array($new) && in_array('status', $new, true);
                    $snapshot['flash_old_has_status'] = is_array($old) && in_array('status', $old, true);
                }
            } catch (Throwable $exception) {
                $snapshot['snapshot_error'] = $exception::class;
            }
        }

        $payload = array_merge([
            'schema' => 1,
            'event' => $event,
            'monotonic_ns' => (string) hrtime(true),
            'wall_time_utc' => gmdate('Y-m-d\TH:i:s.u\Z'),
            'sample' => $request->headers->get('X-Issue365-Sample', '[missing]'),
            'action_mode' => $request->headers->get('X-Issue365-Action', '[missing]'),
            'fixture_mode' => $request->headers->get('X-Issue365-Fixture', '[missing]'),
            'request_id' => $request->attributes->get('request_id'),
            'method' => $request->getMethod(),
            'path' => '/'.$request->path(),
            'route' => $route instanceof Route ? $route->getName() : null,
            'referer_path' => is_string($referer) ? parse_url($referer, PHP_URL_PATH) : null,
        ], $snapshot, $extra);

        $encoded = json_encode($payload, JSON_THROW_ON_ERROR | JSON_UNESCAPED_SLASHES).PHP_EOL;
        if (file_put_contents($traceFile, $encoded, FILE_APPEND | LOCK_EX) === false) {
            throw new RuntimeException('Could not write the Issue #365 server trace.');
        }
    }
}
PHP
```

Patch only the installed framework file, never the repository lockfile or application behavior. The patch fails unless every expected frozen framework shape occurs exactly once.

```bash
export START_SESSION=vendor/laravel/framework/src/Illuminate/Session/Middleware/StartSession.php
cp "$START_SESSION" "$RUN_ROOT/StartSession.php.before"
sha256sum "$START_SESSION" > "$RUN_ROOT/StartSession.sha256.before"

python3 <<'PY'
from pathlib import Path

path = Path('vendor/laravel/framework/src/Illuminate/Session/Middleware/StartSession.php')
text = path.read_text(encoding='utf-8')

def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'expected exactly one StartSession pattern, found {count}: {old[:80]!r}')
    text = text.replace(old, new, 1)

replace_once(
    "    public function handle($request, Closure $next)\n    {\n",
    "    public function handle($request, Closure $next)\n    {\n"
    "        \\App\\Support\\Issue365Trace::framework($request, null, 'start_session_enter');\n",
)

replace_once(
    "        $lock = $this->cache($this->manager->blockDriver())\n",
    "        \\App\\Support\\Issue365Trace::framework($request, $session, 'lock_attempt');\n\n"
    "        $lock = $this->cache($this->manager->blockDriver())\n",
)

replace_once(
    "            );\n\n            return $this->handleStatefulRequest($request, $session, $next);\n",
    "            );\n\n"
    "            \\App\\Support\\Issue365Trace::framework($request, $session, 'lock_acquired');\n\n"
    "            return $this->handleStatefulRequest($request, $session, $next);\n",
)

replace_once(
    "        } finally {\n            $lock?->release();\n        }\n",
    "        } finally {\n"
    "            \\App\\Support\\Issue365Trace::framework($request, $session, 'lock_release_begin');\n"
    "            $lock?->release();\n"
    "            \\App\\Support\\Issue365Trace::framework($request, $session, 'lock_released');\n"
    "        }\n",
)

replace_once(
    "        $request->setLaravelSession(\n            $this->startSession($request, $session)\n        );\n",
    "        \\App\\Support\\Issue365Trace::framework($request, $session, 'session_load_begin');\n"
    "        $request->setLaravelSession(\n            $this->startSession($request, $session)\n        );\n"
    "        \\App\\Support\\Issue365Trace::framework($request, $session, 'session_loaded');\n",
)

replace_once(
    "        $this->saveSession($request);\n        return $response;\n",
    "        \\App\\Support\\Issue365Trace::framework($request, $session, 'session_save_begin');\n"
    "        $this->saveSession($request);\n"
    "        \\App\\Support\\Issue365Trace::framework($request, $session, 'session_saved');\n"
    "        return $response;\n",
)

path.write_text(text, encoding='utf-8')
PY

php -l app/Support/Issue365Trace.php
php -l "$START_SESSION"
sha256sum "$START_SESSION" > "$RUN_ROOT/StartSession.sha256.instrumented"
```

For every blocked Wiki request, the expected server sequence is:

```text
start_session_enter
lock_attempt
lock_acquired
session_load_begin
session_loaded
session_save_begin
session_saved
lock_release_begin
lock_released
```

A missing, duplicated or reordered event invalidates the sample.

## 3. Create the ephemeral browser request tracer

Create `scripts/acceptance/tests/issue365-trace-helper.mjs`:

```bash
cat > scripts/acceptance/tests/issue365-trace-helper.mjs <<'JS'
function safeUrl(raw) {
  try {
    const url = new URL(raw);
    return { origin: url.origin, path: url.pathname };
  } catch {
    return { origin: '[invalid]', path: '[invalid]' };
  }
}

function frameUrl(request) {
  try {
    return safeUrl(request.frame().url()).path;
  } catch {
    return '[no-frame]';
  }
}

export function installIssue365Probe(page) {
  const events = [];
  const mediaInflight = new Set();
  let phase = 'setup';
  let observation = null;

  const record = (event, payload = {}) => {
    events.push({
      event,
      monotonic_ns: process.hrtime.bigint().toString(),
      phase,
      ...payload,
    });
  };

  const classify = (raw) => {
    const { path } = safeUrl(raw);
    const media = path === '/admin/wiki/media'
      || /^\/admin\/wiki\/media\/\d+\/thumbnail$/u.test(path);
    const wiki = path === '/admin/wiki' || path.startsWith('/admin/wiki/');
    return { path, media, wiki };
  };

  page.on('request', (request) => {
    const classification = classify(request.url());
    if (!classification.wiki) return;
    if (classification.media) mediaInflight.add(request);
    record('browser_request_start', {
      method: request.method(),
      path: classification.path,
      resource_type: request.resourceType(),
      frame_path: frameUrl(request),
      referer_path: safeUrl(request.headers().referer ?? '').path,
      redirected_from: request.redirectedFrom() ? safeUrl(request.redirectedFrom().url()).path : null,
      media_inflight: mediaInflight.size,
    });
  });

  page.on('response', (response) => {
    const request = response.request();
    const classification = classify(response.url());
    if (!classification.wiki) return;
    record('browser_response', {
      method: request.method(),
      path: classification.path,
      status: response.status(),
      request_id: response.headers()['x-request-id'] ?? null,
      location_path: safeUrl(response.headers().location ?? '').path,
      media_inflight: mediaInflight.size,
    });
  });

  const finish = (event, request, failure = null) => {
    const classification = classify(request.url());
    if (!classification.wiki) return;
    if (classification.media) mediaInflight.delete(request);
    record(event, {
      method: request.method(),
      path: classification.path,
      failure,
      media_inflight: mediaInflight.size,
    });
  };

  page.on('requestfinished', (request) => finish('browser_request_finished', request));
  page.on('requestfailed', (request) => finish(
    'browser_request_failed',
    request,
    request.failure()?.errorText ?? 'unknown',
  ));

  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) {
      record('browser_main_frame_navigated', { path: safeUrl(frame.url()).path });
    }
  });

  return {
    setPhase(value) {
      phase = value;
      record('browser_phase');
    },
    async waitForMediaIdle({ timeoutMs = 15_000, stableMs = 750 } = {}) {
      const deadline = Date.now() + timeoutMs;
      let zeroSince = null;
      while (Date.now() < deadline) {
        if (mediaInflight.size === 0) {
          zeroSince ??= Date.now();
          if (Date.now() - zeroSince >= stableMs) {
            record('browser_media_idle', { stable_ms: stableMs });
            return;
          }
        } else {
          zeroSince = null;
        }
        await page.waitForTimeout(25);
      }
      throw new Error(`Issue #365 media requests did not settle; in-flight=${mediaInflight.size}`);
    },
    setObservation(value) {
      observation = value;
    },
    snapshot() {
      return {
        schema: 1,
        sample: process.env.ISSUE365_SAMPLE_ID ?? '[missing]',
        action_mode: process.env.ISSUE365_ACTION_MODE ?? '[missing]',
        fixture_mode: process.env.ISSUE365_FIXTURE_MODE ?? '[missing]',
        observation,
        events,
      };
    },
  };
}
JS
```

## 4. Generate the exact frozen scenario with the restored observer

Copy rather than edit the tracked source test:

```bash
cp \
  scripts/acceptance/tests/admin-wiki-administration.spec.mjs \
  scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs

python3 <<'PY'
from pathlib import Path

path = Path('scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs')
text = path.read_text(encoding='utf-8')

def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f'expected exactly one probe-test pattern, found {count}: {old[:100]!r}')
    text = text.replace(old, new, 1)

replace_once(
    "} from './helpers.mjs';\n",
    "} from './helpers.mjs';\nimport { installIssue365Probe } from './issue365-trace-helper.mjs';\n",
)

replace_once(
    "test.beforeEach(async ({ page }) => {\n  page.__acceptanceDiagnostics = installDiagnostics(page);\n});\n",
    "test.beforeEach(async ({ page }) => {\n"
    "  page.__acceptanceDiagnostics = installDiagnostics(page);\n"
    "  page.__issue365Probe = installIssue365Probe(page);\n"
    "});\n",
)

replace_once(
    "test.afterEach(async ({ page }, testInfo) => {\n  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);\n});\n",
    "test.afterEach(async ({ page }, testInfo) => {\n"
    "  await attachDiagnostics(testInfo, page.__acceptanceDiagnostics);\n"
    "  await testInfo.attach('issue365-browser-trace', {\n"
    "    body: Buffer.from(JSON.stringify(page.__issue365Probe?.snapshot() ?? {}, null, 2), 'utf8'),\n"
    "    contentType: 'application/json',\n"
    "  });\n"
    "});\n",
)

replace_once(
    "test('@wiki-admin trusted editor creates, previews and publishes bilingual Wiki content', async ({ page, context }) => {\n",
    "test('@wiki-admin trusted editor creates, previews and publishes bilingual Wiki content', async ({ page, context }) => {\n"
    "  const sample = process.env.ISSUE365_SAMPLE_ID ?? 'missing';\n"
    "  const actionMode = process.env.ISSUE365_ACTION_MODE ?? 'missing';\n"
    "  const fixtureMode = process.env.ISSUE365_FIXTURE_MODE ?? 'missing';\n"
    "  if (!['immediate', 'prescroll'].includes(actionMode)) throw new Error(`Invalid ISSUE365_ACTION_MODE: ${actionMode}`);\n"
    "  if (!['clean', 'one-corrupt'].includes(fixtureMode)) throw new Error(`Invalid ISSUE365_FIXTURE_MODE: ${fixtureMode}`);\n"
    "  await context.setExtraHTTPHeaders({\n"
    "    'X-Issue365-Sample': sample,\n"
    "    'X-Issue365-Action': actionMode,\n"
    "    'X-Issue365-Fixture': fixtureMode,\n"
    "  });\n",
)

old = """  // The responsive editor loads the approved-media picker through authenticated thumbnail requests.
  // Finish those session-bearing subresource requests before the next lifecycle POST.
  await page.waitForLoadState('networkidle');
  await expect(page.getByRole('button', { name: 'Publish', exact: true })).toBeVisible();
  await page.getByRole('button', { name: 'Publish', exact: true }).click();
  await expect(page.getByText(/Status:\\s*Published/i)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Unpublish to draft' })).toBeVisible();
"""
new = """  // Preserve the frozen pre-publication boundary, then compare direct action with
  // an explicit pre-scroll and media-settle control.
  page.__issue365Probe.setPhase('pre-publication-networkidle');
  await page.waitForLoadState('networkidle');
  const publishButton = page.getByRole('button', { name: 'Publish', exact: true });
  await expect(publishButton).toBeVisible();

  if (actionMode === 'prescroll') {
    page.__issue365Probe.setPhase('prescroll');
    await publishButton.scrollIntoViewIfNeeded();
    await page.__issue365Probe.waitForMediaIdle();
  }

  const publishResponsePromise = page.waitForResponse((response) => {
    const url = new URL(response.url());
    return response.request().method() === 'POST'
      && /^\\/admin\\/wiki\\/articles\\/\\d+\\/publish$/u.test(url.pathname);
  });

  page.__issue365Probe.setPhase('publish-click');
  await publishButton.click();
  const publishResponse = await publishResponsePromise;
  page.__issue365Probe.setPhase('post-publish-redirect');

  const statusTexts = await page.getByRole('status').allTextContents();
  page.__issue365Probe.setObservation({
    publish_status: publishResponse.status(),
    publish_location: publishResponse.headers().location ?? null,
    final_url: page.url(),
    role_status_texts: statusTexts.map((value) => value.trim()),
    durable_published_visible: await page.getByText(/Status:\\s*Published/i).isVisible(),
    unpublish_visible: await page.getByRole('button', { name: 'Unpublish to draft' }).isVisible(),
  });

  // Historical observer restored ephemerally on the exact frozen source.
  await expect(page.getByRole('status')).toContainText('Wiki article published.');
  await expect(page.getByText(/Status:\\s*Published/i)).toBeVisible();
  await expect(page.getByRole('button', { name: 'Unpublish to draft' })).toBeVisible();
"""
replace_once(old, new)

path.write_text(text, encoding='utf-8')
PY

node --check scripts/acceptance/tests/issue365-trace-helper.mjs
node --check scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs
```

The copied test must differ from the frozen test only by:

- the browser trace helper;
- sample metadata headers;
- the immediate versus pre-scroll control;
- the restored historical status assertion;
- sanitized evidence attachment.

## 5. Fixture preparation and snapshots

Create an external snapshot helper:

```bash
cat > "$RUN_ROOT/media-snapshot.php" <<'PHP'
<?php

declare(strict_types=1);

use App\EditorialMedia\Infrastructure\Models\EditorialMedia;
use Illuminate\Contracts\Console\Kernel;
use Illuminate\Support\Facades\Storage;

require getcwd().'/vendor/autoload.php';
$app = require getcwd().'/bootstrap/app.php';
$app->make(Kernel::class)->bootstrap();

$rows = EditorialMedia::query()->orderBy('id')->get()->map(static function (EditorialMedia $media): array {
    $disk = Storage::disk($media->disk);
    return [
        'id' => $media->id,
        'storage_exists' => is_string($media->storage_path) && $disk->exists($media->storage_path),
        'thumbnail_exists' => is_string($media->thumbnail_path) && $disk->exists($media->thumbnail_path),
    ];
})->all();

echo json_encode(['count' => count($rows), 'rows' => $rows], JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), PHP_EOL;
PHP
```

For every sample:

```bash
php scripts/acceptance/seed-browser-editorial-media.php reset
php artisan cache:clear
php "$RUN_ROOT/media-snapshot.php" > "$SAMPLE_DIR/media-before.json"
```

For `clean`, assert the count is zero:

```bash
php -r '$x=json_decode(file_get_contents($argv[1]), true, 512, JSON_THROW_ON_ERROR); exit(($x["count"] ?? -1) === 0 ? 0 : 1);' \
  "$SAMPLE_DIR/media-before.json"
```

For `one-corrupt`, create and corrupt exactly one row:

```bash
MEDIA_EMAIL="issue365-media-${SAMPLE_ID}@example.test"
php scripts/acceptance/seed-browser-editorial-media.php \
  seed-identity "$MEDIA_EMAIL" 'Issue365-Media-9!Pass' '' unconfirmed '' \
  > "$SAMPLE_DIR/media-identity.json"

php scripts/acceptance/seed-browser-editorial-media.php \
  seed-referenced "$MEDIA_EMAIL" "Issue 365 ${SAMPLE_ID}" \
  > "$SAMPLE_DIR/media-seeded.json"

MEDIA_ID="$(php -r '$x=json_decode(file_get_contents($argv[1]), true, 512, JSON_THROW_ON_ERROR); echo $x["media_id"];' "$SAMPLE_DIR/media-seeded.json")"
php scripts/acceptance/seed-browser-editorial-media.php corrupt-files "$MEDIA_ID" \
  > "$SAMPLE_DIR/media-corrupted.json"
php "$RUN_ROOT/media-snapshot.php" > "$SAMPLE_DIR/media-before.json"

php -r '
$x=json_decode(file_get_contents($argv[1]), true, 512, JSON_THROW_ON_ERROR);
$rows=$x["rows"] ?? [];
$ok=($x["count"] ?? -1) === 1
    && count($rows) === 1
    && $rows[0]["storage_exists"] === true
    && $rows[0]["thumbnail_exists"] === true;
exit($ok ? 0 : 1);
' "$SAMPLE_DIR/media-before.json"
```

The corrupt objects continue to exist, but their bytes fail the integrity check. This is expected. The row count must be exactly one.

## 6. Mandatory 12-sample matrix

Run three independent zero-retry samples in each cell:

| Fixture | Immediate action | Pre-scroll + media settle |
|---|---:|---:|
| `clean` | 3 | 3 |
| `one-corrupt` | 3 | 3 |

Do not use Playwright retries as samples. Do not run cells concurrently. Keep one worker.

The application HTTP runtime must be restarted after installing the observer and must inherit `ISSUE365_SERVER_TRACE`. Truncate the trace before starting the instrumented runtime:

```bash
: > "$ISSUE365_SERVER_TRACE"
chmod 600 "$ISSUE365_SERVER_TRACE"
```

Use the repository's existing acceptance HTTP-runtime command unchanged, adding only the `ISSUE365_SERVER_TRACE` environment variable. Verify `/health` before the matrix.

Run one sample with:

```bash
export ACCEPTANCE_SHA="$TARGET_SHA"
export ACCEPTANCE_ZERO_RETRIES=1
export ACCEPTANCE_PROFILE=critical
export ACCEPTANCE_RUN_ID="$SAMPLE_ID"
export ACCEPTANCE_OUTPUT_DIR="$SAMPLE_DIR/test-results"
export ISSUE365_SAMPLE_ID="$SAMPLE_ID"
export ISSUE365_ACTION_MODE="$ACTION_MODE"
export ISSUE365_FIXTURE_MODE="$FIXTURE_MODE"

set +e
(
  cd scripts/acceptance
  npx playwright test \
    tests/admin-wiki-issue365-probe.spec.mjs \
    --project=responsive-mobile \
    --workers=1 \
    --retries=0 \
    --reporter=line
) > >(tee "$SAMPLE_DIR/playwright.stdout.log") \
  2> >(tee "$SAMPLE_DIR/playwright.stderr.log" >&2)
TEST_EXIT=$?
set -e
printf '%s\n' "$TEST_EXIT" > "$SAMPLE_DIR/playwright.exit-code"

php "$RUN_ROOT/media-snapshot.php" > "$SAMPLE_DIR/media-after.json"
```

A shell driver may iterate the matrix, but it must perform the reset, cache clear, fixture assertion and evidence hashing separately for every sample.

After recording `media-after.json`, reset again before the next sample:

```bash
php scripts/acceptance/seed-browser-editorial-media.php reset
php artisan cache:clear
```

## 7. Correlation and causal classification

Join browser and server records by the response `X-Request-ID`. Within each sample, order server records by integer `monotonic_ns`.

The relevant routes are:

- `admin.wiki.articles.publish`;
- `admin.wiki.articles.edit`;
- `admin.wiki.media.index`;
- `admin.wiki.media.thumbnail`.

### Publication flash successfully reaches the redirected document

The following chain is required:

1. publication `session_save_begin` has `status_present=true` and `flash_new_has_status=true`;
2. publication `session_saved` has `status_present=true` and `flash_old_has_status=true`;
3. redirected edit `session_loaded` has `status_present=true` and `flash_old_has_status=true`;
4. browser evidence contains `Wiki article published.` in `role_status_texts`;
5. redirected edit `session_saved` may then remove the old flash.

### Old-document media request consumes the flash before redirect GET

Causal proof requires one sample with the complete order below, using matching session-ID hashes:

1. the browser starts a media index or thumbnail request while its recorded frame/referer is the old article-edit document;
2. publication POST acquires the session lock and saves `status` as old flash;
3. that already-started old-document media request acquires the same session lock before the redirected edit GET;
4. media `session_loaded` sees `status_present=true` and `flash_old_has_status=true`;
5. media `session_saved` has `status_present=false` after flash aging;
6. redirected edit `session_loaded` has `status_present=false`;
7. the first redirected document lacks `Wiki article published.` while durable Published state and `Unpublish to draft` remain visible.

Only this complete chain promotes the request-order mechanism from `DERIVED` to `PROVEN`.

### Rejection or weakening

The old-document action-scroll mechanism is rejected or weakened when:

- no media request starts between the immediate action boundary and publication navigation;
- media requests always acquire the lock after the redirect GET;
- the flash is absent already at publication `session_save_begin` or `session_saved`;
- pre-scroll and immediate modes have identical request ordering across all samples;
- a clean no-media sample reproduces the missing flash.

A clean reproduction is `REPRODUCED_CLEAN` and proves the damaged-row fixture is not required. A failure limited to one-corrupt immediate samples with successful pre-scroll controls supports `REPRODUCED_POLLUTED_ONLY`, but causality still requires the complete lock/session chain above.

## 8. Required evidence package

Create one directory per sample containing:

- `playwright.stdout.log` and `playwright.stderr.log`;
- `playwright.exit-code`;
- Playwright test-results and `issue365-browser-trace` attachment;
- `media-before.json` and `media-after.json`;
- identity/seed/corruption JSON when applicable;
- the server trace subset for that sample;
- a machine-readable verdict JSON.

At the root include:

- exact commit identity and source blob hashes;
- runtime/session/cache configuration;
- framework version/reference;
- original and instrumented `StartSession` hashes;
- route list;
- full sanitized server trace;
- sample matrix summary;
- SHA-256 manifest.

No raw cookies, authorization headers, passwords, MFA secrets/recovery codes, CSRF tokens, complete session IDs or complete session payloads may be retained.

Create the manifest and immutable archive:

```bash
find "$RUN_ROOT" -type f ! -name SHA256SUMS -print0 \
  | sort -z \
  | xargs -0 sha256sum > "$RUN_ROOT/SHA256SUMS"

tar --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner \
  -C "$(dirname "$RUN_ROOT")" \
  -czf "${RUN_ROOT}.tar.gz" "$(basename "$RUN_ROOT")"
sha256sum "${RUN_ROOT}.tar.gz" > "${RUN_ROOT}.tar.gz.sha256"
```

## 9. Mandatory cleanup and restoration proof

Stop the instrumented HTTP runtime before restoration.

```bash
cp "$RUN_ROOT/StartSession.php.before" "$START_SESSION"
rm -f app/Support/Issue365Trace.php
rm -f scripts/acceptance/tests/issue365-trace-helper.mjs
rm -f scripts/acceptance/tests/admin-wiki-issue365-probe.spec.mjs

sha256sum -c "$RUN_ROOT/StartSession.sha256.before"
php -l "$START_SESSION"

test -z "$(git status --porcelain=v1)"
git diff --exit-code
git diff --cached --exit-code
git status --porcelain=v1 > "$RUN_ROOT/git-status-after.txt"
```

If `app/Support` was created only for this observer and is empty, remove the directory. The final Git status must be empty. A dirty checkout, changed vendor hash or missing cleanup evidence invalidates the entire package.

## 10. Verdict gate

The audit may become `VALIDATED` only when all of the following are present:

- exact frozen SHA identity;
- all 12 mandatory samples or an explicitly stronger matrix;
- zero Playwright retries;
- clean fixture proof before every clean sample;
- exactly-one-row proof before every polluted sample;
- correlated browser request, Laravel request ID, lock, load, save and flash-aging evidence;
- durable publication observation for every sample;
- complete redaction and hashes;
- successful cleanup with an empty Git status.

Until then:

- `OTERYN-AUDIT-P35-005` remains `REPRODUCED_INTERMITTENT`;
- current remediation remains `NOT_PROVEN_REMEDIATED`;
- `OTERYN-AUDIT-P35-006` remains the separate MEDIUM fixture-isolation finding;
- normalized totals remain **0 HIGH, 6 MEDIUM and 1 LOW**;
- verdict remains **`VALIDATED_WITH_CORRECTIONS`**;
- no merge, deployment, production action or Canary mutation is authorized.
