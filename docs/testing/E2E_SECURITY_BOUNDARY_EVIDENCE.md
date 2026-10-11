# E2E Security Boundary Evidence

## Purpose

This record captures the bounded P0 browser-visible security slice added by PR #94. It supplements the broader Functional Acceptance and E2E Coverage Roadmap without changing the evidence taxonomy or the independent Production Go-Live Gate.

## Scope

The required pull-request `critical` profile now includes two focused `@smoke @security-boundary` scenarios on the primary Chromium production-like HTTP runtime:

1. browser session rotation and observable session-cookie attributes during authentication;
2. browser/request manipulation of a submitted foreign `account_id` during character creation.

The scenarios are intentionally bounded. They do not multiply the complete secret-sensitive acceptance suite across all browser engines.

## Exact-SHA evidence

Corrected implementation head:

`4fdbe99b30c5c43c62e41405e6d98cf7d8f3b3d3`

Required workflow results on that head:

- `Acceptance E2E and Visual UX` run `29842195691` — PASS;
- `CI` run `29842193444` — PASS;
- `Agent Governance` run `29842196941` — PASS;
- `Phase 7 Production-Like Validation` run `29842195562` — PASS;
- `Platform DB Outage Validation` run `29842194228` — PASS.

Within the acceptance run:

- primary Chromium smoke — PASS, including both new security-boundary scenarios;
- bounded Chromium/Firefox/WebKit portability — PASS;
- bounded desktop/tablet/mobile responsive profile — PASS.

## Session/browser boundary

The authentication scenario proves, at the controlled acceptance HTTP boundary:

- a browser session cookie exists before authentication;
- successful authentication rotates the session-cookie value rather than retaining the pre-authentication identifier;
- the observed session cookie is `HttpOnly`;
- the observed SameSite policy is `Lax`;
- the cookie path is `/`.

The test never logs or attaches the cookie value. Raw trace, automatic screenshot and video collection remain disabled for this secret-bearing scenario.

The acceptance runtime intentionally uses plain local HTTP with `SESSION_SECURE_COOKIE=false`, so this scenario does **not** claim production `Secure` cookie behavior. Final deployed TLS/cookie behavior remains production-only evidence under issue #91 and `docs/testing/PRODUCTION_SMOKE_CHECKLIST.md`.

## Server-owned character ownership boundary

The ownership-manipulation scenario:

1. creates two disposable Platform Identities through the real acceptance registration/provisioning flow;
2. confirms each has a distinct ready legacy engine binding;
3. authenticates as the first Identity;
4. injects the second Identity's legacy engine `account_id` into the browser character-creation form as a hidden field;
5. submits the normal character-create request;
6. verifies the character is created for the authenticated Identity's server-owned binding, not for the injected foreign account.

This proves the composed browser/request boundary remains fail closed against client-selected ownership authority. Database transaction, locking, uniqueness and race correctness remain owned by deterministic integration tests rather than being duplicated here.

## First failed run and correction

Initial security-slice head `f9af89dfd017bfa0417d88503277a535e6f9b7a4` ran acceptance workflow `29841948505`.

The session-rotation scenario passed, but the ownership scenario failed before reaching the ownership assertion because it used `seed-account-overview-state.php`. That fixture can represent a Platform `ready` Account Overview state but does not provide the real legacy engine account row required by character creation. The UI correctly failed with `Your bound game account is unavailable.`

The correction replaced that fixture for the ownership scenario with the real acceptance registration/provisioning flow for both disposable identities. No application authorization, provisioning, session, rate-limit or legacy engine contract behavior was weakened or bypassed.

The corrected exact-SHA run `29842195691` passed the complete required critical profile.

## Classification and remaining work

- Browser-visible session rotation / HttpOnly / SameSite boundary: `STAGING_PROVEN` for the controlled acceptance HTTP environment.
- Browser/request foreign ownership manipulation rejection: `STAGING_PROVEN` for the controlled acceptance environment.
- Final production Secure-cookie/TLS/proxy behavior: `UNKNOWN` until direct production verification.
- Existing-data migration/upgrade/controlled rollback P0 work is split to issue #98 because it belongs with release/deployment validation rather than this bounded browser-security slice.
- Final production verification remains issue #91 and cannot be inferred from this staging evidence.
