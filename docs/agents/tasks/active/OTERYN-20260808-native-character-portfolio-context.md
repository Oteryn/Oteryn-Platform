# OTERYN-20260808-native-character-portfolio-context

```yaml
task_id: OTERYN-20260808-native-character-portfolio-context
repository: blakinio/Oteryn-Platform
programme: OTERYN_PLATFORM_ARCHITECTURE
project_lane: oteryn-platform-core
mode: ARCHITECTURE_CONTINUATION
task_kind: discovery
phase: design
status: blocked
base_branch: main
base_sha: b3788c3414b716743baa0500903b02f2e64cca7f
branch: docs/OTERYN-20260808-native-character-portfolio-decision
issue: 857
pull_request: pending
owner: ChatGPT architecture coordinator
implementation_authorized: false
runtime_changes_authorized: false
cross_repository_runtime_changes_authorized: false
created_at: 2026-08-08T00:18:00+02:00
updated_at: 2026-08-08T00:32:00+02:00
execution_mode: github
decomposition_decision: single
context_pressure: medium
```

## Goal

Define the Platform-side **Native Character Portfolio / Account Center v2** boundary before new native consumers such as PlayerCompanion or PlatformAPI copy Canary numeric identifiers and direct Canary assumptions into their target architecture.

This task is architecture/documentation only. It does not authorize Laravel runtime changes, database migrations, Oteryn-v2/Canary writes, protocol-wire changes, deployment or production operations.

## Current verified state

### Platform main

Verified base:

`b3788c3414b716743baa0500903b02f2e64cca7f`

Current Account Center implementation is delivered and valid for the Canary compatibility path, but `app/Accounts/ReadModels/AccountOverviewReadModel.php` currently:

- resolves `IdentityCanaryAccount`;
- uses numeric `canary_account_id`;
- calls `CanaryGameDataRepository::activeCharactersForAccount()`;
- declares `CHARACTER_LIMIT = 10`;
- derives `character_creation_allowed` from the returned active-row count.

`app/CharacterProfiles/Models/CharacterProfilePreference.php` currently stores numeric `canary_player_id` for Platform-owned presentation/privacy preferences.

`app/CharacterProfiles/**` is a real top-level source package, but the top-level table in `MODULE_CATALOG.md` does not classify `CharacterProfiles`.

### Accepted Platform authority

The candidate has been reconciled against:

- ADR 0001 — Laravel modular monolith;
- ADR 0008 — Public Portal / Account Center / Admin shell architecture;
- ADR 0025 — PlayerCompanion boundary;
- ADR 0028 — Platform-issued canonical native `AccountId`;
- ADR 0029 — Platform-issued `WorldId` / `ChannelId`;
- `DATA_OWNERSHIP.md`;
- `MODULE_CATALOG.md`;
- `PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- `ARCHITECTURE_AUTHORITY.md`.

ADR 0008's statement that a general Account Overview was not yet delivered is historical context from its acceptance date. Current source proves Account Center now exists. Historical ADR text must not be rewritten to erase that timeline.

### Oteryn-v2 read-only verification

Current Oteryn-v2 main retains the merged Character Authority decision from PR #90 / ADR-0012:

- Platform Identity owns/issues `AccountId`;
- Oteryn-v2 Character Authority owns/issues `CharacterId`;
- current `AccountId <-> CharacterId` ownership is game-domain authoritative;
- Platform reads are authorized projections and caches/read models are non-authoritative;
- create/rename/delete/restore/finalize/world transfer/account/Bazaar transfer are native game-owned mutations;
- legal rename/world/account transfer preserves `CharacterId`;
- terminal deletion never reuses `CharacterId`;
- Platform direct SQL writes to native character tables are not the steady-state target.

No cross-repository write is authorized by this task.

## Architecture analysis result

A new durable Platform ADR is required because the decision changes long-lived module responsibility, native identity use, capability/freshness semantics and migration direction.

Issue #857 records three options.

### Recommended Option A

Use an **Accounts-owned Character Portfolio application/read boundary** inside the existing Laravel modular monolith.

Target responsibility split:

- `Accounts` — authenticated Account Center and Character Portfolio composition;
- `Characters` — Platform orchestration of explicitly approved native character commands;
- Oteryn-v2 Character Authority — authoritative `CharacterId`, ownership, lifecycle, game-domain capabilities and mutation results;
- `PublicGameData` — public/general game read projections, not Account Center ownership proof;
- `CharacterProfiles` — Platform-owned presentation/privacy preference subdomain;
- `PlayerCompanion` — consumes owned-character context through the accepted Platform boundary and its game facts/rules through its already accepted data sources.

The proposal is recorded as `docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md` with lifecycle `Proposed`.

## Key proposed invariants

1. Native Account Center never uses Oteryn-v2 physical database shape as ownership or mutation authority.
2. Canonical native preference references use `CharacterId`; current `canary_player_id` remains explicit compatibility state until a later additive migration.
3. A portfolio projection carries semantic identity, revision/observation and failure/freshness state; dependency failure is never converted to an empty portfolio.
4. Cached/stale projection never authorizes mutation or proves current ownership.
5. Native character creation/rename/delete/transfer eligibility is not reconstructed from row counts.
6. Game-owned and Platform-owned capability gates remain distinct and are combined fail-closed by the application layer.
7. If a Platform entitlement changes a game-domain limit, it crosses an explicit versioned contract to the authoritative game capability/command path; Platform does not locally bypass game policy.
8. `ChannelId` is not a baseline durable character-portfolio field; channel is topology/runtime placement and is included only for a separately justified runtime/session use case.
9. Canary -> `CharacterId` migration is additive, mapping-driven and reversible; no hash/truncation/implicit identifier derivation.
10. No new microservice is created without measured scaling/isolation/lifecycle need.

## Alternatives

### Option B — PublicGameData owns the authenticated portfolio

Rejected by recommendation because it mixes public-read semantics with private ownership-sensitive Account Center composition.

### Option C — standalone CharacterPortfolio module/service

Rejected by recommendation because no independent deployment/scaling/security-isolation requirement is proven.

## Decision backlog

`ARCH-DEC-0001` is registered in `ARCHITECTURE_DECISION_BACKLOG.json` as `decision_required`.

Blocking owner question:

> Accept Option A for the Native Character Portfolio / Account Center v2 boundary, or choose Option B or C?

No option is accepted by this task record itself.

## Validation classification

```yaml
self_review:
  result: PASS
  exact_head: pending
  acceptance_checked: true
  full_diff_checked: pending
  negative_paths_checked: NOT_APPLICABLE
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - current Platform AccountOverviewReadModel and CharacterProfilePreference source
    - ADR 0001/0008/0025/0028/0029
    - DATA_OWNERSHIP.md
    - MODULE_CATALOG.md
    - PORTAL_COMPLETENESS_ARCHITECTURE.md
    - Oteryn-v2 merged PR #90 / ADR-0012
```

E2E is `NOT_APPLICABLE`: this package changes architecture/task documentation only and creates no executable user or integration journey.

## Context checkpoint

```yaml
last_progress: Reconciled the native Character Portfolio candidate against current Platform main and current Oteryn-v2 Character Authority authority; opened Issue #857 and prepared Proposed ADR 0030 plus the machine-readable decision record.
status: blocked
branch: docs/OTERYN-20260808-native-character-portfolio-decision
head_sha: pending
pr: pending
issue: 857
active_architecture_decision_id: ARCH-DEC-0001
owner_action_required: Choose Option A, B or C in Issue #857 / the architecture conversation.
blocker: Explicit repository-owner acceptance is required before ADR 0030 may be promoted from Proposed to Accepted and canonical focused architecture documents may be reconciled.
next_action: After the repository owner selects an option, update ADR 0030 and the focused canonical architecture documents in this same bounded task; do not change runtime code.
```
