# ADR 0033: Federated content search and discoverability ownership

- Status: Accepted
- Date: 2026-08-09
- Decision record: Issue #935 / task `OTERYN-20260809-federated-search-architecture`; security refinement Issue #938 / `OPA-SEC-0005`
- Related: ADR 0022, ADR 0025, ADR 0032

## Context

Oteryn already has several module-owned public search/read capabilities, including Wiki search and PublicGameData character lookup. `PORTAL_COMPLETENESS_ARCHITECTURE.md` nevertheless retains federated content search as a material discovery gap because the portal does not yet have a durable cross-module ownership contract for one first-party search experience across CMS/news, announcements/events, Wiki, GameCatalog and later explicitly public player-tool references.

A global search feature creates architecture risk if it becomes:

- a second source of truth;
- a global database/model access layer;
- a reason for source modules to depend back on presentation code;
- a private-content enumeration path;
- an independent ranking/business-logic implementation duplicated again in PlatformAPI;
- an infrastructure-first commitment to a search engine before need is measured.

The current repository also contains a pre-existing compatibility reverse edge: Announcements and Events homepage provider/view-model paths import `App\PublicPortal\PublicContentState`. This predates ADR 0033. Adding PublicPortal -> Announcements/Events federated-search calls without first removing that reverse edge would create a bidirectional module dependency.

Search caching also has a privacy/correctness constraint: distinct semantic response-shaping requests must never collide into one cached response merely because some dimensions match, while raw query or opaque cursor material must not be persisted as cache-key/log/metric data. Query text alone is not sufficient cache identity because pagination, filters, provider selection, limit, ranking-policy version and source/index generations can all change the returned slice. A plain unkeyed hash is also insufficient privacy protection for common dictionary-recoverable search terms.

A later independent security audit (`OPA-SEC-0005`, Issue #938) proved a separate publication-control ambiguity. Ordinary stale-index/cache tolerance can coexist with a newer source-level unpublish/revoke/delete/incompatibility decision unless the architecture explicitly orders publication decisions and says which one wins while tombstone/invalidation propagation is delayed, failed or ambiguous. Cache TTL, source revision and index generation alone do not prove that an older public representation remains authorized after a newer restrictive decision.

The decision must therefore preserve the repository's minimum-module rule while allowing later implementation and API reuse **without hiding current dependency debt, leaking request material through cache identity or allowing ordinary stale-serving rules to override a newer restrictive publication decision**.

## Decision

### 1. Federated public content search belongs to `PublicPortal`

`PublicPortal` owns federated search as an application-level discoverability capability.

It owns:

- the public federated query contract;
- provider orchestration and bounded timeout/failure policy;
- deterministic provider grouping/interleaving policy;
- the normalized public result envelope;
- partial-failure/unavailable presentation semantics;
- web search UX, SEO behavior and public rate/abuse policy.

This does **not** create a new top-level `Search` or `Discovery` module and does not make PublicPortal authoritative for source content.

### 2. Source modules remain authoritative for eligibility and source-local search

Participating source modules remain responsible for their own:

- public/publication/visibility eligibility;
- locale-specific search semantics;
- source-local relevance ordering;
- canonical source identity/URL semantics;
- revisions, applicability and freshness where relevant.

PublicPortal uses provider adapters that call bounded source-module application/query interfaces and normalize only public-safe metadata.

The **target federated-search dependency direction** is:

```text
PublicPortal -> CMS / Announcements / Events / Wiki / GameCatalog / explicitly adopted public providers
```

Source modules must not depend on PublicPortal search contracts, normalized search-result types or search/presentation view models, and federated search must not use raw cross-module model/table access.

This is a target dependency direction, not a false claim about every current composition path. Before `Announcements` or `Events` becomes a federated-search provider, the existing reverse imports of `PublicPortal\PublicContentState` must be removed. The preferred bounded repair is for the source module to return a source-owned application response/availability type which PublicPortal maps into its own composition/search presentation state. ADR 0033 does not authorize a generic shared dumping-ground module merely to relocate the enum.

The implementation must not add a new PublicPortal -> Announcements/Events search edge while the corresponding Announcements/Events -> PublicPortal compatibility edge remains.

### 3. Initial provider scope is public first-party content

The accepted provider families are:

- published CMS news/pages;
- eligible public Announcements **after its reverse-edge onboarding prerequisite is complete**;
- eligible public Events **after its reverse-edge onboarding prerequisite is complete**;
- published localized Wiki content;
- active/verified/public-safe GameCatalog entities.

`PlayerCompanion` private records are excluded. Later deliberately public/shareable artefacts may participate only after a separate public-indexability/revocation/abuse contract.

Support, Admin, Audit, Identity, Accounts and other private records are excluded from public federated search.

### 4. Character search remains a separate PublicGameData search product

PublicGameData exact-name character search keeps its existing privacy/enumeration semantics. It is not silently converted into fuzzy people discovery and is not blended into generic content relevance.

A later web surface may offer a clearly separate Character vertical/handoff, but any broader character-search semantics require their own bounded decision.

Marketplace search/filtering likewise remains Marketplace-owned because auction state, availability and sorting policy are materially different from editorial/content discovery.

### 5. Raw relevance scores are not treated as globally comparable

Each provider owns source-local ordering.

The first implementation should prefer grouped verticals with deterministic source ordering and per-source budgets. If an `All` view interleaves sources, PublicPortal uses a versioned deterministic policy based on rank position/source quotas rather than directly sorting unrelated provider scores as though they have common meaning.

Future weights, freshness boosts or personalization require explicit product policy. Hidden behavioral personalization is not accepted by this ADR.

### 6. Publication, localization and privacy are enforced before federation

Provider queries are public-by-construction. PublicPortal must not request broad private/draft datasets and attempt to sanitize them after retrieval.

The normalized result preserves:

- source module/type/public-safe identity;
- locale;
- title and safe snippet;
- canonical URL;
- source revision where relevant;
- provider rank;
- published/effective metadata where relevant;
- game-profile/ruleset/world/season/freshness applicability where relevant.

No account/session/security identifiers or raw source models are part of the generic result contract.

### 7. Partial failure is distinct from zero results

Federated search distinguishes at least:

- complete result;
- partial result with one or more unavailable providers;
- all-providers unavailable;
- invalid query.

A provider outage cannot be silently represented as an empty healthy result. Internal errors/hostnames/stack traces are not exposed to users.

### 8. A dedicated search index is optional and derived

The first delivery should use bounded fan-out over module-owned public application queries unless measured evidence demonstrates that a dedicated index is required.

If an index/search engine is later adopted:

- it is rebuildable derived state, never source truth;
- indexed documents carry source identity/revision/locale and index generation;
- only source-authorized public fields enter the index;
- unpublish/revoke/delete is propagated deterministically;
- rebuild/cutover uses explicit generations;
- stale-index/lag behavior is visible and bounded;
- replacing the search engine does not change canonical source identities.

No specific Elasticsearch/OpenSearch/Meilisearch dependency is accepted by this ADR.

### 9. Newer restrictive publication decisions fence every older derived representation

Publication/visibility decision freshness is distinct from ordinary source/content/index freshness.

Each participating source remains the authority for whether a record is searchable and must provide a monotonic or equivalently strong ordering proof for publication/visibility decisions whenever search can serve a materialized representation after the source query that authorized it. The exact field name is source-owned; conceptually it behaves like a `publication_decision_revision` or restrictive-decision watermark.

The accepted invariant is:

```text
newer restrictive source decision
> older public/allow representation
```

Once a source records a newer unpublish, revoke, delete, moderation/legal removal, incompatibility or equivalent restrictive decision:

- older direct-provider materializations, derived-index documents, result-cache entries, web responses and future PlatformAPI responses become unservable for the affected object/fields even when ordinary cache TTL or tolerated index lag remains;
- every derived representation is bound to the publication-decision evidence under which it was allowed, and out-of-order older allow/update events cannot regress an accepted newer restrictive decision;
- failed, delayed or ambiguous tombstone/index/cache propagation cannot be treated as successful revocation; the affected representation fails closed until the newer decision is proven effective for that delivery path;
- physical deletion/eviction may occur later, but the visibility fence must already prevent delivery;
- publication-authority unavailability is distinguished from ordinary stale-content/provider failure. A stale allow may be reused only while a source-owned bounded publication proof/lease remains valid under an accepted contract; if continuing authority cannot be proven, the affected result is unavailable rather than public-by-default;
- a rebuild, cutover or rollback cannot activate an index generation whose publication-decision watermark would move behind a newer accepted restrictive decision. Affected content remains fenced until a compatible generation is rebuilt/reconciled;
- ordinary source revision, index generation, cache generation and TTL may satisfy this fence only if the source contract proves the same ordered value advances for every restrictive decision and every delivery path checks it.

This rule does not make PublicPortal the publication authority. PublicPortal consumes source-owned decision evidence and enforces the source's restrictive decision across federation-derived representations.

### 10. Query privacy, cache identity, security and abuse are first-class

Search queries and opaque pagination material may contain personal, sensitive or high-cardinality data.

Therefore:

- raw queries and raw opaque cursors are not metric labels and are not written to ordinary structured logs by default;
- paginated-result cache correctness is bound to the **entire semantic response-shaping request**, not only the normalized query;
- a versioned canonical cache-input structure includes normalized query, explicit locale, canonicalized provider/type filters, effective provider set, pagination mode and validated page/opaque cursor value, limit, ranking/grouping policy version and the provider/source/index generation vector required to prevent stale cross-generation reuse;
- the externally stored cache identity is a **versioned server-keyed digest** of that canonical structure, such as `HMAC-SHA-256(cache-key-secret, canonical-cache-input)`, not raw request text and not an unkeyed/plain hash vulnerable to practical dictionary recovery;
- semantically different query/page/cursor/limit/filter/provider/ranking-policy/generation inputs cannot intentionally share one paginated `SearchResponse` cache entry;
- the cache-key secret is managed as application secret material; the key identifier/version participates in the cache namespace so rotation produces a distinct cache generation;
- cache request digests are not emitted as ordinary logs or metric labels and are not reused as analytics/user-tracking identifiers;
- a future pre-pagination cache is a distinct layer and may exclude pagination only if it caches a complete object before slicing and proves slicing cannot change source/ranking semantics;
- cache TTL/hit validity never overrides the restrictive publication-decision fence in Decision 9;
- query length, cursor size, page size and filters are bounded;
- provider/type filters are server-side allowlists;
- arbitrary SQL/search-engine query languages and client-selected fields/sorts are rejected;
- titles/snippets/highlights are rendered safely;
- anonymous rate limits and payload/response limits are required;
- any future people-search composition retains a separate anti-enumeration policy.

### 11. Future `PlatformAPI` reuses the same application service

If PlatformAPI exposes the same first-party federated-search product, it adapts the `PublicPortal` federated-search application service.

PlatformAPI must not independently fan out to providers or recreate ranking/grouping/publication logic. Transport/version/pagination details may differ, but provider ownership, restrictive publication fencing and result/failure semantics remain shared.

A future materially different non-portal discovery product may justify extraction into a shared module only after concrete consumers prove that need.

## Consequences

### Positive

- no new deployable module or infrastructure dependency is required;
- source publication/privacy/localization remains authoritative;
- PublicPortal's existing SEO/discoverability/composition responsibility gains a precise search contract;
- PlatformAPI can reuse one orchestration path;
- exact-name character search is protected from accidental fuzzy enumeration;
- a later search engine remains replaceable derived infrastructure;
- existing dependency debt is explicit and cannot be accidentally deepened by provider onboarding;
- cache correctness distinguishes every response-shaping request without storing or exposing raw query/cursor material as cache identity;
- a newer restrictive publication decision deterministically overrides ordinary stale-index/cache tolerance across every federated-search delivery path;
- revocation safety does not depend on successful physical cache eviction or search-engine tombstone deletion before the public cutoff becomes effective.

### Costs

- PublicPortal provider adapters must normalize heterogeneous source result shapes;
- grouped search may initially feel less “globally ranked” than one blended score;
- partial dependency failure must be designed and tested explicitly;
- Announcements/Events require a bounded compatibility cleanup before federated-search onboarding;
- cache implementations require canonical request serialization plus managed keyed-digest secret/version rotation semantics;
- future dedicated indexing requires revision/tombstone/generation contracts rather than a simple bulk copy;
- source providers/indexers must expose or consume ordered publication-decision evidence and distinguish ordinary freshness failure from inability to prove current visibility authority;
- rebuild/rollback and outage paths require explicit restrictive-watermark validation instead of assuming a previously public generation remains safe.

## Rejected alternatives

### Create a standalone Search microservice now

Rejected. No measured scaling/lifecycle/ownership need currently justifies the operational and consistency cost.

### Create a top-level `Discovery` module now

Rejected. The current requirement is specifically public portal discoverability, already inside PublicPortal's responsibility. Extraction remains possible if materially different consumers later require independent domain ownership.

### Let every consumer fan out independently

Rejected. Blade/web, PlatformAPI and future clients would duplicate source selection, ranking, failure and privacy semantics.

### Build a global index from raw source tables

Rejected. It weakens module boundaries, publication/privacy controls and provenance, and risks turning derived state into hidden authority.

### Blend character search into fuzzy public content search

Rejected. PublicGameData character search has a distinct privacy/enumeration contract and remains separate.

### Mandate an external search engine for v1

Rejected. Architecture should first use existing module application queries and add derived search infrastructure only when evidence requires it.

### Ignore the existing Announcements/Events -> PublicPortal edge

Rejected. Adding the opposite federated-search dependency while that compatibility edge remains would create a cycle and contradict the intended modular direction.

### Move `PublicContentState` into an unowned generic shared module by default

Rejected. A neutral boundary needs genuine ownership and semantics; generic shared-code extraction is not a substitute for source-owned application responses.

### Use raw query or opaque cursor material in result-cache keys

Rejected. Search/pagination inputs can contain personal or sensitive text and should not be persisted as direct cache-key material.

### Use a plain/unkeyed hash of request material as privacy protection

Rejected. Common search terms are dictionary-recoverable; a server-keyed versioned digest of the full canonical response-shaping request provides a stronger privacy boundary while retaining deterministic cache isolation.

### Cache a paginated response without page/cursor/limit identity

Rejected. Different response slices could collide and return the wrong page or limit. Pagination may be excluded only by a separately defined pre-pagination cache that slices strictly after retrieval.

### Treat bounded stale-index/cache TTL as permission to serve after revoke

Rejected. Ordinary freshness tolerance cannot authorize an older allow after the source has accepted a newer restrictive publication decision.

### Depend on best-effort cache eviction/tombstone deletion as the revocation cutoff

Rejected. Physical invalidation can fail or be delayed. The visibility fence must make affected older representations unservable before cleanup is considered complete.

## Implementation and activation limits

This ADR defines architecture only. It does not authorize:

- a new route/controller/UI;
- schema or index creation;
- external search-engine deployment;
- production activation;
- private-content indexing;
- server/client repository access or mutation.

Implementation requires a separate bounded task with exact source contracts, dependency-cycle cleanup for every selected provider that currently imports PublicPortal, full semantic cache-identity/privacy and key-rotation tests, restrictive publication-decision ordering/failure/rollback tests, state/error coverage, security/rate-limit validation, localization, accessibility/responsive behavior and real exact-head E2E.

At minimum, restrictive-publication validation must cover out-of-order allow/revoke events, tombstone propagation failure, result-cache/index lag, concurrent refresh versus revoke, publication-authority outage/expired visibility proof and rollback to an older index generation after a newer revoke. The repair does not claim a current runtime disclosure because no federated-search index/cache runtime is delivered by this ADR.

## Focused architecture

Detailed provider, result, ranking, failure, cache/index, privacy, restrictive-publication-fence, observability, SEO, dependency-cleanup and PlatformAPI rules are in:

- `docs/architecture/FEDERATED_SEARCH_ARCHITECTURE.md`