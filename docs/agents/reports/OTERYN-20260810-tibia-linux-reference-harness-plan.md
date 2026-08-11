# OTERYN-20260810 Tibia Linux reference harness — implementation plan

## Purpose

Resume the reusable synthetic/no-network capability from historical PR #391 on current
`blakinio/Oteryn-Platform` authority without reopening or merging the superseded branch.

The current phase is `local-synthetic-readiness`. `external_service_validation_authorized=false`
and `external_service_execution_ready=false`.

## Verified starting state

### PROVEN

- Fresh task branch: `feat/OTERYN-20260810-tibia-linux-reference-harness`.
- Base at task creation: `dc9adc7d9246e83c7299d8cf9c161524fb85b2c9` (`main`).
- Draft PR: #961.
- Historical PR #391 is closed, unmerged and explicitly superseded.
- Historical PR #391 preserves a synthetic Linux harness whose focused syntax/unit checks passed,
  while its graphical no-network component failed.
- Current authority routes native Rust client/game/protocol implementation to
  `blakinio/Oteryn-v2`; this Platform task performs no cross-repository mutation.

### DERIVED

The safest continuation is to graft only the historical harness subtree onto the fresh branch,
retain fail-closed security checks, and fix CI isolation rather than suppressing secret detection.

### UNKNOWN

- Whether the historical component failure was caused solely by inherited CI environment material,
  X11 transport across the network namespace, or both. Current-head CI is required to prove the
  repair.
- Whether any future normal interactive Linux host will run the unmodified official client and
  BattlEye.
- Whether a future host will provide the required provably encrypted private evidence volume and
  owner-approved exact package identity.

### CONFLICT

None.

## Threat model

The synthetic gate demonstrates absence of each generated synthetic secret from enumerated
Git-visible, process-visible and retained runtime surfaces under the tested Linux execution model.

The claim does not cover arbitrary compromise of the kernel, hypervisor, administrator account,
hardware, firmware or an out-of-scope privileged observer.

## Implementation

1. Selectively restore `tools/tibia-linux-reference/**` from historical PR #391.
2. Keep preflight fail-closed for secret-like ordinary environment variables and injection
   variables.
3. Generate the synthetic login/password/token/authenticator corpus per run and transfer it only via
   an anonymous pipe.
4. Strengthen the fake network-denial proof so success requires:
   - a distinct Linux network namespace;
   - only `lo` visible;
   - IPv4 documentation-address connection denied;
   - IPv6 documentation-prefix connection denied;
   - `.invalid` hostname resolution denied.
5. Run the CI harness process with `env -i` plus an explicit non-sensitive allowlist instead of
   weakening secret scanning for GitHub Actions.
6. Force X11 to the filesystem transport with `DISPLAY=unix/:99`, keep Xvfb TCP disabled and wait
   for `/tmp/.X11-unix/X99` before launch.
7. Disable core dumps and use `umask 077` in CI.
8. Keep workflow validation synthetic-only; never download/launch an official package and never
   upload runtime evidence as a GitHub artifact.
9. Preserve exact identity verification and encrypted-storage preflight as dormant, separately
   owner-gated future-component foundations.
10. Document exact current-head validation and leave `external_service_execution_ready=false`.

## Validation ladder

### Focused

- `python -m compileall -q tools/tibia-linux-reference`
- `PYTHONPATH=tools/tibia-linux-reference python -m unittest discover -s tools/tibia-linux-reference/tests -v`
- synthetic manifest schema validation
- workflow invariants: `env -i`, filesystem X11 transport, no artifact upload, no official component invocation

### Component

GitHub-hosted synthetic graphical dry run:

- Xvfb with TCP disabled;
- filesystem X11 socket;
- generated synthetic corpus;
- anonymous-pipe transport;
- distinct no-network namespace;
- aggregate IPv4/IPv6/DNS denial;
- bounded window/process lifecycle;
- exact/prohibited-location leak scan;
- deterministic cleanup.

### Heavy

- dedicated `Tibia Linux Reference Harness` workflow on exact PR #961 head;
- exact-head changed-path review;
- exact-head required repository checks relevant to this task.

The heavy gate is attempted at most twice in this execution session. No official login is part of
this task.
