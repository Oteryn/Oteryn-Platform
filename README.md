# Oteryn Platform

Oteryn Platform is the first-party web/application platform for the Oteryn Open Tibia Server ecosystem. It is intended to replace MyAAC as the long-term web platform while Canary remains a separate game-server project.

## Current implementation

The repository contains a Laravel 13 modular monolith on PHP 8.5 with a server-rendered Blade UI and explicit module/data boundaries.

Evidence-backed capabilities on `main` include bounded slices of:

- public portal shell, homepage, news, announcements, events, SEO, downloads and legal/informational content;
- public characters, guilds, highscores, online/status, servers and deaths through least-privilege read boundaries;
- Platform Identity, login/logout, registered sessions, recovery, security lifecycle and MFA;
- greenfield account provisioning/binding and character creation through operation-specific contracts and least-privilege adapters;
- CMS, Wiki, EditorialMedia, Support/Moderation, Admin/RBAC and Audit;
- versioned Game Catalog foundations;
- Oteryn Coins Wallet and Character Bazaar foundations;
- operational, public-edge and exact-head E2E/quality controls;
- baseline local/test configuration, lockfile-backed Composer installs and GitHub Actions CI.

`AVAILABLE` means at least one validated repository capability exists; it does not imply complete product scope, production proof or activation authority. Read `docs/architecture/MODULE_CATALOG.md` and exact task/PR evidence before making a completeness claim.

Current planned boundaries include:

- `PlayerCompanion` for versioned calculators, build planning, hunt guidance, private session analysis, progression tracking and recommendations;
- `LiveOps` for authoritative time-sensitive world/service status and schedules;
- a concrete-consumer-driven `PlatformAPI`;
- Products/Entitlements, LegalCommerce and later provider Payments under separate gates.

## Local setup

Requirements: PHP 8.5 with Laravel's required extensions, Composer 2, and PDO SQLite for the default local database.

```sh
cp .env.example .env
composer install
php artisan key:generate
php -r "file_exists('database/database.sqlite') || touch('database/database.sqlite');"
php artisan serve
```

The application is then available at `http://localhost:8000`.

The health endpoint is available at `GET /health`. It reports only application availability and does not expose environment variables, configuration, version details or secrets.

## Canary and integration boundaries

Canary and login-server behavior are external contracts. The generic `canary` database connection is read-only and must use an externally provisioned least-privilege principal.

Shared writes are allowed only through separately documented operation-specific contracts and adapters. Current bounded examples include greenfield account provisioning, character creation and Character Bazaar ownership transfer. No module may infer generic TFS/MyAAC schema or silently broaden those privileges.

Configure applicable `CANARY_DB_*` and runtime values in local environment files. Do not reuse Canary server credentials or database root/admin accounts.

## Validation

```sh
composer validate --strict
composer format:check
composer test
```

To apply formatting:

```sh
composer format
```

GitHub Actions installs dependencies from `composer.lock`, validates Composer metadata and lock consistency, checks formatting and runs the applicable Laravel/PHPUnit and repository validation suites. Required evidence remains tied to the exact tested head.

## Licensing and contributions

Oteryn Platform is proprietary. Public repository visibility does not make the project open source and does not grant permission to copy, modify, redistribute, sublicense, host or commercially use the project or a derivative of it.

`LICENSE.md` is the canonical proprietary notice. It applies only to original Oteryn Platform material that does not carry a separate notice. Third-party dependencies, assets, game data, fixtures and protocol or compatibility material remain governed by their own rights and are bounded by `THIRD_PARTY_NOTICES.md`.

External contributions are not accepted by default. Do not submit code, documentation, assets or data unless the repository owner has invited the contribution and agreed written contribution terms before acceptance. See `CONTRIBUTING.md`.

A future source-available, open-source, dual-license or component-specific policy requires a separate owner-approved decision and a provenance review of the exact material covered.

## Authoritative project documentation

- `AGENTS.md` and `AGENTS.override.md` — mandatory operating rules for agents.
- `LICENSE.md` — canonical repository licensing and use boundary.
- `THIRD_PARTY_NOTICES.md` — third-party rights and unresolved provenance boundary.
- `docs/agents/PROJECT_STATE.md` — current project phase and next work.
- `docs/agents/REPOSITORY_MAP.md` — repository navigation and ownership map.
- `docs/architecture/ARCHITECTURE_AUTHORITY.md` — architecture precedence and canonical routing.
- `docs/architecture/SYSTEM_ARCHITECTURE.md` — system boundaries and target topology.
- `docs/architecture/MODULE_CATALOG.md` — module ownership and repository-availability classification.
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` — current portal assessment, remaining decisions and completion gate.
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` — accepted player-tools architecture and delivery priorities.
- `docs/architecture/SECURITY_ARCHITECTURE.md` — mandatory security invariants.
- `docs/architecture/DATA_OWNERSHIP.md` — persistent-data ownership rules.
- `docs/contracts/` — Canary/login-server and operation-specific integration contracts.
- `docs/agents/tasks/active/` — active implementation/task records.

Repository state, accepted ADRs, focused architecture, task records, Git and live PR/CI state are authoritative over chat history.