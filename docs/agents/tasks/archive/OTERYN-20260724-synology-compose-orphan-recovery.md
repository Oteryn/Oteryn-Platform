---
task_id: OTERYN-20260724-synology-compose-orphan-recovery
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - active tasks and open PRs touching deploy/synology or the oteryn-staging runner
  - existing Synology deployment and rollback scripts
optional_reads:
  - deploy/synology/README.md
---

# OTERYN-20260724-synology-compose-orphan-recovery

## Goal

Recover the live Synology staging stack from Docker Compose replacement containers left with short-ID-prefixed names that Synology Container Manager could not manage, without restarting containers or touching persistent volumes.

## Acceptance criteria

- [x] Recovery detects names matching `<12-hex>_oteryn-staging-<service>-<index>`.
- [x] A candidate is changed only after exact Compose project/service labels are verified.
- [x] Canonical name collisions fail closed rather than overwriting another container.
- [x] Named volumes and unrelated containers, including `liquid20-collector`, are never removed or restarted.
- [x] Prefixed Platform and Canary replacement names are absent and canonical Compose names exist.
- [x] Platform, Gateway and Canary are running after repair.
- [x] Public native OAuth client id `019f93b3-5e7e-721c-bc23-7e22e4bee7cc` is captured as sanitized non-secret evidence.
- [x] The temporary automatic push trigger is removed; bounded manual repair remains available.

## Ownership

```yaml
owned_paths:
  - .github/workflows/repair-synology-compose-orphans.yml
  - docs/agents/tasks/active/OTERYN-20260724-synology-compose-orphan-recovery.md
  - docs/agents/tasks/archive/OTERYN-20260724-synology-compose-orphan-recovery.md
modules:
  - Synology staging deployment
  - Docker Compose recovery
  - self-hosted deployment runner boundary
dependencies:
  - existing oteryn-staging self-hosted runner
  - private issue 163 retained as sanitized operational evidence
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T00:55:00+02:00
head: dd03949d5e4f3555087d43b52cac0fa4859e6bfd
branch: chore/OTERYN-20260724-synology-compose-recovery-closeout
pr: 169
status: ready
context_routes:
  - agent-governance
  - testing
  - ci-repair
owned_paths:
  - .github/workflows/repair-synology-compose-orphans.yml
  - docs/agents/tasks/active/OTERYN-20260724-synology-compose-orphan-recovery.md
  - docs/agents/tasks/archive/OTERYN-20260724-synology-compose-orphan-recovery.md
proven:
  - Initial DSM evidence showed short-ID-prefixed Platform and Canary names and Container undefined errors.
  - PR 162 merged as a9498dacd8a18b51f1bfd940f61796ca76bb393e with fail-closed label-verified docker rename recovery.
  - PR 164 merged as 63c7d7e9b6dc2584919be0fadd64f68f8d7ef0d1 with sanitized private issue reporting.
  - PR 166 merged as e9cb018d609f2bb5064370a5f850ddd091083258 with separated repair and OAuth outcomes.
  - Synology execution 30129145948 reported repair verification success, no remaining prefixed candidates, and canonical platform, canary and gateway running.
  - PR 167 merged as 2b097861ec026b004718937a8e3bb3047d055ccd with terminal-punctuation-safe OAuth id parsing.
  - Synology execution 30129638273 reported repair verification success, OAuth read success, repaired services none, canonical platform, canary and gateway running, and public OAuth client id 019f93b3-5e7e-721c-bc23-7e22e4bee7cc.
  - No recovery run removed volumes, restarted unrelated containers or published secrets/raw Docker inspect data.
  - The Windows LAN-ready client package was generated with the exact public OAuth client id and requires no SSH or first-run identifier input.
  - PR 169 exact head dd03949d5e4f3555087d43b52cac0fa4859e6bfd passed CI 30129831642, Agent Governance 30129831738, Platform DB Outage Validation 30129831711, Phase 7 Production-Like Validation 30129831716 and Game Auth Ticket Concurrency 30129831768.
derived:
  - A bounded earlier run restored canonical names; later idempotent runs found no remaining prefixed candidates.
  - Synology Container Manager can now address the canonical Platform and Canary containers instead of resolving an undefined replacement reference.
unknown: []
conflicts: []
first_failure:
  marker: synology-container-manager-undefined-container
  evidence: DSM could not attach to the prefixed Platform container and reported Container undefined does not exist.
rejected_hypotheses:
  - restart the entire Container Manager package: rejected because it would interrupt unrelated containers including liquid20-collector
  - delete and recreate containers: rejected because verified docker rename repaired names without downtime
  - delete Docker volumes: rejected because the defect was container metadata/name state, not persistent data
changed_paths:
  - .github/workflows/repair-synology-compose-orphans.yml
  - docs/agents/tasks/active/OTERYN-20260724-synology-compose-orphan-recovery.md
  - docs/agents/tasks/archive/OTERYN-20260724-synology-compose-orphan-recovery.md
validation:
  - command: PR 162, PR 164, PR 166 and PR 167 exact-head repository validation
    result: PASS
    evidence: each PR passed required CI, governance, outage, Phase 7 and concurrency checks before squash merge.
  - command: Synology repair execution 30129145948
    result: PASS
    evidence: canonical runtime verification and OAuth read succeeded; no remaining prefixed candidate required rename.
  - command: Synology corrected evidence execution 30129638273
    result: PASS
    evidence: canonical platform, canary and gateway were running and exact OAuth client id was published without terminal punctuation.
  - command: PR 169 exact-head repository checks on dd03949d5e4f3555087d43b52cac0fa4859e6bfd
    result: PASS
    evidence: CI, governance, outage, Phase 7 and concurrency checks all passed.
blockers:
  - none
next_action: Squash-merge PR 169 and close private evidence issue 163 as completed.
```
