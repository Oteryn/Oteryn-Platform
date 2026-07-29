# Support and Moderation Lifecycle Operations

## Scope

This runbook covers Platform-owned support tickets, player/content/guild reports, moderator queues, account-visible enforcement and appeals, notification delivery state and retention. It does not authorize Canary ban mutation, file attachments, production deployment or `PRODUCTION_PROVEN` claims.

## Ownership and trust boundary

Oteryn Platform owns every support table and migration. Browser public IDs are references only; ownership and administrator authority are resolved server-side. Canary remains read-only and authoritative for game-server bans/account status.

## Configuration

`config/support.php` defines positive bounded values for ticket subject/body/open limits, report target/evidence/pending limits, enforcement reason/appeal lengths and retention periods. Invalid non-positive integer values fall back to conservative repository defaults. Production values must follow the approved deployment configuration path.

Attachments are explicitly disabled until a separate reviewed upload model covers MIME/content/size/private storage/malware/privacy requirements.

## Migration

Apply the additive Platform migration before enabling routes:

```bash
php artisan migrate --force
```

The migration creates only Platform support tables and permissions. It does not migrate Canary or login-server schema.

## Permissions

Administrator actions require authentication, confirmed MFA and exactly one applicable permission:

- `support.tickets.manage`;
- `support.reports.manage`;
- `support.enforcement.manage`.

Do not grant wildcard authority or substitute MFA for authorization.

## Retention

Preview configured retention without mutation:

```bash
php artisan support:prune-retention --dry-run
```

Apply retention:

```bash
php artisan support:prune-retention
```

The command deletes eligible old closed tickets/reports and anonymizes eligible expired enforcement content. Never replace it with direct SQL. Review configuration, backup posture and current lifecycle records before scheduling it in production.

## Notifications

Ticket replies/statuses, report outcomes and enforcement/appeal changes persist a pending delivery record and attempt mail after the domain transaction commits. Sent and failed states are durable. Mail failure must not be interpreted as domain rollback.

Restore mail transport, inspect only bounded error codes and use the supported delivery service for a future retry mechanism. Never log message bodies, credentials or complete addresses.

## Privacy and audit

User surfaces expose only owner-approved fields. Internal ticket notes, moderator report notes, reporter identity outside the owner context and administrator audit data stay restricted.

Audit metadata may contain public IDs, categories, status, booleans and lock versions. It must not contain ticket messages, report evidence, appeal text, moderator notes, credentials, session/token data or complete source addresses.

## Failure handling

- stale lock/version: reload the current record; do not overwrite;
- rate/pending/open limit: preserve the existing state and retry only after the configured window or state resolution;
- notification failure: keep the committed domain state and investigate mail separately;
- database unavailable: mutations fail closed; restore the Platform database and verify owner/RBAC boundaries before reopening;
- suspected privacy disclosure: disable affected routes, preserve non-secret evidence and review object-level authorization and audit output;
- requested Canary enforcement: stop and require a separate approved cross-repository contract.

## Validation before release

```bash
composer validate --strict
composer audit
vendor/bin/pint --test
vendor/bin/phpstan analyse --no-progress
php artisan test
npm --prefix scripts/acceptance run test:coverage-contract:strict
npx playwright test --config=scripts/acceptance/playwright.support-moderation.config.mjs
```

The focused browser matrix must pass with retries set to zero across desktop, tablet and mobile, both supported locales, user and moderator flows, IDOR denial, MFA/RBAC denial, notification state and responsive overflow checks.

Green repository or isolated staging-like evidence does not establish production deployment.
