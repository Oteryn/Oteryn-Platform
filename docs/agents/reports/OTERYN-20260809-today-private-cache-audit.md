# Continuous Audit Report — Today Personalized Cache Isolation

## Scope

Audited the accepted WWW Platform `Today` / command-centre composition architecture on protected `main@1e00d6de235588f8314ec8dae8c4bdb63e5068f9`.

Primary evidence:

- `docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md`;
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`;
- `docs/architecture/SECURITY_ARCHITECTURE.md` as the global security-baseline falsification check;
- PR #933 review history;
- live active-task, open-PR and audit-repair ownership state.

The auditor does not implement the finding. Issue #941 owns architecture remediation.

## Delivery/security matrix

| Concern | State | Evidence / disposition |
|---|---|---|
| Today ownership | present | PublicPortal owns composition/presentation, not source facts. |
| Public sources | present | CMS/editorial, LiveOps current state and PublicGameData public projections remain source-owned. |
| Private source | present | Authenticated PlayerCompanion routines/goals/tracked signals are owner-private. |
| Guest behavior | present | Personalized cards are omitted rather than inferred for guests. |
| Source privacy | present | Tracking cannot bypass underlying public/authorized projection privacy. |
| Source freshness/unavailable semantics | present | Composition preserves owner semantics; missing/stale evidence cannot become fabricated state. |
| Global deny/session baseline | present but insufficient for representation caching | `SECURITY_ARCHITECTURE.md` requires fail-closed authorization, session invalidation before protected controllers and server-side privacy, but does not classify mixed personalized responses as private/non-share-cacheable or constrain replay from shared response/CDN/proxy caches. |
| Mixed response privacy classification | absent | No rule states that a response containing any owner-private card is itself private/personalized for caching purposes. |
| Shared page/fragment cache boundary | absent | No rule forbids owner-private mixed output from shared/public response/fragment caches. |
| CDN/proxy behavior | absent | No bypass/private/no-store or equivalent requirement is defined for personalized variants. |
| Guest/authenticated cache separation | absent | Guest omission is a composition rule, not a cache-key/replay invariant. |
| Cross-user private cache identity | absent | No owner identity / authz / privacy revision requirements exist if a private server cache is later adopted. |
| Logout/session/privacy transition fencing | absent | No rule says materialized private representations become unusable after security-context changes. |
| Runtime Today implementation | not delivered | ADR 0032 explicitly excludes route/UI/cache implementation. |
| Runtime/browser E2E for this audit | NOT_APPLICABLE | Audit is documentation-only; no Today executable behavior exists to exercise. |

## Negative-path falsification

### Cross-user replay

1. User A authenticates.
2. PublicPortal composes public cards plus A's owner-private routines/goals/tracked signals.
3. The full response or a mixed fragment is cached by route/query/world/profile or another identity that does not include A's security context.
4. User B requests the same Today surface.
5. A shared cache answers without executing PlayerCompanion owner authorization again.
6. User B receives A's previously authorized private representation.

The accepted contract proves that the composition service should omit A's private cards for B, but it does not prove that a cache hit must execute that composition service or that A's representation was ineligible for shared caching.

The global security baseline does not close this path. Its fail-closed session/authorization rules apply before protected controller execution, while a reverse proxy, CDN or application response cache may replay an already materialized representation without reaching that controller unless cacheability/isolation is separately constrained.

### Authenticated-to-guest replay

1. User A receives a personalized Today response.
2. A shared response/fragment cache stores it.
3. A logs out, the session expires, or an anonymous browser later uses the same route/cache identity.
4. The cached representation is replayed.

“Guests omit personalized cards” does not by itself fence a representation that was materialized while authenticated. Likewise, the global rule that a revoked/expired session is invalidated before a protected controller executes does not constrain a shared cache hit that bypasses protected-controller composition.

### Privacy/authorization tightening

1. A personalized representation is materialized while an owner-private signal is authorized.
2. Account/character ownership, tracking preferences, privacy or authorization narrows.
3. An older private representation remains cached.
4. A future request reuses it without proving the new security context.

No architecture rule currently defines which revision/generation must fence such private view-model reuse.

## Finding

### OPA-SEC-0006 — Personalized Today composition lacks private-cache isolation contract

- **severity:** high
- **confidence:** high
- **evidence_state:** PROVEN
- **finding Issue:** #941
- **disposition:** open
- **affected future surfaces:** PublicPortal Today/command-centre page or fragments, PlayerCompanion owner-private cards, reverse-proxy/CDN/page caches, future PlatformAPI reuse if the same composed representation is exposed

#### Expected architecture

Any representation containing owner-private PlayerCompanion data is treated as private/personalized regardless of how many public cards it also contains. It is not eligible for a shared/public cache. Guest/authenticated and different-owner variants cannot alias. If a private server cache exists, it is keyed and fenced by the authenticated owner and relevant authorization/privacy/applicability revisions. Logout/session replacement/ownership or privacy tightening prevents stale private representation reuse.

Public sub-fragments may still be cached independently only when the cacheable fragment contract cannot capture, vary on or be polluted by owner-private state.

#### Actual architecture

The Today/PlayerCompanion contracts define source ownership, source privacy, guest omission and authenticated owner access but do not define representation caching behavior after public and private content are composed together. The global `SECURITY_ARCHITECTURE.md` adds deny-by-default authorization and session-transition requirements, but it likewise does not define personalized response cacheability, shared-cache/CDN isolation, owner-scoped private cache identity or stale representation fencing.

#### Impact

A later Today implementation could pass all authorization checks when generating a personalized view, yet disclose routines, goals, tracked entities or derived signals across users or to guests through a shared cache replay. The risk is architectural and future-facing; no current Today runtime leak is claimed.

## Duplicate / overlap analysis

Open and closed Issue searches covered:

- Today / command-centre personalization;
- owner-private PublicPortal cache behavior;
- authenticated versus guest cache isolation;
- personalized response/shared-cache leakage;
- `OPA-SEC-0006`.

No pre-existing Issue owned the same root cause.

Related but distinct:

- **OPA-SEC-0005 / Issue #938:** federated public-search publication/revocation ordering across derived search index/result caches. It owns ADR 0033/federated-search paths and remains independent.
- **PR #933 / ADR 0032:** accepted the Today/tracking ownership architecture. Its material review repaired durable ADR authority and inspected ownership/privacy/freshness, but did not define mixed personalized response-cache isolation.
- **`SECURITY_ARCHITECTURE.md`:** global deny/session/privacy controls are relevant defense in depth but do not own or specify the missing mixed personalized response-cache invariant.
- **blocked public-domain and native-auth tasks:** no overlap with the three #941 architecture paths.

## Remediation boundary

Issue #941 exclusively owns:

- `docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md`;
- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`.

Required repair contract includes:

1. private/personalized classification for any mixed representation containing owner-private cards;
2. prohibition of shared/CDN/proxy caching for the private variant or equivalent safe mechanism;
3. deterministic guest/authenticated and cross-owner cache separation;
4. strong owner/security-context keying if private server-side caching is adopted;
5. logout/session/ownership/privacy/authorization transition fencing;
6. safe independent caching rules for truly public sub-fragments;
7. negative validation for two users, guest/auth transitions, stale private fragments and proxy/CDN simulation.

The audit does not authorize Today runtime, cache middleware, routes, schemas, tests, deployment, production or external-repository work.

## Validation

- protected-main / ownership preflight: PASS;
- primary Today architecture negative-path review: PASS;
- global `SECURITY_ARCHITECTURE.md` falsification cross-check: PASS — general fail-closed/session rules exist, but no personalized response cache-isolation rule closes the reproduced path;
- PR #933 review-history inspection: PASS;
- open/closed duplicate search: PASS;
- deterministic `repair/issue-941` branch availability checked before `agent:ready`: PASS;
- current runtime leak claim: REJECTED;
- runtime/browser E2E for audit-document deliverable: **NOT_APPLICABLE**;
- exact-head audit PR CI/review hygiene: pending final validation generation.

## Conclusion

ADR 0032 correctly prevents PublicPortal from becoming a new data authority and correctly labels PlayerCompanion tracking as owner-private, while the global security architecture correctly establishes fail-closed authorization/session principles. Those controls still stop short of defining how an already-materialized mixed private/public representation may be cached and replayed. Before a personalized Today route or public-portal page/fragment caching is implemented, the architecture must explicitly fence the materialized private representation itself. OPA-SEC-0006 / Issue #941 is the single material finding for this audit package.
