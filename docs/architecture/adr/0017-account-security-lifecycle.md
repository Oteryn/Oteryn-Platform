# ADR 0017: Platform account security lifecycle

- Status: Accepted
- Date: 2026-07-28
- Issue: #276

## Context

Oteryn Platform already owns web credentials, password recovery/change, TOTP MFA with recovery codes, revocable web-session generations and a one-to-one immutable binding to a greenfield Canary account. The benchmark audit in Issue #268 identified missing account-management capabilities: confirmed email change, user-visible session management, privacy/status controls, account termination and a separately issued high-assurance recovery artifact.

The current Laravel session backend is configurable and may be file, database or another supported driver. Targeted revocation therefore cannot safely depend on deleting one backend-specific session row. Canary account ownership is outside the Platform database and no unlink, rebind or account-deletion write contract is authorized.

## Decision

### Platform-owned web-session registry

Oteryn will add an `identity_web_sessions` registry independent of Laravel's configured session backend.

Each successful non-MFA or completed-MFA login creates one random UUID registry row after Laravel regenerates the session ID. The authenticated session stores only that UUID and the current identity generation. The registry stores:

- the owning Platform Identity;
- the identity web-session generation at issue time;
- bounded user-agent family text;
- an HMAC of the source IP rather than the raw address;
- issued, last-seen, expiry and revocation timestamps.

`EnsureIdentitySessionIsCurrent` must verify the current database identity and registry row on every authenticated request. A missing, foreign, expired, generation-stale or revoked row fails closed and invalidates the Laravel session. Last-seen writes are throttled. Targeted revocation marks the registry row revoked; backend session data is not trusted as the revocation authority.

Global revocation retains the existing generation increment and also marks every active registry row revoked. This preserves backend-independent password/MFA reset semantics.

### Confirmed primary-email change

Email change is a Platform-only transaction:

1. an authenticated enabled identity submits the canonical new address and current password;
2. the server enforces uniqueness, one pending request, rate limits and a post-change cooldown;
3. random verification and recovery/cancellation tokens are stored only as SHA-256 verifiers;
4. the new address receives a verification link and the old address receives a notice/recovery link;
5. verification locks both the request and identity, re-checks uniqueness and expiry, updates the canonical email, and revokes web sessions and game authorizations;
6. the old-address recovery token may cancel a pending request or revert a completed change during a bounded recovery window, subject to a fresh uniqueness check and another global revocation.

Tokens are single-use, never logged, and are removed or invalidated after terminal use.

### High-assurance recovery key

An authenticated identity may generate or rotate one recovery key. The raw key is displayed once. Only an application-keyed HMAC verifier is stored. Rotation and revocation require the current password. Public recovery requires canonical email, the complete key and a new password under strict rate limits.

A successful recovery consumes the key exactly once, resets the password, clears TOTP and MFA recovery codes, revokes all Platform web sessions and game authorizations, deletes password-reset tokens, and records bounded security events without key material.

The recovery key is not sold and is not a payment product.

### Privacy and status controls

Platform Identity owns conservative booleans for public account association and public online/status visibility. Defaults are private. Current public profile consumers may ignore them until the character-profile work in Issue #277, but future consumers must fail closed and read these server-side values rather than accept browser claims.

### Account termination

Termination is a Platform lifecycle, not a Canary deletion:

- request requires current password and an explicit confirmation phrase;
- a configurable grace period is stored on the identity;
- the user may cancel before finalization with current-password verification;
- request and finalization both fail closed when the identity owns or leads a non-terminal Character Bazaar operation, has reserved wallet funds, or has a pending email change;
- a scheduled idempotent command finalizes due requests under row locks;
- finalization disables Platform login, anonymizes the primary email, replaces the password, clears MFA and recovery-key material, revokes web/game authorizations and password-reset tokens, and preserves the Identity ID, Canary binding and audit records;
- Canary accounts and characters are neither deleted nor reassigned.

The old canonical email is retained only as an application-keyed hash for security and duplicate-abuse investigation; it is not exposed to users or ordinary administrators.

### Canary binding policy

Self-service unlink or rebind is not adopted. The immutable one-to-one binding remains the security boundary. Browser requests cannot name a Canary account as ownership proof. Any future exceptional rebind requires a separate support/moderation workflow, explicit Canary ownership evidence, least-privilege mutation contract, MFA/permission guard, audit, deterministic locking and rollout approval.

The account center will expose the binding as locked by policy rather than presenting a nonfunctional action.

### Email-code MFA policy

Email-code MFA is not adopted. Email is already the primary recovery channel; treating access to the same mailbox as a normal second factor would weaken factor independence and create downgrade ambiguity. TOTP plus single-use MFA recovery codes remains the supported web MFA method. This decision is machine-visible in configuration and documentation.

## Consequences

- Targeted revocation works with file, database or supported remote Laravel session drivers.
- Every authenticated request adds one indexed Platform database session-registry read; last-seen writes are bounded.
- Email change and recovery-key use revoke current sessions, so the user must sign in again.
- Termination is reversible only during the grace period and does not remove game data.
- Product benchmark capabilities for email change, session management, privacy, termination and recovery key may move to implemented after exact-head evidence passes.
- Connected-account rebind and email-code MFA may be classified as not applicable only with this explicit durable rationale.

## Nonclaims

This ADR does not authorize Canary repository writes, production deployment, direct production verification, irreversible game-account deletion or transfer of game ownership.
