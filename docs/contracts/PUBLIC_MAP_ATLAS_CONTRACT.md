# Public Map Atlas Producer/Consumer Contract

## Status

Proposed — 2026-08-14. Runtime activation is blocked until the producer requirements below exist on `blakinio/Otheryn/main` and a complete immutable atlas release has been published.

## Purpose

Oteryn Platform exposes a player-facing **Map** while `blakinio/Otheryn` remains the sole producer and source of truth for the OTBM-derived atlas. This contract prevents a second parser, renderer, generated-data pipeline or independently evolving viewer from appearing in Platform.

This contract is a deployment and presentation boundary. It does not transfer canonical map-data ownership to Platform.

## Repository responsibilities

### Producer — `blakinio/Otheryn`

Otheryn owns:

- `world.otbm` and its authoritative source fingerprint;
- canonical game assets and their fingerprint/version;
- OTBM parsing and atlas generation;
- chunk geometry, multi-resolution rendering and canonical detailed pixels;
- atlas manifest schema and chunk inventory;
- factual/search/detail/overlay data derived from server/map sources;
- environment-animation export and runtime semantics;
- the existing viewer/runtime implementation and its bounded client caches;
- generation-time verification and full-world certification;
- immutable atlas release creation and release provenance.

The producer runs parsing/rendering offline in CI/build-time. Platform request handling must never parse OTBM or regenerate atlas content.

### Consumer — `blakinio/Oteryn-Platform`

Platform owns:

- public `/map` routing and localized presentation metadata;
- player-facing navigation label **Map** / **Mapa** and homepage discovery entry;
- a dedicated native full-viewport Map shell that preserves useful map viewport area;
- deployment-time acquisition and verification of an exact producer release;
- same-origin publication of immutable atlas files for the initial deployment model;
- cache/security headers and operational monitoring for failed atlas loads;
- future integration of the Map with approved Platform player features without changing canonical atlas ownership.

Platform must not copy the OTBM parser, atlas generator, canonical world source, or maintain a forked replacement of the producer viewer/runtime.

## Product URL and viewer state

The public product route is:

```text
/map
```

The existing Otheryn query-state contract remains authoritative where currently supported:

```text
x
y
z
zoom
render
layers
marker
```

A shareable position therefore remains conceptually:

```text
/map?x=<x>&y=<y>&z=<z>&zoom=<zoom>[&render=...][&layers=...][&marker=...]
```

Platform must not introduce an incompatible second X/Y/Z/zoom representation merely for portal integration. Any future state-contract revision belongs to the producer viewer contract and requires backward-compatibility analysis.

## Initial publication model

The selected initial model is **build artifact consumption with same-origin publication**:

```text
Otheryn CI
  -> generates and verifies the complete atlas
  -> creates one immutable versioned release artifact
  -> records release provenance and digest

Oteryn Platform deployment
  -> selects one exact producer release
  -> verifies provenance, descriptor and digest
  -> stages the complete version without making it active
  -> verifies required files and referenced manifest/chunks
  -> publishes immutable files under a versioned same-origin path
  -> switches the active version pointer last
```

This is Variant A from the integration analysis. It is preferred initially because current Platform infrastructure has no atlas object-storage/CDN contract and current CSP permits same-origin scripts, connections and images. A future object-storage/CDN origin may replace the storage backend only through an explicit security/deployment change; it must not change canonical producer ownership or browser chunk-loading semantics.

Platform must not generate the atlas during its application/image build as a substitute for the producer release.

## Required producer release descriptor

Before Platform runtime integration can activate, the producer must publish a stable machine-readable release descriptor. The exact filename may be selected in Otheryn, but the contract must expose at least:

- release descriptor schema version;
- immutable `buildId` / atlas release ID;
- exact Otheryn producer commit SHA;
- atlas manifest path and SHA-256;
- atlas format/version (currently Atlas v3 on inspected main);
- source OTBM SHA-256;
- canonical asset fingerprint/SHA-256;
- chunk size;
- complete payload digest or equivalent independently verifiable release integrity record;
- viewer entry/bootstrap identity and the supported asset-base/mount contract;
- enough metadata for the consumer to reject an incomplete or internally inconsistent release.

Existing `manifest.json` source fingerprints may be reused; they should not be duplicated with different semantics. `buildId` is an activation identity, not a replacement for content fingerprints.

## Required viewer consumption contract

The producer viewer remains authoritative. To embed it as a native Platform feature without an iframe or fork, Otheryn must expose a stable consumer-facing contract that supports an explicit asset/data base URL or equivalent mount/bootstrap configuration.

The contract must allow the existing runtime to resolve, without Platform source rewriting:

- atlas manifest;
- viewer runtime/module assets;
- rendered chunk images and overviews;
- factual/search/detail/overlay data;
- environment-animation data and frames.

Hard-coded assumptions about being served from the standalone viewer document directory must not be the only supported integration path once the release is declared consumer-ready.

Platform may provide its own surrounding shell and portal controls, but must preserve producer behavior for chunk visibility, floors, render modes, canonical detail, search/details/overlays, URL state, animation and bounded caches unless a producer contract explicitly evolves them.

## Static URL and activation layout

The consumer publishes immutable release content beneath a versioned prefix such as:

```text
/map/atlas/<build-id>/...
```

The active-version pointer is a small independently cacheable resource such as:

```text
/map/atlas/active.json
```

The exact static prefix may change before runtime implementation if required by the verified reverse-proxy layout, but the invariants are mandatory:

- immutable build-specific URLs never change bytes after publication;
- a new build is fully staged and verified before activation;
- the active pointer changes atomically after the complete version is present;
- a manifest must never resolve to a mixture of old and new release chunks;
- rollback selects a previously complete immutable version rather than mutating files in place.

## Cache contract

For immutable build-specific files, the target response policy is:

```text
Cache-Control: public, max-age=31536000, immutable
```

For the small active-version pointer, use revalidation semantics such as `no-cache` (or an equivalently short, explicitly justified cache policy).

Other rules:

- use content/versioned URLs for cache busting;
- compress text assets such as JSON/JavaScript where supported;
- do not waste CPU recompressing already compressed PNG payloads solely for transfer policy;
- preserve producer bounded client caches;
- initial render may fetch only the current viewport/floor/resolution plus the producer-defined small margin/prefetch;
- never preload all certified chunks, all floors, or one whole-world JSON representation.

## Runtime invariants

Consumer integration must preserve the current producer invariants unless a later producer contract deliberately changes them:

- chunked world loading;
- Z0-Z15 support for the certified full-world release;
- multi-resolution overview/detail selection;
- canonical detailed pixels at detailed/max zoom;
- floor switching;
- pan and zoom;
- search, details and factual overlays;
- URL state/deep linking;
- environment-animation runtime where the release provides it;
- bounded image, overlay and animation caches.

A Platform feature must not make initial page rendering dependent on downloading the whole atlas.

## Security and provenance

The initial same-origin model intentionally avoids broadening current CSP/CORS policy. If a future CDN/object-storage origin is introduced, CSP/CORS changes require their own reviewed security scope.

The deployment consumer must fail closed when:

- producer release identity or expected digest cannot be proven;
- release descriptor or manifest schema is unsupported;
- manifest digest differs from the release descriptor;
- required viewer/bootstrap files are absent;
- a referenced release path escapes its version root;
- activation would mix versions or overwrite immutable files.

Static serving must use a fixed configured root/alias. User-controlled path input must not become an arbitrary filesystem path. Deployment credentials must have only the minimum permissions required for artifact retrieval/publication, and secrets must not be committed into the repository or release descriptor.

No OTBM parser, untrusted archive processing or generation step is allowed in the public HTTP request path.

## Native Platform UX boundary

The Map is a first-class Platform feature, not a small card or embedded external site.

Desktop:

- maximize map viewport;
- keep pan/zoom/floor/search/overlay/detail controls immediately usable;
- use a compact native portal/brand affordance rather than the ordinary content-width page shell.

Mobile:

- retain a useful map viewport;
- use collapsible drawers/bottom sheets where needed for secondary controls;
- preserve touch pan/zoom without large persistent panels covering most of the map.

An iframe is not the target integration contract. It prevents clean first-party URL/state, shell, telemetry/error handling and future feature composition and is unnecessary once the producer exposes the stable mount/bootstrap contract above.

## Navigation classification

Current Platform navigation is still a flat delivered header, while accepted frontend architecture classifies scalable destinations under grouped public information architecture. Therefore:

- while the flat header remains, **Map** should be adjacent to the existing world/game destinations (after `Servers` unless current implementation evidence requires a different ordering);
- when grouped navigation is activated, **Map** belongs under **Game**;
- the footer may expose Map in the existing World/Game-oriented group;
- the homepage should expose Map through the player discovery area after the runtime feature is real.

The route/navigation entry must not ship before a functional canonical Map is available.

## Future capability boundary

The contract deliberately supports later producer-backed layers and Platform integrations for NPCs, monster spawns, bosses, raids, shops/banks, travel/teleports, quests/actions, hunting areas, POIs, object search, wiki links and routing/route sharing.

Player position, party sharing or live-world data require separate privacy, authority and runtime contracts. This contract does not authorize placeholder backends or collection of live player telemetry.

## Activation gate

Platform runtime implementation is **BLOCKED** until both producer conditions are proven on an exact Otheryn release:

1. a complete immutable production atlas payload is published for consumption, rather than certification evidence alone;
2. the existing viewer exposes a stable explicit asset-base/mount/bootstrap contract suitable for native Platform integration without source forking/post-processing.

Once those conditions exist, the Platform task may continue with route/navigation/static-serving implementation, tests, real browser E2E and production deployment verification.