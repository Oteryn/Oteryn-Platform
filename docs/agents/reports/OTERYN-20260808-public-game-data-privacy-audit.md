# Native PublicGameData privacy audit — 2026-08-08

## Result

`AUDIT_COMPLETE_WITH_FINDINGS`

Audited the native PublicGameData projection contract delivered by Issue #902 / PR #903 on protected `main@bb51c0329b8907502ea1162ff632df7ba968855d`.

One material security-contract gap was proven and routed to `OPA-SEC-0004` / Issue #908. No contract repair, runtime, schema, cache/CDN, deployment, credential, production or external-repository mutation was performed by the audit role.

## What the contract already gets right

The accepted contract correctly separates Oteryn-v2 game facts from Platform presentation policy and establishes that:

- game-originated `public=true` cannot override CharacterProfiles/Identity privacy;
- Platform privacy is an independent upper bound over accepted game facts;
- native public identity must use canonical IDs rather than Canary numeric IDs;
- privacy changes must invalidate/rebuild affected public presentation/cache/search state;
- ordinary HTTP/API/SSR reads should consume Platform projections instead of querying game runtime synchronously;
- stale-but-still-safe game-source evidence may use a bounded last-known-good path;
- hard-expired/invalid source evidence must not masquerade as empty/not-found/zero;
- HTTP/API/SSR/cache/CDN are all part of the public delivery path;
- privacy denies must remain effective over fresh game facts and after cache/search refresh.

## Finding OPA-SEC-0004

**Issue:** #908  
**Severity:** HIGH  
**Priority:** P1  
**Confidence:** HIGH  
**Evidence:** PROVEN

The contract does not close the authorization-ordering boundary between resilient game-source stale serving and restrictive Platform privacy changes.

Missing semantics include:

1. **Privacy decision ordering.** There is no monotonic privacy revision/generation/watermark or equivalent proof associated with a public variant.
2. **Revocation cutoff.** The contract does not state exactly when a newer restrictive preference makes an already cached/indexed/CDN allow variant invalid.
3. **Failed/ambiguous invalidation.** It says privacy changes must invalidate/rebuild, but does not state what happens when purge/rebuild is delayed, partially applied or fails.
4. **Privacy dependency outage.** Game-source unavailability is carefully modeled; privacy-policy/store unavailability is not separately modeled, so an implementation could reuse an older cached allow without proving it is still authorized.
5. **Rollback safety.** Projection generation rollback rules do not state that rollback may never cross a newer privacy deny and resurrect an older public representation.
6. **Acceptance tests.** The validation matrix does not explicitly require concurrent privacy change versus projection refresh, delayed/out-of-order privacy events, purge failure, CDN/search propagation lag, privacy dependency outage or rollback after deny.

### Why this matters

The contract deliberately permits last-known-good public content while game-source facts are stale but within an accepted stale window. That is a sound availability design for game facts. Privacy authorization is different: a user or policy may withdraw permission now.

Without an independent privacy freshness/revision rule, a future implementation could:

- cache a composed public profile while fields are allowed;
- accept a newer `show_* = false` privacy change;
- fail or delay search/CDN/cache invalidation;
- continue serving the old representation because the underlying game projection is still inside stale-while-servable policy.

That sequence would comply with the currently specified game-source freshness model while violating the intended privacy upper bound.

## Current runtime disposition

This audit does **not** claim the current Canary-compatible runtime leaks hidden fields.

Current `PublicCharacterProfileService` composes public output while reading `CharacterProfilePreference` state directly. `CharacterProfilePreferenceService` updates preferences transactionally and records a preferences-updated event. There is no native PublicGameData projection/cache implementation in this audit scope.

Therefore the finding is architectural and pre-cutover: resolve the native contract before introducing public projection caching/search/CDN behavior that can outlive a privacy decision.

## Duplicate/ownership disposition

- Issue #902 delivered the current contract and is closed.
- #486 contains broad identity/account/character capability/evidence gaps, including privacy, but does not define native PublicGameData privacy ordering.
- #487 contains broad public-portal evidence gaps but likewise does not own this security boundary.
- Issue #905 owns stale continuous-audit programme state, not PublicGameData semantics.
- No open Issue owned this exact native privacy-revocation contract before #908.

## Required remediation direction

The repair should remain technology-neutral but require an equivalently strong mechanism to:

- order privacy decisions monotonically;
- make a newer deny authoritative over any older allow representation;
- define fail-closed behavior for ambiguous/failed invalidation;
- distinguish privacy-policy unavailability from game-source unavailability;
- prevent rollback/rebuild from crossing a newer deny;
- prove those properties across HTTP/API/SSR/search/cache/CDN.

The audit does not choose Redis, tags, surrogate keys, event transport, CDN provider, cache technology or the exact revision storage representation.

## Validation disposition

- Runtime/application build: `NOT_APPLICABLE` — no executable behavior changed.
- Browser/runtime E2E: `NOT_APPLICABLE`.
- External repositories: no mutation.
- Required final evidence: exact-head Agent Governance, repository-selected CI, bounded diff review, zero unresolved review threads, merge and lifecycle closeout.
