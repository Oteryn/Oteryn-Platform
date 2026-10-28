---
task_id: OTERYN-20261016-n4p4-db1-game-amendments
governing_issue: 1559
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261016 N4P-4 DB-1 and Game architect amendments

## Goal

Governing GitHub Issue: #1559 — canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Docs only. Apply the Game architect decisions D1076–D1080 and owner decisions Q1/Q2 (issue #1559, comments 6217008134 and 6217029052): ADR 0045 amendments A1–A4, contract §19 amendments (A1, A3, session-generation fence, migration number), and a DB-1 amendment section in the remembered-device candidate. Candidate status, default-OFF and testing/preproduction scope are unchanged.

## Acceptance criteria

- [x] ADR 0045 carries A1–A4.
- [x] Login contract carries A1, A3, the session-generation fence and the migration-number wording.
- [x] Remembered-device candidate carries the DB-1 amendment and stays CANDIDATE, default OFF, not approved for production.
- [x] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/active/OTERYN-20261016-n4p4-db1-game-amendments.md
  - docs/agents/tasks/archive/OTERYN-20261016-n4p4-db1-game-amendments.md
modules:
  - game-auth
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - OTV2-20261012-recovery-locator-producer
  - OTV2-20261016-recovery-profile-g2
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-28T14:30:00Z
head: db6ba28f182b6760d824d12a4bff9569c2f21a34
branch: claude/n4p4-db1-game-amendments
pr: 1651
status: completed
context_routes:
  - docs
owned_paths:
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md
  - docs/agents/tasks/active/OTERYN-20261016-n4p4-db1-game-amendments.md
  - docs/agents/tasks/archive/OTERYN-20261016-n4p4-db1-game-amendments.md
proven: []
derived: []
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/contracts/NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md
validation:
  - command: governance and documentation checks
    result: NOT_RUN
    evidence: see PR
blockers:
  - none
next_action: none; delivered by PR 1651 and archived
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: PR 1651 merged as db6ba28f182b6760d824d12a4bff9569c2f21a34
```

## Notes

Remembered-device DB-1 stays a candidate until DB-9 security review and Platform acceptance. No production authority is granted.
