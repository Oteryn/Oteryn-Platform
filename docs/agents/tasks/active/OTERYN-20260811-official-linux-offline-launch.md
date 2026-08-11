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

Validate the exact unmodified official Tibia Linux client on a dedicated normal Linux host under fail-closed outbound network denial, with no account data, no login and no official-service gameplay contact. Issue: #987. Draft PR: #988.

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
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
modules:
  - research-tooling
  - ci
dependencies:
  - OTERYN-20260810-tibia-linux-reference-harness
  - issue-987
blockers:
  - no VMM/SSH/cloud-host control is exposed to this execution session for provisioning/accessing a dedicated normal Linux graphical host
  - official static download endpoint returns HTTP 403 from both GitHub-hosted Azure and project-owned Synology egress when requested non-interactively; bypass/evasion is forbidden
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-11T09:41:00Z
head: d0921e09aca56806b1020e7fc0483f70930c6dd4
branch: research/OTERYN-20260811-official-linux-offline-launch
pr: 988
status: blocked
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
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
proven:
  - Owner explicitly authorized the bounded official-offline-launch sequence in chat on 2026-08-11.
  - Issue 987 records the authorization and safety boundary; draft PR 988 carries the implementation.
  - Previous synthetic harness task is completed and archived on main.
  - CipSoft documents the Linux x64 tarball name tibia.x64.tar.gz, extraction into a Tibia folder and direct execution of the Tibia binary; Ubuntu-based distributions require libxcb-cursor0.
  - CipSoft documents Linux graphics-driver expectations and the Service Agreement incorporates the BattlEye EULA.
  - Identity tooling hashes the archive and ELF members, extracts ELF Build IDs and bounded version tokens, rejects archive traversal/non-approved source and never executes an ELF.
  - Host preflight rejects CI runners, containers and WSL before official execution, reuses official-mode encryption/network/display checks and requires direct GL rendering with no software-only renderer.
  - Dedicated-host preparation script is Ubuntu/systemd-only, creates a non-admin task user, generates no credential and installs bounded runtime/identity dependencies.
  - LUKS evidence setup accepts only an explicitly confirmed blank block device, rejects mounted/root/signature-bearing storage, requires interactive TTY for the passphrase, cleans partial mount/mapper state on failure and proves TYPE=crypt after setup.
  - Tibia Linux Official Identity run 31476777859 job 93732191657 passed 5/5 then-current focused tests and proved the official host gate rejects CI before any binary execution.
  - The Azure runner received HTTP 403 from https://static.tibia.com/download/tibia.x64.tar.gz on three bounded attempts; cleanup and clean-worktree verification passed.
  - A second acquisition probe on project-owned Synology egress, run 31478574949 job 93737998188, performed no checkout or Docker access, received HTTP 403 from the same approved target and passed cleanup.
  - The temporary self-hosted acquisition job was removed immediately after evidence collection.
  - No proprietary archive or official binary was committed, uploaded as an artifact or executed by either acquisition probe.
derived:
  - Exact package acquisition must occur on the dedicated normal Linux host through CipSoft's normal interactive official browser download flow rather than attempting to evade the proven automated-download 403.
  - A host/VM is not acceptable merely because it is Linux; it must pass dedicated-user, interactive-display, direct-rendering, encryption and isolation preflight before official execution.
unknown:
  - Exact current package SHA-256 and executable identities.
  - Whether the current downloaded archive contains the final game binary or launcher/bootstrap components requiring a separately bounded acquisition/update step.
  - Exact current launcher/client version identity.
  - Availability and identity of a suitable dedicated normal Linux graphical host accessible to this task.
  - Whether a Synology VMM guest can pass direct-rendering requirements.
  - BattlEye behavior under the final dedicated-host no-network launch.
conflicts: []
first_failure:
  marker: official-source-http-403-for-noninteractive-download
  evidence: run 31476777859 job 93732191657 on GitHub-hosted Azure and run 31478574949 job 93737998188 on project-owned Synology egress both returned HTTP 403; both cleanup paths passed and no bypass was attempted
rejected_hypotheses:
  - GitHub-hosted runner can serve as the official execution host; the dedicated-host preflight intentionally rejects CI.
  - GitHub-hosted runner can reliably acquire the official package from static.tibia.com; run 31476777859 proves HTTP 403 from that environment.
  - Switching the same curl request to project-owned home/Synology egress fixes acquisition; run 31478574949 proves the same HTTP 403.
  - A mirror or header/referer/cookie spoof should be used to bypass CipSoft download policy; source integrity and non-evasion require the normal official browser flow instead.
  - Any Synology VM with a framebuffer is sufficient; current host preflight requires direct GL rendering and rejects software rasterizers.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260811-official-linux-offline-launch.md
  - docs/agents/reports/OTERYN-20260811-official-linux-offline-launch.md
  - .github/workflows/tibia-linux-official-identity.yml
  - tools/tibia-linux-reference/official_identity_probe.py
  - tools/tibia-linux-reference/official_host_preflight.py
  - tools/tibia-linux-reference/official_host_prepare.sh
  - tools/tibia-linux-reference/official_evidence_luks_setup.sh
  - tools/tibia-linux-reference/tests/test_official_offline.py
validation:
  - command: Tibia Linux Official Identity run 31476777859 job 93732191657 focused tooling + CI host rejection
    result: PASS
    evidence: 5/5 tests PASS; py_compile PASS; CI execution gate PASS; official_binary_executed=false
  - command: official package automated acquisition from static.tibia.com on GitHub-hosted Azure
    result: BLOCKED
    evidence: HTTP 403 on three bounded attempts; no bypass attempted; cleanup PASS
  - command: official package automated acquisition from static.tibia.com on project Synology egress
    result: BLOCKED
    evidence: run 31478574949 job 93737998188 HTTP 403; no checkout/Docker/official execution; cleanup PASS
  - command: dedicated normal Linux host + encrypted evidence preflight
    result: BLOCKED
    evidence: current session exposes no VMM/SSH/cloud host control; no host state is fabricated
blockers:
  - Dedicated normal Linux graphical host cannot be provisioned or accessed from the currently available tools.
  - Exact official package identity cannot be established until the official archive is acquired through CipSoft's normal interactive browser download flow on that host.
next_action: Provision or expose access to a dedicated normal Ubuntu graphical host, run official_host_prepare.sh, create a blank second disk and run official_evidence_luks_setup.sh, acquire tibia.x64.tar.gz via the normal official browser flow into the encrypted evidence volume, then run official_identity_probe.py before any official binary execution.
```
