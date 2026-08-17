# Oteryn Ecosystem Repository Migration — Wave 1 Readiness

Date: 2026-08-17
Issue: #1130
Programme: `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION`
Authority: ADR 0041 remains the latest located ecosystem-topology authority on `Oteryn-Platform/main` for this wave.

## Decision

**Wave-1 verdict: NO_GO for physical repository cutover.**

This is a fail-closed readiness decision, not a rejection of the accepted four-repository architecture. The accepted target remains:

1. `Oteryn` — narrow META/architecture repository;
2. `Oteryn-Game` — continuation of the `Oteryn-v2` Game lineage;
3. `Oteryn-Platform` — existing Platform repository, not split;
4. `Oteryn-Atlas` — selectively extracted derived-data product.

The physical cutover is not yet safe because the destination organization is unresolved/not visible to the authenticated GitHub account, external GitHub Actions/reusable-workflow callers of `Oteryn-v2` have not been exhaustively proven, GHCR/package inventory is unavailable to the current integration, and current `Otheryn` Atlas automation includes an active private Synology deployment path that cannot be moved blindly.

## Fresh live-state baseline

The following SHAs are observation baselines only. They are not cached as future authority and must be refreshed before any later mutation.

| Repository / target | State observed | Baseline |
| --- | --- | --- |
| `blakinio/Oteryn-Platform` | exists, protected `main` | `3724349a8738bff8229b83239244120876bdedfd` |
| `blakinio/Oteryn-v2` | exists, protected `main` | `c8a3ac666845f3e8679e55dd4c84d1e440e830c8` |
| `blakinio/Otheryn` | exists, protected `main`; Atlas deployment activity advanced during this audit | `5001cb42b9027d763d87a099696136acd3e12e83` |
| `blakinio/Oteryn` | not found at inspection | n/a |
| `blakinio/Oteryn-Game` | not found at inspection | n/a |
| `blakinio/Oteryn-Atlas` | not found at inspection | n/a |

Authenticated GitHub organization membership inspection returned an empty organization set. Therefore creating temporary repositories under `blakinio` would not satisfy ADR 0041's future-organization topology and is intentionally rejected.

## Architecture verdict

**FACT:** ADR 0041 defines the four-repository target above, keeps Platform as one repository, identifies `Oteryn-v2` as the Game lineage, and requires selective rather than wholesale Atlas extraction.

**FACT:** No later repository search result was found that supersedes ADR 0041 for ecosystem topology.

**INFERENCE:** The architecture remains valid, but its physical organization owner has not yet been established in the authenticated account. This blocks META bootstrap and makes cross-owner transfer planning premature.

## META — `Oteryn`

Status: **ARCHITECTURE_READY / PHYSICAL_BLOCKED**.

The META repository should remain narrow: architecture authority, ecosystem ownership map, contribution/security policy and links. It must not become a duplicate schema/runtime repository.

Blockers:

- no authenticated GitHub organization membership is currently visible;
- the intended future organization identity therefore cannot be proven;
- the available GitHub connector has no repository-create/rename/transfer action exposed;
- creating `blakinio/Oteryn` as a temporary substitute would create topology debt and is not authorized.

## OTERYN-GAME — `Oteryn-v2` -> `Oteryn-Game`

Status: **NO_GO_YET**.

Positive evidence:

- target `blakinio/Oteryn-Game` was not found, so there is no observed same-owner name collision;
- current GitHub Releases API for `Oteryn-v2` returned an empty release list at the inspected baseline;
- `.github/repository-policy.json` contains no hard-coded repository coordinate;
- `tools/repository/apply_github_settings.py` derives the repository from `GITHUB_REPOSITORY`;
- `.github/workflows/repository-configuration.yml` is same-repository/dynamic;
- `.github/workflows/merge-gate.yml` uses `${{ github.repository }}` for inspected repository API calls;
- the current recursive tree contains no `action.yml` path;
- `game-atlas-profile-spike.yml` checks out and runs only local paths.

These are non-coordinate safety signals. They are deliberately not placed in A-G redirect classes because they do not themselves represent old repository coordinates that need redirect/cutover treatment.

Remaining cutover gates:

1. **External Actions/reusable workflows.** GitHub documents that Actions references to a renamed repository do not follow ordinary repository redirects. Repository-local inspection and connector code search did not prove every caller in every accessible/external repository. Any caller using `blakinio/Oteryn-v2/.github/workflows/...@...` or a repository-hosted action would need a cutover edit.
2. **GHCR/packages.** The package endpoint returned `403 Resource not accessible by integration`; exact image/package names, links, workflow access and consumers are therefore UNKNOWN.
3. **Open work.** At inspection, open draft PRs #314 and #317 exist in `Oteryn-v2`. A rename may preserve normal PR state, but active branches/checkpoints and any external automation must be revalidated immediately before cutover.
4. **Brand/current-name cleanup.** `README.md` still presents `Oteryn v2`; current-name presentation strings are post-rename cleanup, not rename blockers.

Rollback condition: if the rename is later executed, do not reuse the old repository name while redirect-based rollback/compatibility is required.

## OTERYN-PLATFORM

Status: **KEEP_AS_ONE_REPOSITORY / NO_PHYSICAL_CHANGE_IN_WAVE_1**.

The two inspected native-protocol workflows are repository-local and do not embed an `Oteryn-v2` Actions coordinate. This is useful negative evidence but is not treated as an exhaustive all-history/all-file search. Platform transfer to a future organization is blocked by the same unresolved organization destination and is not coupled to the Game rename in this wave.

## OTERYN-ATLAS

Status: **EXTRACTABLE_WITH_REFACTOR / PHYSICAL_NO_GO**.

Fresh evidence materially strengthens the need for selective extraction:

- `Otheryn/main` advanced during the audit to `5001cb42...` with `ops(atlas): request private Synology deployment`;
- `.github/workflows/otbm-atlas-deploy-request.yml` validates an explicit `private-synology` request and dispatches the canonical full-world workflow using `GITHUB_REPOSITORY`;
- `.github/workflows/otbm-atlas-full-world-16.yml` has a guarded deployment mode, a self-hosted `[ots, synology]` receiver job, `/volume1/docker/otheryn/atlas/...` state, temporary receiver/tunnel/network resources, and copies `tools/otbm_atlas`, `tools/otbm_atlas_facts` plus deployment control paths into the generation;
- the workflow also consumes provenance-pinned map/assets under `vendor/map-analysis/...` and generates `build/**` artifacts.

Therefore `tools/otbm_atlas/**` cannot be copied wholesale and called complete. The extraction must distinguish Game-owned export/source authority, Atlas-owned transformation/index/render logic, deployment control, provenance-pinned inputs and generated artifacts. `build/**` remains generated output and must not be migrated as source.

The active Synology path is an extraction/deployment blocker, not an A-G GitHub redirect classification. No Synology runner, tunnel, deployment, private state, DNS, production environment or secret was mutated by this wave.

## GitHub rename / transfer impact

GitHub's documented repository rename redirect is useful for normal web/git references, but is not a safety substitute for a coordinate inventory. In particular, GitHub Actions and reusable-workflow references hosted in a renamed repository do not follow the ordinary redirect and must be updated by callers. Pages has separate rename/transfer caveats. Package access/linkage also requires separate verification when repository ownership changes.

The cutover plan therefore treats ordinary historical web/git references differently from executable Actions and package coordinates.

## Coordinate classification summary

Canonical machine-readable inventory: `docs/architecture/migration/oteryn-repository-coordinate-inventory.json`.

- **A — MUST_CHANGE_BEFORE_RENAME:** no concrete A entry is proven in Wave 1. This does not imply none exist; G entries must be resolved first.
- **B — MUST_CHANGE_AT_CUTOVER:** no concrete B entry is proven in Wave 1. Any discovered external reusable-workflow/action caller of the old Game coordinate becomes B.
- **C — MUST_CHANGE_AFTER_RENAME:** current non-historical name/branding claims such as `README.md` after the physical rename.
- **D — SAFE_VIA_GITHUB_REDIRECT_TEMPORARILY:** no concrete D entry is declared until an exact ordinary web/git reference is enumerated and proven redirect-safe.
- **E — HISTORICAL_PROVENANCE_DO_NOT_REWRITE:** evidence, archived checkpoints and historical ADR/PR references whose old coordinate is part of provenance.
- **F — LEGACY_REFERENCE_INTENTIONALLY_PRESERVE:** `blakinio/canary` and `blakinio/otclient` legacy/reference coordinates.
- **G — UNKNOWN_REQUIRES_EVIDENCE:** future organization/target-owner coordinates, GHCR/packages, exhaustive external Actions/reusable-workflow callers, current ordinary links before exhaustive classification, broader Platform current references, and the Atlas target coordinate.

Dynamic same-repository workflow behavior, Releases state, Atlas generated/reference paths and Synology deployment facts are recorded separately as non-coordinate evidence rather than forced into A-G.

## CI / GHCR / release impact

| Surface | Current evidence | Readiness |
| --- | --- | --- |
| Game repository-local CI/control plane | mostly dynamic via GitHub context/env; no repository-hosted `action.yml` observed | positive |
| external Actions/reusable callers | not exhaustively proven | **BLOCKER** |
| Game GitHub Releases | no releases returned at inspected baseline | positive, refresh at cutover |
| Game GHCR/packages | integration denied package inventory (403) | **BLOCKER** |
| Platform cross-repo caller surface | two migration-relevant native-protocol workflows inspected; no old executable coordinate there | partial only |
| Atlas CI | extensive active `otbm-atlas-*` workflows | requires selective migration |
| Atlas production/deployment | active private Synology path in Otheryn | **BLOCKER to blind extraction** |

## Changes made by Wave 1

- created issue #1130 as the Wave-1 ownership anchor;
- reconstructed current source/target topology;
- inspected current migration-critical Game control-plane paths;
- inspected migration-relevant Platform native-protocol workflows;
- revalidated current Otheryn Atlas deployment topology against the latest observed main;
- recorded coordinate classification and Atlas extraction manifest;
- made no product/runtime/repository-rename/transfer/create/deployment mutations.

## Blockers

### P0

None requiring emergency rollback; no physical cutover was attempted.

### P1

1. Exact future `Oteryn` GitHub organization is not available/visible to the authenticated account.
2. GHCR/package inventory for `Oteryn-v2` is unavailable through the current integration (`403`).
3. External Actions/reusable-workflow caller inventory is not exhaustive; GitHub rename redirects do not protect this execution path.
4. Current Atlas has a live Otheryn-bound Synology deployment pipeline and mixed ownership that requires path-level extraction design.

### P2

- rename-era branding/current-name cleanup;
- ordinary links that become class D only after exact enumeration and redirect-safety proof;
- historical provenance references that should remain unchanged.

## Intentionally unchanged

- `blakinio/Oteryn-v2` repository name and ownership;
- `blakinio/Oteryn-Platform` repository name and ownership;
- `blakinio/Otheryn` repository and its active Atlas deployment;
- all `blakinio/canary` and `blakinio/otclient` content;
- Synology, DNS, production environments, secrets, packages, payments and live game state.

## One next action

**Owner action: create or identify the intended future `Oteryn` GitHub organization and make it visible to the authenticated GitHub account.**

That is the dependency-safe next action because ADR 0041 puts META before product transfer/rename, and creating temporary same-user target repositories would violate the accepted topology. After the organization exists, the executor can bootstrap META there and continue package/Actions evidence collection before any Game cutover.
