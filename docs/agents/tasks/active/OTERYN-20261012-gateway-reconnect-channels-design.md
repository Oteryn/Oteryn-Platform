---
task_id: OTERYN-20261012-gateway-reconnect-channels-design
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
search_first: []
optional_reads:
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
---

# OTERYN-20261012 Gateway recovery and multichannel design

## Goal

Governing GitHub Issue: #1419 (N4-P native Game Gateway) is the canonical lifecycle authority for this task.

This is a design-only packet that changes no code. It covers two extensions of the native Gateway login:

1. a recovery grant path: a client that lost its game connection gets a short-lived `oteryn-reauth-recovery-v1` grant without a browser sign-in, fenced by the account's security and play generations;
2. multiple channels per World: per-channel route records, channel selection in the login response, a channel-specific refusal, and separate key purposes for fresh admission and existing-actor recovery.

## Acceptance criteria

- [x] Candidate contract amendment §19 (protocol 3) in `OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`. Protocol 2 is unchanged.
- [x] Proposed ADR 0045, registered in the ADR README inventory.
- [x] Game dependencies listed (G1 recovery locator, G2 profile clarification, client protocol 3). No Game file is written.
- [x] Owner decisions of 2026-10-12 recorded:
  - U19: channels `antica`, `obscura` and `pythera`, label equal to the capitalised key, `oteryn-staging-1` for staging only;
  - U20: H = 12 h;
  - U21: the handle counts as fresh authentication, and device binding is follow-up H1 after a repository check;
  - U22: G1 is built now, with its ready-to-dispatch Game packet in contract §19.14.
- [ ] The Game architect accepts or amends ADR 0045, §19, G1 and G2.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20261012-gateway-reconnect-channels-design.md
  - docs/agents/tasks/archive/OTERYN-20261012-gateway-reconnect-channels-design.md
modules:
  - game-auth
dependencies:
  - Game FND-04 recovery profile and FND-04B (read-only)
blockers:
  - owner and Game architect acceptance
cross_repository_tasks:
  - Game G1 recovery locator and G2 recovery profile clarification (listed, not written)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T18:00:00Z
head: UNKNOWN
branch: claude/gateway-reconnect-channels-design-20261012
pr: 1558
status: waiting
context_routes:
  - game-auth native gateway contract
owned_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20261012-gateway-reconnect-channels-design.md
proven:
  - Platform already produces ReadRecoverySigningTrustV2 and ReadRecoveryAccountSecurityV2 with key purpose existing_actor_recovery
  - Game serves ClientResume with an oteryn-reauth-recovery-v1 grant on the Channel owner (FND-04B section 20)
  - the recovery profile forbids channel, node and runtime-owner claims
  - ADR 0044 fences the preproduction store to one route record
  - the remembered-device family exists only as a disabled, opt-in candidate (NATIVE_REMEMBERED_DEVICE_SESSION_CONTRACT_CANDIDATE.md); its credential rotates on use and its tickets do not record the family
  - the Registry channel_key grammar already admits antica, obscura and pythera
derived:
  - the recovery grant needs a Platform credential that survives ticket issuance; the design proposes a hashed play-session recovery handle
unknown:
  - U23 to U25 in contract section 19.12
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - recovery with a fresh-entry grant
  - channel claim inside the recovery grant
changed_paths:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0045-native-gateway-recovery-and-multichannel-login.md
  - docs/architecture/adr/README.md
  - docs/agents/tasks/active/OTERYN-20261012-gateway-reconnect-channels-design.md
validation:
  - command: python3 tools/validation/adr_registry.py
    result: PASS
    evidence: registry validation passed
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group rose
  - command: python3 tools/agents/checkpoint.py <this record> --require-checkpoint
    result: PASS
    evidence: checkpoint validated
  - command: python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: documentation IA validated
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only design; no code, route or configuration changed
blockers:
  - Game architect acceptance of ADR 0045, contract section 19, G1 and G2
next_action: Game architect reviews ADR 0045 and contract section 19; the Game lead dispatches the section 19.14 packet
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Owner decisions U19–U22 (2026-10-12) are recorded in contract §19 and ADR 0045. The owner wants a dedicated Issue for this work; it is proposed in the PR report and not created here. Game packet `OTV2-20261012-recovery-locator-producer` (contract §19.14) is ready to dispatch in Oteryn-Game. It is listed, and no Game file is written. Jira synchronization: pending (no mapped Story was resolved in this session). After acceptance, each implementation step of contract §19.11 needs its own packet.
