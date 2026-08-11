# OTERYN-20260811 official Linux offline validation

## Scope

This task is an owner-authorized `official-offline-launch` validation only. It does not authorize account data, login, world entry, gameplay-service contact, `official-live`, client/BattlEye modification, anti-cheat bypass, hooking/injection/ptrace/debugging, or traffic interception/decryption/replay/injection.

## PROVEN

- Issue #987 is the bounded owner authorization for this phase; draft PR #988 owns the implementation.
- Previous synthetic/no-network harness task `OTERYN-20260810-tibia-linux-reference-harness` is completed and archived on `main`.
- CipSoft's current Linux support documentation names `tibia.x64.tar.gz`, says extraction creates a `Tibia` directory and requires launching the `Tibia` binary from that directory; Ubuntu-based distributions require `libxcb-cursor0`.
- CipSoft states that the Linux client is unsupported. Its Linux graphics support documentation expects a working accelerated graphics stack.
- The Tibia Service Agreement incorporates the BattlEye End-User Licence Agreement.
- Package identity tooling is pinned to the public CipSoft target `https://static.tibia.com/download/tibia.x64.tar.gz`; no mirror is an approved identity source.
- `official_identity_probe.py` hashes the archive and every ELF member, extracts ELF Build IDs and bounded version-like tokens, rejects non-approved source/path traversal, and never executes an ELF.
- `official_host_preflight.py` rejects CI, containers and WSL before invoking the existing official-mode storage/network/display preflight. After that safe preflight it additionally requires `glxinfo -B` direct rendering and rejects `llvmpipe`, `softpipe`, `swrast` and other software-only renderers.
- `official_host_prepare.sh` is Ubuntu/systemd-only, creates a dedicated non-admin user and installs bounded host/client dependencies including `libxcb-cursor0`, `mesa-utils`, cryptsetup and identity tools. It does not generate or retain any login credential.
- `official_evidence_luks_setup.sh` accepts only an explicitly confirmed blank second block device, rejects root/mounted/signature-bearing storage, requires an interactive TTY for the LUKS passphrase, cleans mount/mapper state on failure and leaves a session mount only after `lsblk TYPE=crypt` proves dm-crypt.
- `Tibia Linux Official Identity` run `31476777859`, job `93732191657`, passed all 5 then-current focused tests, Python compile checks and the CI execution rejection proof; `official_binary_executed=false`.
- That GitHub-hosted Azure run attempted the approved static target three bounded times and received HTTP 403 each time. No source-evasion/header/cookie workaround was attempted.
- A second bounded acquisition probe used the existing project-owned Synology egress on self-hosted runner `oteryn-synology-staging`, run `31478574949`, job `93737998188`. It performed no checkout, received no environment secrets, did not call Docker, did not execute an official binary and received the same HTTP 403 from the approved static target. Its cleanup step passed.
- The temporary project-egress acquisition job was removed immediately after that evidence was collected so arbitrary PR execution is not left enabled on the privileged runner.
- No proprietary archive/binary was committed or uploaded as an artifact by either acquisition probe.
- Synology Virtual Machine Manager supports creating independent guest VMs and VNC access, but the current tool session exposes no safe VMM/SSH action with which to create or manage a new guest. The existing `oteryn-staging` runner is a Docker-based deployment runner with the Docker socket mounted and is intentionally not repurposed as the official execution host.

## DERIVED

- GitHub-hosted runners are suitable for validating the non-executing tooling and proving that CI is rejected as an execution host, but they are not a valid acquisition/execution environment for this task.
- Two independent automatic `curl` environments now return 403. Exact acquisition must therefore occur through CipSoft's normal interactive official browser download flow on the eventual dedicated host; HTTP-denial circumvention would weaken provenance and is deliberately rejected.
- The preferred execution host is a separate normal Ubuntu physical host or VM with a dedicated non-root account, actual accelerated graphics, an interactive display session and a LUKS-backed private evidence mount outside Git.
- Synology VMM is a possible host boundary only if the guest independently passes direct-rendering and official host preflight; merely being a VM or exposing a framebuffer is insufficient.

## UNKNOWN

- Exact current archive SHA-256, current ELF SHA-256 values and Build IDs.
- Whether the current public archive contains the final game client ELF or launcher/bootstrap components that need a separately bounded acquisition/update step.
- Exact current client/launcher version identity.
- Availability of a remotely controllable dedicated normal Linux graphical host from the current tool session.
- Whether a Synology VMM guest exposes sufficient accelerated graphics for the current Linux client.
- BattlEye behavior under the final dedicated-host no-network launch.

## CONFLICT

None.

## Dedicated-host target

```yaml
os_family: Ubuntu LTS x86_64
boundary: normal physical host or VM
forbidden_boundaries:
  - GitHub-hosted CI
  - container
  - WSL
user: oteryn-tibia-ref
privilege: non-root
graphical_session: required
gpu_device: required
direct_rendering: required
software_only_renderer: forbidden
evidence_storage:
  location: /srv/oteryn-tibia-reference/evidence
  relationship_to_git: outside checkout
  encryption: LUKS2/dm-crypt with lsblk TYPE=crypt proof
  permissions: owner-only
  persistent_automount: false
network_execution:
  default: denied before client process creation
  namespace: distinct
  visible_interfaces:
    - lo
credentials: none
official_service_contact_during_execution: forbidden
```

## Host preparation sequence

1. Create a fresh Ubuntu LTS x86-64 graphical host/VM; do not reuse Home Assistant, Synology system services, runner containers or another task's VM.
2. Attach one normal OS disk and one fresh blank evidence disk. The evidence disk must contain no prior filesystem/signature/data.
3. Run `tools/tibia-linux-reference/official_host_prepare.sh` as root. It installs the bounded dependencies and creates `oteryn-tibia-ref` without sudo/admin/docker/lxd membership.
4. Run `tools/tibia-linux-reference/official_evidence_luks_setup.sh oteryn-tibia-ref /dev/<blank-disk> /srv/oteryn-tibia-reference/evidence DESTROY:/dev/<blank-disk>` from an interactive TTY. `cryptsetup` requests the LUKS2 passphrase interactively; the passphrase must never enter Git/chat/logs.
5. Enter the dedicated graphical session as `oteryn-tibia-ref` and run `official_host_preflight.py`. Any CI/container/WSL, missing graphics/display, software-only rendering, unproven encrypted storage, secret-like environment or unavailable fail-closed network namespace is terminal for that host until the host itself is corrected.
6. While normal networking is still available, acquire `tibia.x64.tar.gz` only through CipSoft's normal interactive official browser download flow into the private evidence volume. Do not use a mirror and do not automate around the proven HTTP denial.
7. Run `official_identity_probe.py` against the downloaded archive. Only after exact package/binary identity is established and approved may the offline component gate be considered.
8. Disable outbound networking through the harness isolation before process creation and run the existing identity-verified `official-component` path with no credentials. Any BattlEye/environment refusal is evidence and must not be worked around.

## Gate order

1. Dedicated-host creation/access.
2. Encrypted evidence-volume proof.
3. Official interactive acquisition and non-executing package identity.
4. Exact identity approval file outside Git.
5. Offline client/component launch with networking denied before process creation.
6. GUI/BattlEye/network/leak/cleanup evidence review.
7. Close task as PASS or BLOCKED. `official-live` always remains a separate explicit task.

## Current disposition

`BLOCKED` at the physical/external-host and interactive-download boundary. Repository preparation and fail-closed tooling are implemented; actual host provisioning, exact current package identity, official binary execution and BattlEye observation have **not** been performed or claimed.
