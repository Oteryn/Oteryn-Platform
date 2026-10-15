---
task_id: OTERYN-20261015-oracle-arm64-deployment
governing_issue: 1639
required_reads: []
search_first: []
optional_reads: []
---

# Oracle ARM64 deployment

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1639

The owner authorized installing and qualifying Game, Platform, gateway and their databases on the existing Oracle A1 host before migrating NAS data and switching traffic. No cutover before the real login/create/list path passes. Preserve all production trust, freshness, fencing and durability boundaries. No paid resources.

Owned paths: deploy/oracle/** and this packet. Game source is read-only, at 87f770c87ce17518ba69f5effdbd04725d8a7d06. Platform source base: 1194d5548760d9d773a1cebb182bc5986a6a42bf. Existing account repair PR #1637 is a separate candidate and is not silently included.

Native image builds are target-host acceptance of the ARM64 deployment using immutable public source archives; no repository checkout is edited or published through Remote Desktop. Runtime wiring is authored in this isolated task worktree. The host is accessed only through exact-call validated host exceptions. Native ARM64 deployment qualification has no existing equivalent CI job.

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T18:35:01Z
head: 1194d5548760d9d773a1cebb182bc5986a6a42bf
branch: codex/oracle-arm64-deployment-20261015
pr: 1640
status: implementing
context_routes:
  - deployment
owned_paths:
  - deploy/oracle/**
  - docs/agents/tasks/active/OTERYN-20261015-oracle-arm64-deployment.md
proven:
  - Oracle ARM64 4 OCPU 24 GB RAM 200 GB disk and persistent remote agent
  - Docker 29.1.3 Compose 2.40.3 and buildx 0.30.1 installed
  - Platform and gateway ARM64 builds passed from immutable source 1194d5548760
derived: []
unknown:
  - Game ARM64 image build and full-stack qualification results
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/oracle/game.Dockerfile
  - deploy/oracle/prepare-platform.sh
  - deploy/oracle/README.md
validation:
  - command: target docker version and docker compose version
    result: PASS
    evidence: verified on Oracle through its remote agent
blockers:
  - none
next_action: finish native ARM64 image builds and wire the isolated stack
```

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

Owner explicitly approved temporarily removing only sudo from the Oracle agent blockedCommands for this deployment. Original list is preserved; restore it when deployment work stops. No other agent control is changed.
