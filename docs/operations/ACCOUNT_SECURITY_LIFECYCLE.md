# Account Security Lifecycle Operations

## Scope

This runbook covers the Platform-owned account-security lifecycle delivered by PR #283:

- confirmed primary-email change with old-address cancellation and recovery;
- registered browser-session inventory and targeted or global revocation;
- account-level privacy controls;
- a single active high-assurance recovery key stored only as a keyed verifier;
- bounded account-termination request, grace period, cancellation and finalization;
- English and Polish account-security presentation.

This runbook does not authorize Canary schema writes, Canary account deletion, character deletion or transfer, account unlink/rebind, production deployment or direct production verification.

## Ownership boundary

Oteryn Platform owns the Identity security records, migrations, web-session registry, security events, notifications, privacy flags, recovery-key verifier and termination state. The ready Platform-to-Canary account binding remains immutable for this lifecycle.

Canary remains the semantic owner of its account and character data. Finalizing Platform termination disables and anonymizes Platform login; it does not delete, unlink, rebind or transfer Canary-owned data.

## Configuration

The implementation reads bounded values from `config/identity_security.php`:

- email verification lifetime;
- old-address recovery window;
- email-change cooldown;
- registered-session lifetime and touch interval;
- recovery-key prefix;
- termination grace period and confirmation phrase;
- binding-mutation policy;
- email-code MFA policy.

Invalid or out-of-range security configuration fails closed. Production values must be injected through the approved deployment configuration path and must not be inferred from repository defaults alone.

## Database migration

Apply the Platform-owned migration before enabling the new routes:

```bash
php artisan migrate --force
```

The migration is additive for Platform-owned Identity state. It does not migrate Canary or login-server tables.

Rollback must be evaluated against live lifecycle records. Do not remove the new tables or columns while pending email changes, active recovery keys, registered sessions or termination requests still require operator visibility or recovery.

## Termination finalizer

Due termination requests are finalized through:

```bash
php artisan identity:finalize-terminations --limit=100
```

Operational requirements:

1. Run the command from the same approved release and configuration as the web application.
2. Schedule it through the production scheduler only after migration and mail delivery are verified.
3. Keep the batch limit between 1 and 1000.
4. Treat a non-zero command result as an operational failure requiring investigation.
5. Do not replace the command with direct SQL updates.

The finalizer locks each due Identity, rechecks termination eligibility, revokes Platform web sessions and game authorizations, anonymizes the Platform email and records a security event. It leaves the Canary binding and Canary-owned account and character rows unchanged.

## Email-change operations

A change request sends two separate single-use links:

- confirmation to the proposed new address;
- cancellation/recovery to the previous address.

The selected `en` or `pl` locale is carried into the notifications and token surfaces. Operators must verify that the configured mail transport can deliver both messages and that generated URLs use the intended public HTTPS origin.

Do not request, log or copy raw email-change tokens. A user reporting an invalid or expired link should start a new change after the cooldown and current account state permit it.

A confirmed change revokes all Platform web sessions and game authorizations. Recovery to the previous address repeats the revocation. This is expected behavior, not an outage.

## Registered web sessions

The account-security page displays only a bounded user-agent summary and timestamps. Raw source addresses are not shown; the registry stores a protected source-address fingerprint.

Expected actions:

- revoke one owned session;
- revoke every other owned session while retaining the current one;
- revoke the current session and redirect to login;
- reject stale, foreign or already-revoked session identifiers.

Browser-supplied session identifiers never establish ownership. Operators must not reactivate a revoked registry row manually.

## Recovery keys

Only one active recovery key exists per Identity. The raw key is displayed once and must be stored offline by the user. Oteryn persists only a keyed verifier.

Generating a new key replaces the previous verifier. Revoking or using a key makes it unusable. Successful recovery resets the password, removes MFA and revokes all Platform web sessions and game authorizations.

Support and administrators cannot retrieve or reconstruct a raw recovery key. Do not ask users to send recovery keys through tickets, chat, logs or screenshots.

## Privacy controls

The Platform owns `public_account_association` and `public_status_visible`. Both default to private. Public character/profile modules must read these flags server-side before presenting account association or status; browser input alone is not authorization.

## Audit signals

The lifecycle records bounded security events for:

- email change requested, confirmed, cancelled and recovered;
- one session revoked and all other sessions revoked;
- privacy settings updated;
- recovery key generated, revoked and used;
- termination requested, cancelled and finalized.

Audit metadata must not contain raw passwords, session IDs, reset or email tokens, MFA secrets, raw recovery keys, complete source addresses or arbitrary request bodies.

## Failure handling

### Mail unavailable

Keep the change pending only within its configured expiry. Restore the mail dependency, verify queue/transport health and let the user initiate a new request when necessary. Do not confirm a change manually in the database.

### Session registry unavailable

Protected account-security mutations must fail closed. Restore the Platform database/session dependencies and verify that stale or revoked sessions still redirect to login before reopening the surface.

### Finalizer failure

Record the command result and affected batch boundary, restore the dependency or configuration, and rerun the idempotent command. Do not mark an Identity terminated unless the supported finalizer completed its transactional state change.

### Suspected account compromise

Revoke all Platform web sessions and game authorizations through supported application actions, protect the old-address recovery channel, rotate credentials and recovery key, and review bounded security events. Canary-side incident actions require their own approved operational contract.

## Validation before release

Required repository and isolated staging-like evidence:

```bash
composer validate --strict
composer audit
vendor/bin/pint --test
vendor/bin/phpstan analyse --no-progress
php artisan test
npm --prefix scripts/acceptance run test:coverage-contract:strict
npm --prefix scripts/acceptance run test:account-lifecycle
```

The focused account-security acceptance must also pass with zero retries and cover:

- desktop, tablet and mobile without horizontal overflow;
- English and Polish surfaces;
- valid and invalid email-change links;
- multi-session inventory and revocation;
- privacy persistence;
- termination schedule and cancellation;
- recovery-key generation, revocation, use and replay denial;
- stale-session redirect before protected controller execution.

Green repository and staging-like workflows do not establish production deployment. Production verification remains separately owned by the production acceptance process.
