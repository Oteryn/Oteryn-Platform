# OTERYN-20260811 official Linux offline validation

## Scope

This task is an owner-authorized `official-offline-launch` validation only. It does not authorize account data, login, world entry, gameplay-service contact, `official-live`, client/BattlEye modification, anti-cheat bypass, hooking/injection/ptrace/debugging, or traffic interception/decryption/replay/injection.

## PROVEN

- Issue #987 is the bounded owner authorization for this phase.
- Previous synthetic/no-network harness task `OTERYN-20260810-tibia-linux-reference-harness` is completed and archived on `main`.
- CipSoft's Linux-client support documentation names the x86-64 archive `tibia.x64.tar.gz` and requires launching from the extracted Tibia directory.
- CipSoft states that the Linux client is unsupported and 64-bit only.
- The Tibia Service Agreement incorporates the BattlEye End-User Licence Agreement.
- Package acquisition for this task is pinned to the public CipSoft host `https://static.tibia.com/download/tibia.x64.tar.gz` and is permitted only for identity establishment.
- `.github/workflows/tibia-linux-official-identity.yml` is acquisition/identity-only: it hashes and inspects the archive without executing an ELF and does not upload the proprietary archive as an artifact.
- `official_host_preflight.py` rejects CI, container and WSL execution before invoking the existing official-mode storage/network preflight.

## DERIVED

- A GitHub-hosted runner is suitable for public package acquisition and non-executing identity inspection, but it cannot satisfy the dedicated normal Linux host acceptance criterion.
- The execution host should be a separate normal Ubuntu desktop/VM with a dedicated non-root account, an actual graphics device, an interactive display session and a LUKS-backed private evidence mount outside Git.
- A separate VM is preferable to reusing shared Synology/runner/container state because execution evidence and cleanup must be ownership-scoped.

## UNKNOWN

- Exact current archive SHA-256, current ELF identities and Build IDs until the acquisition workflow completes.
- Whether the current public archive contains the final game client ELF or only launcher/bootstrap components.
- Exact current client version until package/update metadata is acquired without executing the client.
- Availability of a remotely controllable dedicated normal Linux desktop/VM from the current tool session.
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
evidence_storage:
  location: outside Git
  encryption: LUKS/dm-crypt or another preflight-proven encryption type
  permissions: owner-only
network_execution:
  default: denied before client process creation
  namespace: distinct
  visible_interfaces:
    - lo
credentials: none
official_service_contact_during_execution: forbidden
```

## Gate order

1. Public acquisition and non-executing package identity.
2. Dedicated-host boundary proof.
3. Encrypted evidence-volume proof.
4. Exact identity approval file outside Git.
5. Offline client/component launch with networking denied before process creation.
6. GUI/BattlEye/network/leak/cleanup evidence review.
7. Close task as PASS or BLOCKED. `official-live` always remains a separate explicit task.
