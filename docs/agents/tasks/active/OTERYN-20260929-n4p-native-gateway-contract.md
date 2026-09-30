---
task_id: OTERYN-20260929-n4p-native-gateway-contract
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_PRE_ADMISSION_HANDOFF_CONTRACT.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/contracts/OTERYN_V2_WORLD_TOPOLOGY_CONTRACT.md
search_first:
  - services/game-gateway/internal/gateway types and service
  - app/GameAuth Tickets, OAuth, Worlds and NativeEvidence
optional_reads: []
---

# OTERYN-20260929-n4p-native-gateway-contract

## Goal

Governing GitHub Issue: #1419 (https://github.com/Oteryn/Oteryn-Platform/issues/1419) — canonical lifecycle authority for this task. Coordination: Oteryn/Oteryn-Game#162; owner authority Q14 (comment 5899892092) and Q15a–Q18a (comment 5899942821).

Delivery item 1 of #1419: the N4-P contract candidate `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md` (native `/v1/login`, native ticket redemption to canonical AccountId, Character selection over the Game-owned projection, route selection over NativeTopologyRegistry and reported runtime status, FND-04 grant issuance and signing-key custody, `attempt_ref` idempotency, error mapping, rate limits, Canary-only list, UNKNOWN register). Documentation only. Acceptance is by the Game architect and the owner.

## Acceptance criteria

- [x] Candidate contract covers every element of #1419 delivery item 1 and marks each unknown with an identifier.
- [x] No accepted Platform contract semantics change; refinements and decisions are labelled.
- [ ] Game architect and owner accept the candidate (Q15a).
- [ ] The Oteryn-Game lock entry references this file at the frozen head of this PR.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md
modules:
  - GameAuth
  - game-gateway
dependencies:
  - Oteryn-Game FND-04_PRE_ADMISSION_GRANT_PROFILE_V1 (read-only)
  - Oteryn-Game ADR-0020 section 7 N4-P (read-only)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game OTV2-20260929-n4p-contract-game (lock entry and Game-side candidates)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-30T00:30:00Z
head: 050df7491852d40e9d19f23dad38fc5d6b1b4268
branch: claude/n4p-native-gateway-contract
pr: 1420
status: validating
terminal_pr_policy: archive_pending
context_routes:
  - architecture
  - auth-identity
  - api
owned_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md
proven:
  - Gateway /v1/login today redeems Canary tickets and returns integer canary ids (services/game-gateway)
  - Ticket issuance requires a ready IdentityCanaryAccount binding (IssueGameLoginTicket)
  - identities.account_id is a canonical UUIDv7 and native_security_generation exists (native evidence contract)
  - NativeTopologyRegistry issues UUIDv7 WorldId and ChannelId only in testing or preproduction
  - NativeSigningTrustRegistry stores public keys only for the fixed fresh-entry scope
derived:
  - One Laravel issuer transaction can make redemption, selection and signing atomic
  - Deterministic Ed25519 re-signing makes attempt_ref retries byte-identical without storing tokens
unknown:
  - U1 to U18 as listed in section 15 of the candidate contract
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Node-reported gameplay endpoint and route_revision rejected in review round 1; the World Registry owns the route record
  - Two-phase Gateway selection handle rejected as recommendation because it adds a second bearer credential
changed_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md
validation:
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260929-n4p-native-gateway-contract.md --require-checkpoint
    result: PASS
    evidence: local run before push
  - command: git diff --check
    result: PASS
    evidence: local run before push
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only contract candidate; no runtime path changes
blockers:
  - none
next_action: await architect and owner acceptance of the frozen candidate; after this PR integrates, archive this packet to docs/agents/tasks/archive in the next #1419 delivery task
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository PR path
source_branch_evidence: pending
```

## Notes

Documentation only: no code, migration, route, configuration, secret or workflow change. The Game-side lock entry pins this file by the frozen head of this PR as pending evidence; the canonical commit is set only after merge.
