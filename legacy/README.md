# legacy

This directory holds the earlier Oteryn Platform implementation, moved here
unchanged on owner decision 2026-11-11 when the Rust portal in `../portal-rs/`
became the main platform:

- the Laravel 13 application (PHP 8.5): `app/`, `bootstrap/`, `config/`,
  `database/`, `lang/`, `public/`, `resources/`, `routes/`, `storage/`,
  `tests/`, `artisan`, `composer.*`, `phpunit*.xml`, `phpstan.neon.dist`,
  `.env.example`;
- the Go game gateway: `services/game-gateway/`;
- their scripts, tools and deployment package: `scripts/acceptance/`,
  `scripts/phase7/`, `scripts/ci/` (Synology image routing and container
  supply chain), `tests/ci/` (Synology deployment contracts), `tools/`
  (Laravel-only validators and audits), `deploy/ci/` (Laravel CI images) and
  `deploy/synology/` (Synology staging Compose package for Laravel and the
  Go gateway).

## Status

Frozen. No workflow builds, tests, scans for coverage or deploys anything
here; the CI lanes that ran it were removed or reduced to report-only stubs
that keep their required check names. Relative paths inside these files
still assume the old repository root and were not rewritten, so the tree is
not runnable in place.

Do not extend this code. Change it only for a task that names it explicitly
(for example a security fix needed before cutover or evidence for a
contract). New work goes to `../portal-rs/` (plan
`../docs/architecture/OTERYN_PORTAL_RUST_PLAN.md`).

## Why it is kept

- Reference for needs and contracts while the portal is built (plan §1).
- Source of the one-time legacy data export read by `admin-cli
  import-legacy` (plan §9); the import reads that export, not this tree.
- Rollback reference until cutover (plan §9.1).

The directory is deleted by a separate pull request at cutover (plan §9,
step 6). Git history keeps every file.
