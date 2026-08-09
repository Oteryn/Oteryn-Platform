# Oteryn Federated Search Architecture

## Status

**CURRENT — accepted architecture boundary when merged with ADR 0033.**

This document defines the first-party WWW Platform architecture for federated public content search and discoverability. It is an application-level capability of `PublicPortal`, not a new deployable module, search microservice or source-of-truth datastore.

Implementation availability remains separate. This document does not claim that a federated-search route, persistence schema, external search engine or production index exists.

## Purpose

Oteryn has several useful public search surfaces, but each source owns different truth and visibility rules. The portal needs one discoverability workflow that can help a player find first-party content across modules without:

- copying source content into a second authority;
- bypassing publication, localization or privacy rules;
- exposing raw database models;
- making unrelated relevance scores look directly comparable;
- duplicating orchestration in Blade, future PlatformAPI and other clients;
- turning exact-name character lookup into fuzzy public enumeration.

## Decision summary

Federated content search belongs to `PublicPortal` as an application capability named conceptually `FederatedSearch`.

`PublicPortal` owns:

- the public query contract;
- provider selection and timeout budget;
- deterministic cross-source grouping/interleaving policy;
- the normalized public result envelope;
- partial-failure presentation;
- public search page UX, canonical navigation and SEO behavior;
- public-search rate/abuse policy;
- shared application orchestration reused by future `PlatformAPI` when it exposes the same search product.

Source modules continue to own:

- content/data truth;
- publication and visibility eligibility;
- locale-specific search semantics;
- source-local relevance/ranking;
- canonical source URL generation or canonical route identity;
- source revision/freshness/applicability metadata.

No new top-level `Search`, `Discovery`, Elasticsearch, OpenSearch or Meilisearch service is required by this architecture. A dedicated index may be introduced later only as a measured, rebuildable derived projection.

## Boundary map

```text
                         Public web UI
                              |
                              v
                  PublicPortal.FederatedSearch
                  - public query validation
                  - provider orchestration
                  - grouping/interleaving
                  - partial-failure semantics
                  - normalized SearchHit view
                    /     |      |      \
                   /      |      |       \
                  v       v      v        v
                CMS    Events  Wiki   GameCatalog
                 |       |       |        |
          public-only source application queries

Announcements --------------------^

PlayerCompanion public/shareable references
  -> excluded by default
  -> may join only after an explicit public-index contract

PublicGameData exact-name character search
  -> separate search product / vertical
  -> never silently blended into content relevance

Future PlatformAPI
  -> adapter over the same PublicPortal FederatedSearch application service
```

The arrows express the **target federated-search application-query dependencies**. They do not transfer source ownership into `PublicPortal` and do not claim that every current homepage/composition dependency already follows this direction.

## Existing compatibility reverse edge

Current repository code contains a pre-existing reverse dependency that must be accounted for before federated-search provider onboarding:

- `app/Announcements/Queries/AnnouncementTickerProvider.php` imports `App\PublicPortal\PublicContentState`;
- `app/Announcements/ViewModels/AnnouncementTicker.php` imports `App\PublicPortal\PublicContentState`;
- `app/Events/Queries/UpcomingEventProvider.php` imports `App\PublicPortal\PublicContentState`;
- `app/Events/ViewModels/UpcomingEventSummary.php` imports `App\PublicPortal\PublicContentState`.

This coupling supports existing homepage composition and predates ADR 0033. It is compatibility debt, not the accepted federated-search dependency direction.

Before `Announcements` or `Events` is added as a federated-search provider, a bounded implementation task must remove that reverse edge. The preferred shape is that each source returns a source-owned application response/availability state and the PublicPortal adapter maps that state into PublicPortal composition/search presentation state. An explicitly neutral existing boundary may be used only if it has genuine cross-module ownership; ADR 0033 does not authorize creating a generic shared dumping-ground module.

The cleanup must preserve existing homepage behavior and tests. Federated search must not deepen the existing cycle by adding `PublicPortal -> Announcements/Events` search dependencies while `Announcements/Events -> PublicPortal` still exists.

## Provider model

A source participates only through an explicit public provider adapter owned by `PublicPortal` that calls a bounded source-module application query.

The **target** provider dependency direction is:

```text
PublicPortal -> source module application interface
```

Source modules do **not** implement a `PublicPortal` interface directly and must not depend on PublicPortal search contracts, normalized search-result types or search/presentation view models. Existing non-search reverse imports such as the Announcements/Events `PublicContentState` coupling above are migration prerequisites, not exceptions to the target direction.

The adapter normalizes already-authorized public results. It must not fetch broad private rows and filter them later in the presentation layer.

### Initial eligible providers

#### `CMS`

May expose:

- published news/articles;
- published managed pages.

CMS remains authoritative for publication state, localization and canonical public routes.

#### `Announcements`

May expose only announcements eligible under the source module's public/search policy. Expired or historical discoverability is a product decision inside the source query contract; PublicPortal must not infer it from dates independently.

**Onboarding prerequisite:** remove the existing Announcements -> PublicPortal `PublicContentState` dependency before adding the PublicPortal -> Announcements federated-search provider edge.

#### `Events`

May expose publicly searchable event records under source-owned lifecycle rules. Cancelled, draft, scheduled and historical behavior remains owned by Events.

**Onboarding prerequisite:** remove the existing Events -> PublicPortal `PublicContentState` dependency before adding the PublicPortal -> Events federated-search provider edge.

#### `Wiki`

May expose published, locale-appropriate articles/categories through Wiki's existing public search semantics. Draft/review/archive content remains excluded by the source boundary.

#### `GameCatalog`

May expose entities from an active, compatible, public-safe and verified-content boundary only. Inactive candidate snapshots, unknown verification boundaries and unsupported schema content must not become discoverable merely because they exist in storage.

### Conditionally eligible provider

#### `PlayerCompanion`

Private workspaces, goals, tracking lists, session analyses and recommendations are never public-search input by default.

A later explicitly public/shareable artefact, such as an owner-published build representation, may participate only when a separate contract defines:

- deliberate public publication/indexability;
- revocation/expiry behavior;
- allowlisted result fields;
- stable public identity and canonical URL;
- removal propagation;
- abuse/moderation controls.

Possessing a share token does not automatically make an artefact globally searchable.

### Explicitly separate search products

#### `PublicGameData` character search

Character lookup remains an exact-name/read-model function with its own privacy and enumeration boundary. Federated content search may present a clearly separate “Characters” handoff/vertical later, but must not silently mix character records into fuzzy content ranking or broaden enumeration semantics.

#### Marketplace search/filtering

Marketplace catalogue search/filtering remains a Marketplace capability because listing availability, auction state, price/sort policy and abuse controls have different semantics from editorial/content discovery.

#### Support/Admin/account search

Private support, administration, audit, Identity, Accounts and owner-private application records are not public federated-search providers.

## Public query contract

The web/application query is typed and bounded. Exact implementation limits are chosen by the delivery slice, but the architecture requires:

- one explicit locale;
- Unicode-aware normalization and whitespace normalization;
- bounded query length and bounded page/cursor size;
- allowlisted source/result-type filters only;
- no client-supplied SQL fields, arbitrary sort expressions, regular expressions or executable query syntax;
- deterministic normalization so equivalent requests share the same policy/cache identity;
- explicit empty-query behavior rather than treating an empty string as “dump all content”.

A conceptual request is:

```text
FederatedSearchQuery
  query
  locale
  provider_filter[]
  result_type_filter[]
  cursor | page
  limit
```

Before cache identity is derived, all cache-relevant request fields use one versioned canonical serialization: normalized query, explicit locale, sorted/deduplicated allowlisted filters, effective provider set, exactly one pagination mode/value (`page` or validated opaque `cursor`), limit and ranking/grouping policy version. Provider/index/source-generation identity is then appended to the same canonical cache-input structure.

Authentication is not required for the initial public-content product. Adding authenticated/private search would be a separate capability because permission and cache semantics change materially.

## Normalized result envelope

A provider returns source-local public hits. PublicPortal normalizes only presentation-safe metadata into a common envelope:

```text
SearchHit
  source_module
  source_type
  stable_public_id
  source_revision | null
  locale
  title
  snippet_plaintext | null
  canonical_url
  provider_rank
  published_or_effective_at | null
  applicability | null
  freshness | null
  badges[]
```

Rules:

- `stable_public_id` is a public-safe source identity, never an account/session/security identifier;
- `source_revision` is included when the source exposes meaningful revision identity;
- snippets are plain text or safely rendered allowlisted content; raw trusted/untrusted HTML is never passed through as a generic highlight channel;
- canonical URLs are generated/resolved through source-owned public route semantics, not assembled from unchecked database strings;
- game-profile/ruleset/world/season applicability is preserved when relevant;
- freshness is preserved when relevant and is never extended by PublicPortal caching;
- result metadata is sufficient for presentation but not a serialized source database model.

## Ranking and grouping

Raw relevance scores from different source engines are not assumed comparable.

### Initial policy

The preferred first implementation is grouped/vertical presentation:

- Wiki;
- Game Catalog;
- News/pages;
- Events/announcements;
- other explicitly adopted public providers.

Each provider controls source-local ranking. PublicPortal controls deterministic provider order and per-provider result budgets.

### Optional “All” interleaving

If an aggregated list is adopted, PublicPortal must use a versioned deterministic interleaving policy based on provider rank position and bounded source quotas rather than directly sorting unrelated raw scores.

Any future source weights, freshness boosts or personalization are product policy and must be explicit/versioned. Hidden behavioral personalization is not part of the initial public search architecture.

## Localization

- search is executed for an explicit supported locale;
- providers enforce their own locale/publication invariants;
- a result in another locale may appear only under an explicit fallback contract and must carry fallback metadata;
- no silent EN/PL merging that causes a localized slug/title to point at another language;
- canonical URL and hreflang remain source responsibilities;
- query normalization must not destroy meaningful locale-specific characters.

## Publication, privacy and authorization

The source provider query is public-by-construction.

PublicPortal must not:

- request private/draft rows and filter them after retrieval;
- bypass source publication state;
- widen fields beyond the provider's public allowlist;
- index private PlayerCompanion content;
- infer account/character ownership;
- expose moderator/admin/audit/support records;
- turn a public source identifier into an authorization decision.

A source becoming unpublished, revoked or incompatible must stop appearing in search according to the same source truth used by its canonical public page.

### Restrictive publication-decision fence

Publication/visibility authority and ordinary content freshness are separate concerns. A source must expose, directly or through an equivalently strong source-owned projection, enough ordered decision evidence to distinguish an older public/allow decision from a newer restrictive decision such as unpublish, revoke, delete, moderation/legal removal or incompatibility.

The exact source-local field name is not prescribed, but the semantics are equivalent to a monotonic `publication_decision_revision` plus the decision it represents. The ordering proof belongs to the source authority; PublicPortal and any derived index/cache consume it and must not invent or decrement it.

Rules:

- every materialized searchable representation that can outlive the source query binds to the publication/visibility decision revision or equivalent proof under which it was allowed;
- the source-authoritative restrictive decision transition must advance the ordered publication/revocation fence before or atomically with making the restrictive decision effective for public search; asynchronous index/cache tombstone cleanup may follow, but the fence may not lag behind the effective deny;
- once the source authority accepts/effectuates that newer restrictive decision for an object, that decision is the visibility cutoff: an older public representation is no longer serveable even if its ordinary source freshness, index-lag budget or cache TTL has not expired;
- direct provider composition, a derived index, paginated/pre-pagination result caches, web presentation and any future PlatformAPI adapter over this service obey the same cutoff;
- out-of-order delivery cannot regress the accepted publication-decision revision; a delayed older allow cannot supersede a newer deny;
- physical deletion is not required before the cutoff is effective: it is sufficient that every delivery path either proves the newer decision is reflected or rejects the affected representation;
- stale serving may be allowed only for ordinary non-restrictive content/index freshness. It never grants permission to serve through a newer restrictive decision;
- ordinary `source_revision`, index generation, cache generation and TTL are not substitutes for the publication-decision fence unless the source contract proves that the same ordered value advances for every restrictive decision and is checked by every delivery path.

A provider may avoid a synchronous source lookup only through an equivalently strong source-owned decision mechanism whose serving path can still prove the current restrictive fence. A time-expiring allow proof/lease by itself is insufficient: a newer restrictive decision must immediately invalidate or fence that proof for search delivery, for example through a separately current revocation watermark/generation that is checked before serving. If current restrictive-fence state cannot be proven, derived/cached allow material fails closed.

### Propagation, acknowledgement and authority outage

A restrictive decision propagating into an index/cache has an explicit safe state. An affected representation may be served only while its publication decision remains proven current under the restrictive fence. If tombstone/update/cache invalidation is delayed, fails or has ambiguous acknowledgement, the affected representation fails closed until the newer restrictive decision is proven effective for that delivery path.

Publication-authority unavailability is not ordinary stale-content availability:

- a cached/indexed allow decision whose continuing authority cannot be proven against the current restrictive fence must not be silently reused merely because its data TTL or time-based allow lease remains valid;
- a source-owned proof may avoid synchronous lookup only if every accepted newer restrictive decision can invalidate/fence it for all search delivery paths; time expiry alone is not a revocation mechanism after a deny has been accepted;
- if the current restrictive watermark/fence cannot be validated, the affected result is unavailable rather than public-by-default;
- failure may degrade one provider/result group to `PARTIAL` or make the product `UNAVAILABLE` according to the existing failure contract; it must not be represented as a healthy zero-result state or as continued stale authorization;
- recovery from the outage revalidates publication-decision evidence before old cached/indexed material can be served again.

This fence changes visibility authority only. It does not transfer publication ownership from the source module into PublicPortal.

## Failure and partial availability

Federation must fail honestly without turning one dependency failure into fabricated emptiness.

The orchestration result distinguishes:

```text
COMPLETE
PARTIAL
UNAVAILABLE
INVALID_QUERY
```

Rules:

- one provider failure may yield `PARTIAL` results from healthy providers;
- the UI identifies unavailable result groups generically without leaking internal hostnames, SQL errors or stack traces;
- zero results from a healthy provider differ from an unavailable provider;
- if every required provider is unavailable, the search product returns an explicit unavailable state rather than “0 results”;
- provider timeouts are bounded and do not allow one dependency to consume the entire request budget;
- retries, if any, are bounded and must not multiply load during an outage;
- inability to prove current publication authority for an affected derived representation follows the restrictive-fence failure semantics above rather than ordinary stale-content serving.

## Dedicated index / search-engine direction

A standalone search engine is **not** a prerequisite for the first delivery.

The preferred sequence is:

1. remove any reverse module dependency that would create a cycle for the providers selected by the implementation slice;
2. use module-owned public application queries and bounded fan-out;
3. measure latency, result quality, query volume and database cost;
4. introduce a dedicated derived index only when evidence shows fan-out/local search is insufficient.

If a dedicated index is introduced later:

- it is a **rebuildable projection**, never source truth;
- every indexed document records source module/type/public ID/locale/source revision/index generation and the publication-decision revision/equivalent proof under which it is searchable;
- only source-authorized public fields are indexed;
- publication/removal/revocation creates deterministic update/tombstone behavior;
- rebuild and cutover are generation-based so a partial rebuild cannot become silently authoritative;
- stale-index behavior and maximum tolerated lag are explicit per source, but never override a newer restrictive publication decision;
- an index generation cannot be activated or rolled back if its publication-decision watermark would move any source/object behind a newer accepted restrictive decision; affected entries remain fenced until rebuilt/reconciled;
- out-of-order update/tombstone delivery cannot lower the accepted publication-decision watermark;
- indexing failures do not mutate source publication state and do not make a failed restrictive propagation look successful;
- secrets/private records are never ingested “for filtering later”;
- engine replacement is possible without changing canonical source identities/URLs.

No Elasticsearch/OpenSearch/Meilisearch-specific business contract is accepted by this architecture.

## Cache policy

Search terms and opaque cursor material can contain sensitive or high-cardinality data, so caching is conservative.

- do not create unbounded permanent cache keys from arbitrary requests;
- cache correctness is defined over the **entire semantic response-shaping request**, not only the query term;
- a versioned canonical cache-input structure includes: normalized query, locale, sorted/deduplicated provider filter, sorted/deduplicated result-type filter, effective provider set, pagination mode plus validated `page` or opaque `cursor` value, `limit`, ranking/grouping policy version, and the provider/source/index generation vector needed to prevent stale cross-generation reuse;
- the externally stored cache query/request identity is a versioned server-keyed digest of that canonical structure (for example `HMAC-SHA-256(cache-key-secret, canonical-cache-input)`), never raw query/cursor text and never an unkeyed/plain hash that is practical to dictionary-recover for common terms;
- semantically different page/cursor/limit/filter/provider/ranking-policy/generation inputs therefore cannot intentionally share a paginated `SearchResponse` cache entry;
- the HMAC key is managed as application secret material, is not emitted to logs/artifacts, and its identifier/version participates in the cache namespace so rotation creates a clean cache generation rather than cross-key ambiguity;
- cache lookup equality uses the full digest; implementations must not truncate it below their collision-resistance requirement merely to shorten cache keys;
- if a future implementation deliberately caches a complete pre-pagination result set instead of a paginated response, that must be a separately named cache layer whose identity excludes pagination only because page/cursor/limit slicing occurs strictly after the cached object and is proven not to mutate source/ranking semantics; the paginated response cache defined here always includes pagination inputs;
- any result cache uses bounded TTL/size; a source's shorter freshness/publication rule wins over a longer PublicPortal cache;
- cache hit/TTL validity never bypasses the restrictive publication-decision fence; an entry materialized under an older allow is rejected once a newer restrictive decision is authoritative or its required current restrictive-fence proof is unavailable;
- restrictive invalidation is considered safely effective only when the delivery path reflects the newer decision or is fenced from serving the affected older representation; physical cache eviction may occur later;
- `PARTIAL`/`UNAVAILABLE` responses use shorter or no caching as appropriate;
- authenticated/private-search responses, if ever added, require owner/authorization-safe cache partitioning and are outside this public contract;
- cache clearing is not a substitute for deterministic source revision/index invalidation or publication-decision fencing.

This keyed request identity is a cache/internal identifier only. It is not a user identifier, analytics identifier or permission token and must not be promoted into logs, metrics labels or public URLs.

## Security and abuse controls

Implementation must include:

- bounded anonymous request rates with stricter limits for expensive filters/providers;
- server-side allowlists for provider/type filters;
- parameterized source queries and no query-language passthrough to SQL/search engines;
- payload/response size limits;
- HTML/script-safe rendering of titles/snippets/highlights;
- URL generation from trusted route identity or validated canonical URLs;
- bot/scraping controls that remain defense in depth and do not replace application limits;
- separate anti-enumeration policy for any future character-search composition;
- denial/fallback behavior that does not disclose private provider existence.

Search is read-only. It authorizes no mutation.

## Query privacy

Search text and opaque pagination material can contain names, identifiers or other sensitive data even on a public form.

Therefore:

- raw queries and raw opaque cursors are not metric labels;
- raw queries/cursors are not written to ordinary structured application logs by default;
- keyed cache request digests are not emitted as ordinary logs/metric labels or reused as cross-context tracking identifiers;
- traces record bounded metadata such as normalized length bucket, pagination mode, provider set, result-count bucket and status rather than full query/cursor text or cache-key digest;
- any future product analytics over search terms requires a separate privacy/retention decision and aggregation policy;
- error reports must not echo raw query/cursor text into public diagnostics or audit metadata.

## Observability

Safe bounded metrics include:

- request count/status;
- overall and per-provider latency;
- provider timeout/failure count;
- provider result-count buckets;
- zero-result rate;
- `COMPLETE` versus `PARTIAL` versus `UNAVAILABLE` ratio;
- cache hit/miss where adopted;
- derived-index generation/lag/rebuild health if an index exists;
- restrictive publication-fence rejection/lag/propagation-health counts using bounded source/status dimensions, never public IDs as labels.

Do not use query strings, opaque cursors, cache request digests, public IDs, slugs, titles or user identifiers as unbounded metric labels.

## SEO and discoverability

Federated search improves navigation but is not a content source.

- canonical source pages remain the indexable documents;
- arbitrary query-result URLs should normally be `noindex` to avoid crawl-space explosion and duplicate thin pages;
- sitemap generation remains source/publication driven, not generated from popular search terms;
- search links do not publish otherwise hidden content;
- structured data belongs to the canonical source page, not the generic search result unless separately justified.

## PlatformAPI reuse

If `PlatformAPI` later exposes first-party federated search, it must adapt the same `PublicPortal` application service and normalized search contract.

It must not:

- independently fan out to providers;
- recreate cross-source ranking/grouping policy;
- expose raw provider database records;
- broaden provider visibility because the consumer is a client instead of a browser;
- bypass or independently reinterpret the restrictive publication-decision fence.

The API may version transport details, pagination encoding and scopes while preserving the same underlying provider/publication/partial-failure semantics.

This dependency is intentional:

```text
Web controller --------+
                       v
          PublicPortal.FederatedSearch application service
                       ^
PlatformAPI adapter ---+
```

If a future non-portal product requires materially different search semantics, that need is evaluated separately before extracting a shared `Discovery` module.

## Implementation shape

A future bounded slice should prefer a structure similar to:

```text
app/PublicPortal/Application/Search/
  FederatedSearchQuery.php
  FederatedSearchService.php
  SearchHit.php
  SearchResponse.php
  Providers/

app/PublicPortal/Http/
  SearchController.php

resources/views/public/search/
```

Provider adapters depend on application/query interfaces and source-owned response/availability types from their source modules. They do not import arbitrary Eloquent models across module boundaries.

For any provider whose current module already imports PublicPortal composition state, removal of that reverse import is part of the provider-onboarding prerequisite and must be validated before the new PublicPortal -> source search edge is added.

This is a direction, not authorization to refactor unrelated code.

## Required implementation states

A delivered search surface must prove at least:

- valid results;
- no results;
- invalid/too-short/too-long query;
- one-provider partial failure;
- all-provider unavailable state;
- unsupported locale/filter;
- source item unpublished between query/index refresh and canonical navigation;
- newer revoke/unpublish/delete/incompatibility while an older indexed/cached allow representation still exists;
- out-of-order older allow/update arriving after a newer restrictive decision;
- tombstone/index/cache invalidation failure or ambiguous acknowledgement after a restrictive decision;
- concurrent refresh versus revoke with the restrictive decision winning deterministically;
- publication-authority outage while an older cached/indexed allow exists, including inability to validate the current restrictive watermark/fence;
- rebuild/cutover and rollback to an older index generation after a newer restrictive decision without resurrection of the result;
- responsive/mobile layout;
- keyboard and screen-reader result navigation;
- EN/PL behavior;
- rate-limit response;
- dependency recovery with publication-decision evidence revalidated before stale derived material becomes serveable again;
- cache isolation for distinct normalized queries with identical locale/filter/provider generations;
- cache isolation for different `page`, opaque `cursor`, `limit`, filters, provider sets and ranking-policy versions;
- cache identity does not expose raw query/cursor text and keyed-digest rotation creates a separate cache namespace;
- exact-head browser E2E with retries zero.

If a derived index exists, also prove rebuild, stale generation, restrictive-decision watermark, deletion/tombstone and rollback/cutover behavior.

Provider onboarding must additionally prove that the resulting module dependency graph has no new `PublicPortal <-> provider` cycle.

## Explicit non-goals

- replacing source-module search/business rules;
- global database/model serialization;
- fuzzy public people discovery by default;
- searching private account/support/admin/audit data;
- indexing private PlayerCompanion data by possession of a share token;
- external web search or scraping third-party sites;
- AI-generated answers presented as search truth;
- mandatory external search-engine infrastructure;
- a new microservice without measured need;
- modifying game/server/client repositories.

## Completion criteria

The architecture is satisfied when:

- ADR 0033 is accepted;
- `PublicPortal` ownership in `MODULE_CATALOG.md` reflects federated-search orchestration without claiming implementation;
- `PORTAL_COMPLETENESS_ARCHITECTURE.md` moves federated content search from unresolved discovery to architecture-accepted/planned implementation;
- the known Announcements/Events reverse edge is recorded as an implementation prerequisite rather than hidden by the target dependency diagram;
- cache correctness/privacy binds the full response-shaping request — including query, locale, filters, providers, pagination, limit, ranking-policy version and generations — to a keyed canonical-request identity so distinct responses cannot share a cache entry and raw request material does not become cache-key/log/metric data;
- source-owned publication/visibility decision evidence is ordered separately from ordinary freshness, every stale derived representation is fenced by newer restrictive decisions, propagation failure/outage fails closed, and rebuild/rollback cannot cross a newer restrictive watermark;
- Issue #935 remains terminal and Issue #938 is terminal after exact-head self-review, CI, review hygiene, merge and any required lifecycle closeout.