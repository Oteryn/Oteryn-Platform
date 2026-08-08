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

The arrows express application-query dependencies. They do not transfer source ownership into `PublicPortal`.

## Provider model

A source participates only through an explicit public provider adapter owned by `PublicPortal` that calls a bounded source-module application query.

Source modules do **not** implement a `PublicPortal` interface directly and do not depend back on `PublicPortal`. This preserves one-way dependency direction:

```text
PublicPortal -> source module application interface
```

The adapter normalizes already-authorized public results. It must not fetch broad private rows and filter them later in the presentation layer.

### Initial eligible providers

#### `CMS`

May expose:

- published news/articles;
- published managed pages.

CMS remains authoritative for publication state, localization and canonical public routes.

#### `Announcements`

May expose only announcements eligible under the source module's public/search policy. Expired or historical discoverability is a product decision inside the source query contract; PublicPortal must not infer it from dates independently.

#### `Events`

May expose publicly searchable event records under source-owned lifecycle rules. Cancelled, draft, scheduled and historical behavior remains owned by Events.

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
- retries, if any, are bounded and must not multiply load during an outage.

## Dedicated index / search-engine direction

A standalone search engine is **not** a prerequisite for the first delivery.

The preferred sequence is:

1. use existing module-owned public application queries and bounded fan-out;
2. measure latency, result quality, query volume and database cost;
3. introduce a dedicated derived index only when evidence shows fan-out/local search is insufficient.

If a dedicated index is introduced later:

- it is a **rebuildable projection**, never source truth;
- every indexed document records source module/type/public ID/locale/source revision/index generation;
- only source-authorized public fields are indexed;
- publication/removal/revocation creates deterministic update/tombstone behavior;
- rebuild and cutover are generation-based so a partial rebuild cannot become silently authoritative;
- stale-index behavior and maximum tolerated lag are explicit per source;
- indexing failures do not mutate source publication state;
- secrets/private records are never ingested “for filtering later”;
- engine replacement is possible without changing canonical source identities/URLs.

No Elasticsearch/OpenSearch/Meilisearch-specific business contract is accepted by this architecture.

## Cache policy

Search terms are high-cardinality and may contain personal or sensitive text, so caching is conservative.

- do not create unbounded permanent cache keys from arbitrary queries;
- any result cache uses bounded TTL/size and includes locale, filters and provider/index generation identity;
- a source's shorter freshness/publication rule wins over a longer PublicPortal cache;
- `PARTIAL`/`UNAVAILABLE` responses use shorter or no caching as appropriate;
- authenticated/private-search responses, if ever added, require owner/authorization-safe cache partitioning and are outside this public contract;
- cache clearing is not a substitute for deterministic source revision/index invalidation.

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

Search text can contain names, emails, support-like text or other personal data even on a public form.

Therefore:

- raw queries are not metric labels;
- raw queries are not written to ordinary structured application logs by default;
- traces record bounded metadata such as normalized length bucket, provider set, result count and status rather than full query text;
- any future product analytics over search terms requires a separate privacy/retention decision and aggregation policy;
- error reports must not echo raw query text into public diagnostics or audit metadata.

## Observability

Safe bounded metrics include:

- request count/status;
- overall and per-provider latency;
- provider timeout/failure count;
- provider result-count buckets;
- zero-result rate;
- `COMPLETE` versus `PARTIAL` versus `UNAVAILABLE` ratio;
- cache hit/miss where adopted;
- derived-index generation/lag/rebuild health if an index exists.

Do not use query strings, public IDs, slugs, titles or user identifiers as unbounded metric labels.

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
- broaden provider visibility because the consumer is a client instead of a browser.

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

Provider adapters depend on application/query interfaces from their source modules. They do not import arbitrary Eloquent models across module boundaries.

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
- responsive/mobile layout;
- keyboard and screen-reader result navigation;
- EN/PL behavior;
- rate-limit response;
- dependency recovery;
- exact-head browser E2E with retries zero.

If a derived index exists, also prove rebuild, stale generation, deletion/tombstone and rollback/cutover behavior.

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
- Issue #935 is terminal after exact-head self-review, CI, review hygiene, merge and task archival.
