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
- [ ] Owner and Game architect accept or amend ADR 0045 and §19. U19–U25 are answered.

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
updated_at: 2026-10-12T12:00:00Z
head: UNKNOWN
branch: claude/gateway-reconnect-channels-design-20261012
pr: none
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
derived:
  - the recovery grant needs a Platform credential that survives ticket issuance; the design proposes a hashed play-session recovery handle
unknown:
  - U19 to U25 in contract section 19.12
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
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only design; no code, route or configuration changed
blockers:
  - owner and Game architect acceptance of ADR 0045 and contract section 19
next_action: owner and Game architect review ADR 0045 and contract section 19 and answer U19 to U25
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Jira synchronization: pending (no mapped Story was resolved in this session). After acceptance, each implementation step of contract §19.11 needs its own packet.
