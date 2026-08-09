# Continuous Audit Report — Federated Search Publication Revocation

## Scope

Audit subject: the accepted WWW Platform federated-search architecture delivered by Issue #935 / PR #936 and present on protected `main@af3c23943106cd10c7eea42f6644ae12e1e69990`.

Primary evidence:

- `docs/architecture/adr/0033-federated-content-search-and-discoverability.md`;
- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`;
- PR #936 complete review history and its three material repair cycles;
- Issue #935 acceptance;
- live active task, open PR and remediation-queue state.

Audit role is discovery/evidence only. No product, runtime, route, schema, index, cache implementation, workflow, deployment, production or external-repository mutation is authorized.

## Subject inventory and delivery matrix

| Layer | State | Evidence / audit disposition |
|---|---|---|
| Durable ownership | present | ADR 0033 assigns federated public content search orchestration to PublicPortal while source modules retain content/publication authority. |
| Provider boundary | present | Public-by-construction bounded source queries; private/draft broad reads are forbidden. |
| Dependency direction | present with explicit compatibility prerequisite | Announcements/Events reverse `PublicContentState` dependencies are documented and must be removed before provider onboarding. |
| Localization/source ranking/canonical URL authority | present | source-owned in ADR 0033 and focused architecture. |
| Failure semantics | present | COMPLETE/PARTIAL/UNAVAILABLE/INVALID_QUERY distinguish provider failure from healthy zero results. |
| Query privacy and abuse | present | bounded query/cursor/filter/rate behavior; raw requests excluded from ordinary logs/metric labels. |
| Paginated cache request identity | present | complete semantic response-shaping request bound through a versioned server-keyed digest after PR #936 review repairs. |
| Derived-index authority | present | index is rebuildable projection, never source truth; generation-based rebuild/cutover required. |
| Unpublish/revoke/delete propagation | partial | deterministic update/tombstone behavior is required, but restrictive-decision ordering and fail-closed propagation semantics are not defined. |
| Restrictive visibility cutoff | absent | no contract says exactly when a newer restrictive source decision makes older indexed/cached output unservable. |
| Visibility decision revision/watermark | absent | searchable/indexed representation has source revision/index generation but no required monotonic publication/visibility decision fence. |
| Tombstone/index/cache propagation acknowledgement | absent | failed/delayed/ambiguous invalidation has no explicit fail-closed affected-result behavior. |
| Publication-authority outage behavior | partial/absent | generic provider/index failure semantics exist, but unavailability of the authority needed to prove the current public decision is not separated from ordinary freshness. |
| Rebuild/rollback across newer revoke | absent | generation mechanics exist, but no rule prevents rollback to an older index generation from resurrecting a representation predating a newer restrictive decision. |
| Runtime implementation | not implemented | ADR 0033 explicitly architecture-only; no current federated-search route/index/cache runtime is claimed. |
| Product E2E | not applicable to current architecture-only delivery | no executable federated-search implementation exists to exercise. Future implementation must validate the negative paths defined by Issue #938. |

## Negative-path falsification

Consider one source record `R` with this order:

1. source publication state `P10 = public`;
2. federated index generation `I10` contains title/snippet for `R` and a result cache entry is derived from it;
3. source authority records `P11 = revoked/unpublished`;
4. tombstone/index/cache propagation is delayed, fails, or has ambiguous acknowledgement;
5. search serves from the still-existing `I10` / result cache during the architecture's tolerated bounded stale interval; or an operator rebuild/rollback switches from a later index to an older generation containing `R`.

The accepted architecture says `R` must stop appearing according to canonical source truth and that revocation propagates deterministically, but it does not define an ordered representation invariant such as `representation.publication_decision >= current_restrictive_decision`, a restrictive-decision fence checked at serve time, or equivalent proof. It also does not state that stale serving is forbidden for an affected representation once the newer restrictive decision is authoritative.

Therefore the current contract admits two simultaneously plausible implementations:

- **safe interpretation:** `P11` immediately fences every old representation; serve fails closed until affected index/cache copies prove the newer decision;
- **unsafe but contract-compatible interpretation:** `I10` remains within configured stale-index/TTL tolerance and can be served until tombstone/invalidation completes.

A security-sensitive architecture must not leave that choice implicit.

## Finding

### OPA-SEC-0005 — Federated search lacks fail-closed publication revocation fencing

- **severity:** high
- **confidence:** high
- **evidence_state:** PROVEN
- **affected modules/surfaces:** PublicPortal federated search, source-provider publication boundary, future derived index, result cache, future PlatformAPI reuse
- **disposition:** open
- **Issue:** #938

#### Exact evidence

`FEDERATED_SEARCH_ARCHITECTURE.md` requires:

- source public eligibility to remain authoritative;
- unpublished/revoked/incompatible content to stop appearing according to canonical source truth;
- future unpublish/removal/revocation to create deterministic update/tombstone behavior;
- explicit generation-based index rebuild/cutover;
- bounded/visible stale-index lag;
- bounded result caching and source revision/index invalidation.

ADR 0033 similarly requires derived indexes, deterministic unpublish/revoke/delete propagation, explicit generations and bounded stale-index lag.

What neither contract specifies is the publication-decision ordering proof that governs an already-materialized older result while the newer restrictive decision propagates.

#### Expected observable contract

Once source authority accepts a newer restrictive decision for an object, every search delivery path must have one deterministic rule that prevents older public output from remaining serveable, including direct provider composition, derived index, result cache, web response and future API reuse. Propagation failure, authority outage and rollback must not weaken that rule.

#### Actual contract

The architecture specifies desired propagation and ordinary freshness/generation mechanics but leaves the restrictive-decision cutoff, acknowledgement/failure state and rollback fence undefined.

#### Impact

A future implementation can retain a previously public title, snippet, canonical link or discoverability signal after the source has revoked publication, while still satisfying ordinary stale-index/cache limits. This can matter for accidental publication, privacy withdrawal, moderation/legal takedown, deleted content and future owner-published PlayerCompanion artefacts.

This report does **not** claim a current runtime disclosure. The federated-search implementation is not delivered yet.

#### Remediation acceptance

Issue #938 owns an architecture-only repair requiring:

1. separate publication/visibility-decision freshness from ordinary source/index freshness;
2. monotonic or equivalently strong ordering proof for restrictive source decisions;
3. explicit visibility cutoff across provider/index/cache/web/API paths;
4. fail-closed behavior or equivalent safe proof when invalidation/tombstone propagation is delayed, failed or ambiguous;
5. publication-authority outage behavior distinct from ordinary stale data;
6. rollback/rebuild fencing across a newer revoke;
7. validation scenarios for out-of-order events, tombstone failure, cache/index lag, concurrent revoke/refresh, authority outage and rollback after revoke;
8. architecture-only delivery without false claim of a current runtime leak.

## Duplicate and overlap analysis

Live duplicate searches covered open and closed Issues using combinations of federated search, revocation, unpublish, removal, stale index, tombstone, cache visibility and `OPA-SEC-0005`. No existing Issue owns this root cause.

Related but not duplicate:

- **Issue #935 / PR #936:** terminal architecture delivery that introduced ADR 0033; it does not own post-delivery audit remediation.
- **Issue #908:** terminal native PublicGameData privacy-revocation fence. It concerns a different source/product and contract, though the ordering hazard is structurally similar.
- **PR #541:** unrelated public-domain external evidence wait.
- **PR #338:** unrelated inactive Game Catalog 1.3 consumer compatibility hold.
- blocked public-domain and native-auth active tasks own no federated-search architecture path.

PR #936's review history was inspected rather than trusted by summary. Three material Codex repair cycles addressed:

1. reverse Announcements/Events -> PublicPortal dependencies;
2. missing privacy-safe query identity in result-cache keys;
3. incomplete response-shaping cache identity without pagination/limit dimensions.

No PR #936 material review addressed restrictive publication decision ordering, failed tombstone propagation or rollback across a newer revoke.

## Risk classification

`HIGH / P1 / HEIGHTENED` is justified because the missing rule sits at a future confidentiality/publication-control boundary where stale derived public data can outlive a newer deny. The issue is still architecture-only and reversible today; the high risk is specifically why it should be repaired before a derived search index or composed-result cache is implemented.

## Auditor boundaries

The audit created Issue #938 and intentionally did not edit:

- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`;
- `docs/architecture/adr/0033-federated-content-search-and-discoverability.md`;
- any application/runtime/test/schema/workflow/deployment path.

Those architecture paths belong to the independent remediation owner defined by Issue #938.

## Audit validation

- protected-main selection/ownership preflight: PASS;
- primary ADR/focused architecture inspection: PASS;
- negative-path reasoning against accepted contract: PASS;
- PR #936 review-history inspection: PASS;
- open/closed duplicate search: PASS;
- Issue taxonomy and deterministic branch availability check: PASS; `repair/issue-938` did not exist when the ready finding was published;
- runtime/browser E2E for this audit-document deliverable: **NOT_APPLICABLE** — only audit documentation changes and the audited feature has no delivered runtime;
- final exact-head repository CI: pending audit PR creation.

## Conclusion

The selected federated-search architecture is materially stronger after PR #936's dependency and cache-identity repairs, but its publication-control boundary is not yet safe enough to guide a stale derived index or result cache. OPA-SEC-0005 / Issue #938 is the single confirmed material finding for this package and is independently actionable before federated-search implementation.