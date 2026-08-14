---
task_id: OTERYN-20260814-public-map-atlas-integration
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0008-oteryn-frontend-information-and-shell-architecture.md
  - docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/contracts/PUBLIC_MAP_ATLAS_CONTRACT.md
search_first:
  - Otheryn PRs #381 #384 #387 #389
  - Otheryn tools/otbm_atlas
  - public navigation and routing
  - Synology staging static-asset and reverse-proxy deployment
optional_reads:
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
---

# OTERYN-20260814-public-map-atlas-integration

## Goal

Define and, when the producer contract exists, deliver the canonical Otheryn OTBM Atlas as a first-class public Map capability in Oteryn Platform without duplicating the atlas generator, canonical world data, or viewer runtime ownership.

## Acceptance criteria

- [x] Reconcile current `blakinio/Oteryn-Platform/main`, open PR/path ownership, public routing/navigation/layout, security headers and Synology deployment shape.
- [x] Reconcile current `blakinio/Otheryn/main`, merged atlas PRs, `tools/otbm_atlas/`, viewer URL/runtime behavior and the full-world certification workflow.
- [x] Record a durable product/architecture decision for public route, navigation placement, layout, repository ownership and publication model.
- [x] Record the producer-to-consumer publication, versioning, cache, atomic activation and security contract.
- [ ] Otheryn publishes one complete immutable production atlas release artifact, not certification evidence only, with an explicit release/build identity and provenance.
- [ ] Otheryn exposes a stable consumer-facing viewer mount/bootstrap or configurable asset-base contract so Platform can embed the existing runtime natively without forking it.
- [ ] Platform implements the public Map route, navigation/home entry, dedicated full-viewport shell and same-origin immutable atlas serving against the exact producer contract.
- [ ] Desktop/mobile browser E2E proves routing/deep links, canonical rendering, floors, zoom, search/details/overlays, bounded chunk loading and animation behavior on the exact implementation head.
- [ ] Public production deployment is verified in a real browser, including network requests, console, CSP/CORS, manifest version and cache behavior.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-map-atlas-integration.md
  - docs/architecture/adr/0038-public-map-atlas-integration.md
  - docs/architecture/adr/README.md
  - docs/contracts/PUBLIC_MAP_ATLAS_CONTRACT.md
modules:
  - public-web
  - player-companion
  - atlas-integration
  - synology-staging
dependencies:
  - blakinio/Otheryn canonical OTBM Atlas producer at current main
  - ADR 0008 public frontend and shell architecture
  - ADR 0025 PlayerCompanion boundary
blockers:
  - Otheryn current full-world release workflow publishes tiny verification evidence artifacts only, not the complete generated atlas payload required by a consumer deployment.
  - Otheryn current standalone viewer resolves manifest/runtime/data through relative paths and has no explicit stable consumer-facing asset-base or mount/bootstrap contract for native Platform integration.
cross_repository_tasks:
  - A producer-owned Otheryn follow-up must publish the complete immutable atlas release and stable viewer consumption contract; Otheryn remains read-only to this Platform task.
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T19:13:36Z
head: e4498ba9856a3779c8ae3a6f5bed608256a35fef
branch: agent/oteryn-20260814-public-map-atlas-integration
pr: none
status: blocked
context_routes:
  - agent-governance
  - architecture
  - frontend
  - deployment
  - security
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-map-atlas-integration.md
  - docs/architecture/adr/0038-public-map-atlas-integration.md
  - docs/architecture/adr/README.md
  - docs/contracts/PUBLIC_MAP_ATLAS_CONTRACT.md
proven:
  - Oteryn-Platform main was reconciled at e4498ba9856a3779c8ae3a6f5bed608256a35fef before this task branch was created.
  - Otheryn main was reconciled at da553b1f2f157526e69e26d051ca3297db7abcf6; atlas PRs #381, #384, #387 and #389 are merged.
  - Otheryn full-world certification proves Z0-Z15 and exactly 3494 chunks for the certified build.
  - Current Otheryn full-world release workflow uploads per-floor verification JSON artifacts rather than complete generated atlas directories; observed current certification artifacts are only hundreds of bytes.
  - Current Otheryn viewer already owns chunked viewport loading, floors, render modes, search/details/overlays, URL state, environment animation and bounded caches.
  - Current Platform has no atlas route or navigation entry, no object-storage/CDN atlas configuration, same-origin-only CSP for script/connect/image, and a Synology Nginx layer that currently proxies all paths to the Platform application.
  - No open Oteryn-Platform PR or active-task owned path inspected for this preflight claims the four documentation paths owned here.
derived:
  - Implementing a public Map route before a complete producer artifact exists would create a dead or non-canonical product surface.
  - Copying OTBM parsing/build logic or maintaining a forked viewer in Platform would violate the repository source-of-truth boundary.
  - Initial same-origin consumption of a producer-built immutable artifact is the lowest-risk deployment model because it preserves current CSP/CORS boundaries and existing chunked browser behavior.
unknown:
  - Exact final Otheryn release descriptor/path and stable viewer asset-base/bootstrap API until the producer follow-up is implemented.
  - Exact production Map browser behavior because no public Map implementation exists yet.
conflicts: []
first_failure:
  marker: producer-consumption-artifact-missing
  evidence: Otheryn .github/workflows/otbm-atlas-full-world-release.yml uploads verification evidence only; no complete immutable atlas release is emitted for Platform consumption.
rejected_hypotheses:
  - Rebuild the atlas in Platform; rejected because Otheryn is the canonical producer and already owns the mature pipeline.
  - Publish a temporary iframe; rejected because it would not satisfy first-class Platform URL, shell, CSP and future integration requirements.
  - Use current object storage/CDN; rejected because no current Platform atlas storage/CDN contract exists and current CSP is same-origin only.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-public-map-atlas-integration.md
validation:
  - command: repository and cross-repository GitHub audit
    result: PASS
    evidence: current main heads, PR states, workflow sources, atlas runtime sources and Platform routing/deployment sources inspected through the GitHub connector
  - command: runtime/browser E2E
    result: BLOCKED
    evidence: producer release artifact and native viewer consumption contract do not yet exist, so no truthful canonical Platform Map implementation can be exercised
  - command: public production Map verification
    result: BLOCKED
    evidence: no production Map implementation is authorized before the producer contract exists
blockers:
  - Otheryn must publish the complete immutable atlas release artifact and stable viewer asset-base/bootstrap contract before Platform runtime implementation can continue without duplication.
next_action: In blakinio/Otheryn, implement and merge a producer-owned complete immutable atlas release plus a stable configurable viewer asset-base/bootstrap contract, then resume this Platform task against that exact build ID and digest.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task remains blocked on an explicit cross-repository producer contract before runtime implementation
source_branch_evidence: dedicated branch agent/oteryn-20260814-public-map-atlas-integration created from exact Platform main e4498ba9856a3779c8ae3a6f5bed608256a35fef
```

## Notes

The public product name is **Map** / **Mapa**. `OTBM Atlas` remains an implementation/provenance term, not the primary player-facing navigation label. Runtime implementation must stay absent until the producer artifact contract is real; this task intentionally does not create a placeholder backend, fake data source or dead `/map` route.