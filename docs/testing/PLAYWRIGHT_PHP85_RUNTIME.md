# Containerized Playwright PHP 8.5 Runtime

## Purpose

Oteryn acceptance tests call PHP from the Playwright process through `scripts/acceptance/tests/helpers.mjs`. The browser runner must therefore satisfy both toolchains at once:

- PHP compatible with `composer.json` and `composer.lock`;
- Node and the exact `@playwright/test` version declared by `scripts/acceptance/package.json`;
- installed Chromium, Firefox and WebKit browser binaries.

The retained runtime is built from `deploy/ci/playwright-php.Dockerfile` and entered through `scripts/acceptance/run-playwright-ci.sh`.

## Fixed failure mode

Issue #365 run `30763456046` used the official Playwright Ubuntu image and installed the distribution `php-cli` package inside each sample. That supplied PHP `8.3.6`, while Oteryn requires PHP `^8.5`. Every sample stopped before browser execution when an acceptance helper invoked `php artisan cache:clear`.

The retained runtime instead starts from `php:8.5-cli-bookworm`, adds Node 22 and the exact pinned Playwright package, and installs all three browser engines during image construction. No sample installs or selects PHP independently.

## Runtime contract

At startup the runner fails closed unless:

1. PHP is at least 8.5;
2. the image's `@playwright/test` version exactly equals the repository declaration;
3. the mounted checkout contains Composer dependencies;
4. Composer platform requirements pass;
5. the acceptance PHP helper parses;
6. `php artisan cache:clear` succeeds from the same mounted checkout.

The runner temporarily links the image's pinned Node dependencies only when the checkout has no `scripts/acceptance/node_modules`, and removes that link after execution.

## Commands

Build the exact checkout runtime:

```bash
docker build \
  -f deploy/ci/playwright-php.Dockerfile \
  -t oteryn-playwright-php85 \
  .
```

Verify the PHP/Composer/Node/Playwright contract after installing Composer dependencies:

```bash
docker run --rm \
  -v "$PWD:/workspace" \
  -w /workspace \
  oteryn-playwright-php85 \
  --verify-only
```

Launch all three installed browser engines:

```bash
docker run --rm \
  --ipc host \
  --shm-size 1g \
  -v "$PWD:/workspace" \
  -w /workspace \
  oteryn-playwright-php85 \
  --runtime-smoke
```

Run an acceptance project without changing the test source:

```bash
docker run --rm \
  --ipc host \
  --shm-size 1g \
  --network host \
  --env-file .env.acceptance \
  -v "$PWD:/workspace" \
  -w /workspace \
  oteryn-playwright-php85 \
  test --project=responsive-mobile --workers=1 --retries=0
```

## CI evidence

`.github/workflows/playwright-runtime-validation.yml` builds the image on the exact pull-request head, installs the exact Composer dependencies through that image, verifies the runtime contract, launches Chromium/Firefox/WebKit and uploads sanitized source/toolchain evidence.

This workflow validates test infrastructure only. It does not prove a product flow, staging deployment or production behavior. Product acceptance continues to use the existing exact-SHA profiles and their own environment evidence.
