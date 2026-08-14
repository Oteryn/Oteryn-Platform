# ADR 0038: Public Map and OTBM Atlas integration

- Status: Proposed
- Date: 2026-08-14
- Decision owner: repository owner
- Task: `OTERYN-20260814-public-map-atlas-integration`

## Context

Oteryn Platform is the public web/application layer for players. `blakinio/Otheryn` already contains a mature canonical OTBM Atlas under `tools/otbm_atlas/`; rebuilding the map stack in Platform would create a second source of truth and duplicate a functioning producer/runtime.

The 2026-08-14 audit proved:

- Platform `main` was `e4498ba9856a3779c8ae3a6f5bed608256a35fef` at task-branch creation;
- Otheryn `main` was `da553b1f2f157526e69e26d051ca3297db7abcf6` at the producer audit;
- Otheryn atlas PRs #381, #384, #387 and #389 are merged;
- the certified full-world build covers Z0-Z15 and exactly 3494 chunks;
- the existing producer viewer owns viewport-bounded chunk loading, multi-resolution render modes, canonical detailed pixels, floor selection, search/details/overlays, URL state and bounded animation/image/data caches;
- the current full-world certification workflow uploads per-floor verification evidence, not the complete generated atlas payload needed by another deployment;
- current viewer assets/data are resolved through standalone relative paths and no explicit stable consumer-facing asset-base/mount/bootstrap API exists yet;
- Platform currently has no Map/Atlas route or navigation entry, no atlas object-storage/CDN contract, same-origin CSP for scripts/connections/images, and a Synology Nginx configuration that proxies all requests to the application rather than serving a large versioned static atlas tree directly;
- accepted frontend architecture requires scalable player-facing information architecture and responsive shells rather than forcing every surface through one permanent content-width layout;
- accepted PlayerCompanion architecture explicitly treats advanced maps as a bounded player-facing capability rather than a second website.

The product decision must therefore separate two questions:

1. where a player should find and use the Map in Platform;
2. how Platform can consume the existing canonical producer without copying its generator/runtime or weakening chunked delivery.

## Decision

### 1. Player-facing name and route

The canonical player-facing feature is **Map** in English and **Mapa** in Polish.

The public route is:

```text
/map
```

`OTBM Atlas` remains an implementation/provenance term and may appear in technical/about metadata, not as the primary navigation label.

The existing Otheryn viewer query contract for `x`, `y`, `z`, `zoom`, `render`, `layers` and `marker` is preserved where supported. Platform must not create a competing coordinate/deep-link state model without a separate compatibility decision.

### 2. Navigation and discovery

When the current flat public header is still in use, `Map` should appear alongside the delivered world/game destinations, after `Servers` unless implementation evidence at that time requires a different local ordering.

When the accepted grouped information architecture is activated, `Map` belongs under **Game**.

The homepage gains a Map entry in the player/discovery area only when the canonical runtime feature is functional. Platform must not advertise a dead or placeholder route.

### 3. Dedicated native full-viewport shell

`/map` uses a dedicated first-party Map layout/shell inside Platform rather than the ordinary constrained content-width page shell.

The shell keeps only compact portal/brand/navigation affordances around a dominant map viewport. Desktop prioritizes visible map area and immediate controls. Mobile recomposes secondary controls into collapsible drawers/bottom sheets so pan/zoom remains usable.

The target integration is native; an iframe is rejected as the normal architecture.

### 4. Repository responsibility boundary

`blakinio/Otheryn` remains producer and source of truth for:

- OTBM and canonical assets;
- parser/render/build pipeline;
- chunks and manifests;
- canonical/factual/search/overlay data;
- viewer/runtime and animation semantics;
- atlas release provenance.

`blakinio/Oteryn-Platform` is consumer and owns:

- public route and navigation;
- native portal shell and responsive UX;
- deployment-time acquisition/verification of an exact producer release;
- public static serving/cache/security policy;
- load-failure observability and future approved portal integrations.

Platform must not copy the producer parser/generator or maintain an independently evolving replacement viewer.

### 5. Initial publication model: Variant A

The initial selected publication model is **producer build artifact -> Platform deployment consumer -> same-origin immutable static publication**.

Otheryn CI must produce one complete, immutable, versioned atlas release with a machine-readable release descriptor, explicit `buildId`, producer commit, manifest digest, OTBM fingerprint, asset fingerprint and complete release integrity evidence.

Platform deployment selects one exact producer build and digest, verifies it, stages it under a versioned path, validates the release, and only then atomically switches the active-version pointer.

Target static shape is conceptually:

```text
/map/atlas/<build-id>/...
/map/atlas/active.json
```

Build-specific URLs are immutable and aggressively cacheable. The active pointer is small and revalidated/short-lived. Activation must never expose a manifest that resolves to mixed old/new chunks.

### 6. Preserve chunked runtime behavior

The browser continues to fetch only data required for current viewport, floor and resolution plus the producer-defined bounded margin/prefetch.

Platform must not:

- load the whole OTBM in the browser;
- load all 3494 certified chunks at entry;
- serialize the whole world into one giant JSON document;
- make initial rendering depend on the complete atlas;
- replace bounded producer caches with unbounded Platform caches.

### 7. Stable producer viewer consumption contract is required before runtime implementation

Otheryn must expose an explicit stable asset-base/mount/bootstrap contract so the existing viewer can be hosted inside the native Platform shell while resolving manifest, chunks, factual/search/overlay data and animation assets from the selected versioned release.

Platform must not solve the current standalone-relative-path assumption by permanently rewriting/post-processing producer JavaScript or forking the viewer source.

The detailed producer/consumer requirements are canonical in `docs/contracts/PUBLIC_MAP_ATLAS_CONTRACT.md`.

### 8. Security and cache boundary

The initial same-origin design intentionally preserves current CSP/CORS assumptions. OTBM parsing/rendering remains offline/build-time and is forbidden in the HTTP request path.

Immutable version paths use long-lived immutable caching; the active pointer uses revalidation. Deployment fails closed on release/digest/schema/path inconsistencies and activates only a complete staged version.

A future object-storage/CDN origin may be adopted through a separate reviewed deployment/security change once infrastructure and CSP/CORS contracts exist.

## Alternatives considered

### `/atlas` as the primary public route

Rejected. `Atlas` is implementation-oriented terminology; `Map`/`Mapa` is clearer for ordinary players and fits the portal's player-facing vocabulary.

### Standard content page/card inside the existing portal shell

Rejected. A world map benefits directly from maximum viewport area. A normal constrained page shell would trade away the primary interaction surface and perform poorly on mobile.

### iframe to a standalone atlas

Rejected as the target architecture. It weakens first-party URL/state composition, responsive shell integration, load-error handling, accessibility/navigation cohesion and future Platform feature composition. Current CSP also intentionally disallows arbitrary framed applications through `frame-ancestors 'none'`; no iframe exception is justified.

### Rewrite the atlas viewer/runtime in Platform

Rejected. Otheryn already owns a capable tested viewer/runtime. A rewrite would create duplicated semantics, cache behavior, URL behavior and rendering regressions.

### Generate/copy OTBM source into Platform (Variant C as producer duplication)

Rejected. Platform is a consumer, not an atlas generator. Canonical map source and parser/render pipeline remain in Otheryn.

### Object storage/CDN now (Variant B)

Deferred, not rejected permanently. Current Platform repository has no atlas storage/CDN contract and its CSP is same-origin-only for the relevant resource classes. Introducing a second origin before it is operationally necessary creates additional CORS/CSP/credential/cache surface. A CDN can later back the same immutable versioned contract without changing producer ownership.

### Publish `/map` now with placeholder/fallback content

Rejected. The public route must represent the canonical Map. Advertising a dead route, sample build or non-canonical fallback would violate the evidence boundary and user acceptance requirements.

## Consequences

### Positive

- one canonical atlas producer survives;
- existing viewer/runtime capability is reused rather than reimplemented;
- player-facing route/name are simple and durable;
- deep links remain compatible with producer state;
- same-origin initial publication preserves current CSP/CORS defaults;
- versioned immutable payloads enable aggressive caching, rollback and atomic activation;
- later factual layers, wiki links, routes and approved player features can compose around one Map surface.

### Negative

- Platform runtime work cannot finish until Otheryn publishes a complete consumer-ready release and stable mount/asset-base contract;
- initial same-origin serving requires Platform deployment/reverse-proxy work for large static payloads;
- Map UX needs a dedicated layout rather than pure reuse of the current standard page shell;
- producer and consumer releases become an explicit cross-repository compatibility contract that must be versioned and validated.

## Activation boundary

This ADR is **Proposed** because the required producer consumption contract does not yet exist on Otheryn `main`.

This ADR does not authorize a placeholder `/map`, iframe, copied viewer or Platform-side OTBM generator.

Runtime implementation may continue only after an exact Otheryn release proves both:

1. a complete immutable production atlas payload suitable for deployment consumption;
2. a stable viewer asset-base/mount/bootstrap contract that avoids a Platform fork/post-processing dependency.

After those prerequisites are real, the Platform task must implement routing/navigation/static serving, run repository tests and real desktop/mobile browser E2E, deploy through the approved production path, and verify the public Map in a browser before the task can be called complete.

## References

- `docs/contracts/PUBLIC_MAP_ATLAS_CONTRACT.md`
- `docs/architecture/adr/0008-oteryn-frontend-information-and-shell-architecture.md`
- `docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md`
- `docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md`
- `docs/agents/tasks/active/OTERYN-20260814-public-map-atlas-integration.md`
- `blakinio/Otheryn` PRs #381, #384, #387, #389
- `blakinio/Otheryn/tools/otbm_atlas/`