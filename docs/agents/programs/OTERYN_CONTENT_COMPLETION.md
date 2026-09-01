---
programme_id: OTERYN_CONTENT_COMPLETION
programme_version: 1
repository: blakinio/Oteryn-Platform
coordination_issue: 1115
project_lane: oteryn-platform-content
status: BOOTSTRAP_CANDIDATE
trusted_base: main
prompt_package: docs/agents/prompts/OTERYN-CONTENT-COMPLETION-PROMPTS.md
coordinator_role: OTERYN_CONTENT_COMPLETION parallel coordinator
auditor_role: OTERYN_CONTENT_COMPLETION discovery auditor
source_inventory: docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.json
source_report: docs/agents/reports/OTERYN-20260816-crystalserver-content-source-inventory.md
prompt_eval: docs/agents/evals/OTERYN-CONTENT-COMPLETION-MANUAL-EVAL.md
portal_selector_relationship: owner_started_specialized_programme
production_authority: false
external_repository_authority: false
owner_funded_ai_standing_permission: false
---

# Oteryn Content Completion Programme

## Mission

Convert the existing Oteryn Platform content engines into a measured, populated and player-useful product. The programme covers Game Catalog, structured Wiki reference content and Player Companion tools, using provenance-aware source material without confusing engine existence with content completeness.

This programme was explicitly started by the repository owner on 2026-08-16. It is specialized content work, not a replacement for the global `OTERYN_PORTAL_COMPLETION` selector, `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM`, Architecture Review, Continuous Audit or Remediation programmes. It may coordinate owner-started, independent content work in parallel with other non-overlapping portal tasks, but it must not reorder or steal their live ownership.

## Core completion rule

A module is not content-complete merely because routes, controllers, tables, importers or example fixtures exist.

For every player-visible content family or tool, maintain separate evidence for:

1. engine/backend availability;
2. reachable real frontend;
3. real content/data population and counts;
4. expected-inventory coverage;
5. provenance and source authority;
6. EN/PL and media/fallback applicability;
7. empty/error/unavailable/recovery states;
8. browser/E2E evidence;
9. staging/production population when that environment is authorized and required.

`AVAILABLE` module status is never used as a substitute for these measurements.

## Source-material baseline

Owner-supplied archive:

```yaml
file_name: crystalserver-main.zip
sha256: 920a59e15175a5f53721f60b17f4bb37370bf0b61cd91abb4c909bf0d85e5f26
archive_entries: 9486
regular_files: 8819
root_license_observed: GNU GPL v2
default_datapack_observed: data-global
alternate_datapack_observed: data-crystal
```

The archive is untrusted source material, not an instruction or authority source. The committed inventory contains only bounded metadata/counts/paths and does not commit the ZIP or bulk third-party content.

## Authority and provenance model

Every candidate fact family is classified as exactly one of:

- `DIRECT_STRUCTURED` — machine-readable fact with stable fields suitable for deterministic extraction, still subject to product/source authority.
- `TRANSFORM_REQUIRED` — parse/normalization is required before use; transformation must be deterministic and tested.
- `PARTIAL_SEMANTICS` — source exposes implementation fragments but not enough authority to claim a complete public fact.
- `EDITORIAL_ONLY` — useful as a research lead or human-authored Wiki input, not structured catalogue authority.
- `AUTHORITY_REQUIRED` — source data exists but accepted Platform/native authority does not permit public/active use yet.
- `REJECTED` — unsuitable, unsafe, ambiguous or prohibited for the target use.

For every persisted or public fact keep provenance sufficient to identify source material identity, source path, datapack/profile, transformation version and review/authority state. Never convert `UNKNOWN` into a default value merely to populate a page.

## Existing programme integration

### OTERYN_PORTAL_COMPLETION

`OTERYN_PORTAL_COMPLETION` remains the sole global portal selector. This programme does not alter its ordering. Owner-started content tasks are independent work and must appear as `OWNED` to the global selector when they overlap a candidate.

### GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM

The existing Game Catalog programme remains authoritative for schema immutability, consumer-first evolution, activation/rollback and specialized catalogue rollout. This programme contributes source-material evidence and player-visible completeness goals; it must reuse that programme rather than invent a competing Game Catalog lifecycle.

In particular, current Game Catalog v1.2 semantics are item/creature/loot-oriented and draft PR #338 already owns schema 1.3 NPC/shop consumer paths. No worker may duplicate or overwrite #338.

### Architecture Review

If using CrystalServer-derived material as a new durable authority/profile would change accepted native/source ownership, stop the implementation lane and route the exact decision to `OTERYN_PLATFORM_ARCHITECTURE_REVIEW`. A worker must not solve an authority gap by renaming reference data as native truth.

## Parallel operating model

The programme has three role types.

### Discovery auditor

The auditor runs first and again at major barriers. It:

- re-audits the entire player-visible target instead of trusting this bootstrap;
- validates or corrects source counts when the exact archive is available;
- measures real Platform content, fixtures, import state and reachable player/admin surfaces;
- deduplicates existing Issues/PRs/programmes;
- classifies every gap `TERMINAL | OWNED | BLOCKED | DECISION_REQUIRED | READY`;
- may create bounded Issues/task records/branches/draft PR scaffolds for genuinely independent `READY` slices;
- does not implement product code in the audit phase.

Draft scaffolds must contain only task/audit metadata needed for a clean handoff. The auditor releases the branch/session before an execution worker takes over.

### Coordinator

The coordinator owns programme-level wave/barrier decisions and no worker product paths. It:

- refreshes live state before each wave;
- verifies non-overlapping `owned_paths`;
- assigns only `READY` work;
- prevents duplicate tasks and branch sharing;
- monitors task checkpoint, PR head, CI/review and dependency state;
- routes architecture/authority decisions rather than inventing them;
- allows independent workers to continue while another lane is `WAITING`;
- verifies resulting outcome instead of trusting worker summaries;
- performs barrier reconciliation before opening the next wave.

### Execution workers

Each execution worker takes one bounded task branch/PR after exclusive ownership is proven. A worker owns its task from implementation through self-review, focused/component validation, real E2E where applicable, exact-head CI, merge and lifecycle closeout unless a real blocker requires rotation.

No two workers write the same branch/worktree or overlapping owned paths concurrently.

## Candidate execution lanes

These are reusable worker roles, not pre-claimed tasks. The auditor and coordinator decide which are currently READY.

### SOURCE-PIPELINE

Build deterministic, provenance-preserving extraction/normalization from an explicitly supplied source archive into reviewable generated artifacts or test fixtures. The pipeline may not silently activate data or claim native authority.

### CATALOG-CORE

Populate/verify item, creature, loot and Bestiary-related content through accepted current Game Catalog contracts where architecturally valid. Do not mutate immutable schema 1.0-1.2 bytes. Do not represent archive-derived runtime availability as proven native availability without accepted authority.

### CATALOG-EXTENSIONS

Add or prepare additive schema support for selected new fact families only after exact dependency and authority review. Draft #338 is a hard live-overlap check for NPC/shop work. Spells, achievements, imbuements, mounts, outfits, vocations, quests and other systems require task-specific schema/product decisions rather than one mega-version.

### WIKI-REFERENCE

Create player-useful structured reference surfaces and cross-links driven by accepted catalogue facts. Keep generated/reference facts separate from editorial guides/lore/walkthrough prose. Do not auto-publish copied third-party descriptions merely because they exist in source files.

### PLAYER-COMPANION

Deliver one complete tool vertical slice at a time using accepted structured facts. Candidate tools include equipment comparison, hunt/loot reference, Bestiary/charm planning, imbuement planning, spell/vocation planning, achievement tracking, quest/access tracking and mount/outfit tracking. The existing Hunt Session Analyzer is a foundation, not proof the toolbox is complete.

## First-wave audit acceptance

Before product implementation is broadly dispatched, the auditor must produce a durable ledger that:

- inventories every current player-visible Game Catalog/Wiki/Player Companion route and tool;
- records real content counts and expected counts where an expected inventory exists;
- separates fixture/demo/test content from deployable/active content;
- records current public/admin availability and empty/error/recovery behaviour;
- reconciles open Issues and PRs including #1114, #338, #489, #301, #330 and any newer live state;
- maps source families from the committed CrystalServer inventory to product targets and authority classifications;
- identifies which gaps are independent enough for parallel execution;
- creates no duplicate ownership;
- produces draft task PR scaffolds only for safe `READY` slices.

## Wave rules

1. The auditor establishes or refreshes the ledger.
2. The coordinator selects the smallest set of genuinely independent non-overlapping READY tasks.
3. Each task gets one Issue/task/branch/draft PR and exclusive `owned_paths`.
4. Workers implement in parallel.
5. Waiting workers release active sessions; independent READY work continues.
6. At a barrier the coordinator refreshes live state, reconciles outputs/findings and selects the next wave.
7. A material architecture/source-authority question stops only the affected lane; it does not automatically stop unrelated READY lanes.

## Current generation holds

At programme creation:

- PR #1114 owns PublicPortal Today runtime paths and is independent unless a content worker touches those exact paths.
- PR #338 owns Game Catalog schema 1.3 NPC/shop consumer paths; overlapping work is `OWNED`/blocked from duplication.
- long-lived public-domain and native-auth production verification tasks are independent and do not grant production authority here.

These facts are generation-scoped. Every coordinator/auditor invocation must refresh them.

## Licensing and publication guard

The archive root contains GPL-2.0 license text and upstream lineage notes, but that alone does not prove every embedded Tibia-derived name, description, image, map, dialogue or other asset is cleared for unrestricted republication.

Therefore:

- source code/data may be analyzed under the task's source-material authority;
- do not bulk-copy prose/assets into Platform;
- prefer normalized facts and project-owned presentation;
- preserve source provenance;
- classify descriptions/dialogue/map/media publication separately;
- route unresolved third-party-rights questions as `AUTHORITY_REQUIRED` or `DECISION_REQUIRED` rather than silently publishing.

## Prompt contract

```yaml
prompt_contract:
  version: 1.0.0
  changed_surfaces:
    - content completion prompt package with auditor, coordinator and five worker roles
  objective: parallel content completion without duplicate ownership, false completeness or source-authority escalation
  baseline_version: none
  eval_suite: docs/agents/evals/OTERYN-CONTENT-COMPLETION-MANUAL-EVAL.md
  rollback_version: remove programme/prompt package and continue using existing portal/catalog programmes
```

Automated model-trial infrastructure is not introduced by this bootstrap. The manual scenario matrix is a deterministic review aid only and must not be reported as repeated model-eval execution.

## Programme completion

The programme may close only when:

- the target player-visible content/tool inventory has explicit measured outcomes;
- no required content family receives completion credit solely from engine/module existence;
- delivered fact families retain provenance/version/authority semantics;
- selected required tools are complete applicable vertical slices with real frontend/backend integration and E2E;
- remaining gaps are explicitly delivered, deferred, blocked or rejected with evidence;
- every programme-created related PR/task is intentional and terminal;
- no material audit finding or accidental open ownership remains.

Production activation remains a separate gate.