---
task_id: OTERYN-20260811-official-linux-offline-launch
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/EXECUTION_RESOURCE_HYGIENE.md
  - docs/agents/tasks/archive/OTERYN-20260810-tibia-linux-reference-harness.md
search_first:
  - official-offline-launch
  - tibia-linux-reference
  - BattlEye
---

# OTERYN-20260811 official Linux offline launch

## Goal

Validate the exact unmodified official Tibia Linux client on a dedicated normal Linux host under fail-closed outbound network denial, with no account data, no login and no official-service gameplay contact. Issue: #987.

## Authorization boundary

```yaml
external_service_validation_authorized: true
official_package_acquisition_authorized: true
official_offline_execution_authorized: true
official_service_contact_authorized: false
real_credentials_authorized: false
official_live_authorized: false
client_or_battleye_modification_authorized: false
anti_cheat_bypass_authorized: false
```

Acquisition traffic to the official public CipSoft download/update surfaces is permitted only to establish the exact package/client identity. Official client execution is permitted only after the dedicated-host, encrypted-storage and exact-identity preflight gates pass. Any login, world entry, account data, gameplay-service connection, client/BattlEye modification, hooking, injection, ptrace/debugging, traffic interception/decryption/replay/injection or bypass remains forbidden.

## Acceptance criteria

- [ ] Dedicated normal Linux desktop/VM host is proven; CI runners, containers and WSL are rejected for official execution.
- [ ] Private evidence storage outside Git is proven encrypted, owner-only and fail-closed.
- [ ] Exact current official source URL, acquisition time, package SHA-256, launcher/client version evidence, executable SHA-256 and ELF Build ID are recorded without committing proprietary binaries.
- [ ] Unmodified official client starts with networking denied before process creation and receives no account/session material.
- [ ] GUI lifecycle, network namespace/isolation, BattlEye-visible behavior, leak scan and deterministic cleanup PASS.
- [ ] No official endpoint is contacted during the offline execution phase.
- [ ] Raw evidence remains private outside Git; only bounded redacted metadata is retained in the repository.
- [ ] `official_live_authorized=false` remains authoritative at closeout; live work requires a separate task.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260811-official-linux-offline-launch.md
  - docs/agents/reports/OTERYN-20260811-official-linux-offline-launch.md
  - .github/workflows/tibia-linux-official-identity.yml
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
modules:
  - research-tooling
  - ci
dependencies:
  - OTERYN-20260810-tibia-linux-reference-harness
  - issue-987
blockers:
  - dedicated normal Linux host access is not yet proven
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T09:05:00Z
head: fb473b5030e20886692e0833ab944f0717ab3ab7
branch: research/OTERYN-20260811-official-linux-offline-launch
pr: none
status: implementing
context_routes:
  - agent-governance
  - security
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260811-official-linux-offline-launch.md
  - docs/agents/reports/OTERYN-20260811-official-linux-offline-launch.md
  - .github/workflows/tibia-linux-official-identity.yml
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
proven:
  - Owner explicitly authorized the bounded official-offline-launch sequence in chat on 2026-08-11.
  - Issue 987 records the authorization and safety boundary.
  - Previous synthetic harness task is completed and archived on main.
  - CipSoft documents the Linux x64 tarball name tibia.x64.tar.gz; current public acquisition target is the official static.tibia.com download surface.
derived:
  - Package acquisition/identity can be proven on an ephemeral networked runner without authorizing official client execution there.
  - Official execution must reject CI/container/WSL and wait for a dedicated normal desktop/VM Linux host with encrypted private storage.
unknown:
  - Exact current package SHA-256 and executable identities.
  - Whether the downloaded archive contains the final game binary or only launcher/bootstrap components.
  - Availability and identity of a suitable dedicated normal Linux execution host accessible to this task.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-official-linux-offline-launch.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: task initialized; acquisition and host preflight tooling not yet committed
blockers:
  - Dedicated normal Linux host access is not yet proven; GitHub-hosted runners are acquisition-only and cannot satisfy official execution acceptance.
next_action: Implement bounded official artifact identity acquisition/probe and dedicated-host fail-closed preflight, then run acquisition CI.
```
