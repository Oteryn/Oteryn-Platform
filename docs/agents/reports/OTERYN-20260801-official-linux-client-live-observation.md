# Official Tibia Linux client local-harness observation

## Status and scope

Phase: `local-harness-readiness`.

No official-service authentication, world entry, gameplay, official traffic collection or external-repository mutation occurred in the proven phase.

This report predates the final Oteryn-v2 authority consolidation. Its synthetic harness evidence remains valid as bounded local research evidence, but its interpretation is now governed by ADR 0031:

- Oteryn Platform owns Identity/Gateway/control-plane concerns;
- Oteryn-v2 is the canonical native Rust client/game/protocol implementation authority;
- historical `blakinio/otclient`, Canary and official Tibia behavior are reference/compatibility evidence only.

Nothing in this report proves final Oteryn-v2 protocol or runtime conformance.

## Environment discovery

### `PROVEN`

- The historical implementation host was Windows 11 Pro x86-64 with WSL2/WSLg Ubuntu 26.04.
- WSLg exposed graphical session support and GPU integration required for the fake-client component test.
- An unprivileged user/network namespace was available for the no-network fake-client run.
- The guest filesystem was available, but the process could not prove host-volume encryption.
- The owner-approved official package/executable identity was absent; no official binary or proprietary asset was copied into Git.

### `DERIVED`

- WSL2/WSLg was sufficient for deterministic synthetic graphical lifecycle, redaction and cleanup validation.
- WSL2 evidence does not establish that the official client or BattlEye accepts virtualization.
- A refusal from the unmodified official client/BattlEye would require moving to an accepted dedicated interactive Linux host, not bypassing the refusal.

### `UNKNOWN`

- Whether the future research host/evidence volume satisfies the required encryption boundary.
- The exact approved official package SHA-256 and private package path.
- Whether the exact official client and BattlEye start unmodified in the eventual approved environment.

### `CONFLICT`

- None in the preserved synthetic evidence.

## Preserved harness implementation

The historical PR #391 harness under `tools/tibia-linux-reference/` provides:

- fail-closed Linux/display/environment/tracing/injection/storage/network preflight;
- exact package/executable SHA-256 and ELF Build ID verification without execution;
- a deterministic graphical fake client launched inside a no-network namespace;
- synthetic login/password/token/authenticator transfer through an anonymous pipe;
- bounded process/window/network-denial/timing/temporary-filesystem evidence;
- a redacted manifest schema/example, exact-value leak scanner and deterministic cleanup verifier;
- an exact-client component mode that forbids authentication and outbound networking;
- focused unit tests and a Linux CI workflow.

Issue #886 intentionally does not modify this harness or workflow. The retained branch and Git history preserve it even though PR #391 is not suitable for direct merge from its historical base.

## Validation findings

### Focused validation — historical branch

The preserved evidence records:

- checkpoint validation: `PASS`;
- Python compilation: `PASS`;
- focused unittest suite: `PASS` (11 tests at the recorded phase, later branch history may contain additional tests);
- synthetic manifest schema validation: `PASS`;
- workflow YAML parsing: `PASS`;
- high-confidence tracked-file secret/token scan and `git diff --check`: `PASS`.

### Component validation — historical branch

- WSL2/WSLg graphical fake-client no-network run: `PASS`.
- The fake client used a distinct network namespace with only loopback; a reserved TEST-NET connection was denied and no official endpoint was contacted.
- X11/process lifecycle completed normally.
- Exact synthetic secret corpus was absent from Git-visible/retained outputs after leak scanning.
- Invalid-display failure-path probe failed closed and left no protected temporary process/file state.
- Official-mode preflight failed closed because encrypted storage was not proven.
- Identity verification with the intentionally incomplete template failed closed; no official client execution occurred.
- A generic-token false positive against unchanged synthetic fixtures was isolated; the repair preserved exact-value scanning and bounded generic scanning rather than weakening leak protection.

### Heavy/live validation

`NOT_RUN / BLOCKED`

The historical task recorded the synthetic exact code head `cabad487a139aaf0983dfc55cfb18d9f43720633` as passing its focused and graphical no-network component gate. PR #391 later advanced to `630ed73c09242cf3d37f3652b06fa252c6b0f10d`.

The official-client/BattlEye component launch and official-service login were not performed because the private package identity and encrypted evidence boundary were not proven together. That remains the correct fail-closed decision.

## Current architecture interpretation

### `PROVEN`

- The local synthetic harness validates only its own launch/redaction/isolation/cleanup behavior.
- No official live session occurred, so there is no live login/world-state compatibility evidence to hand off.
- ADR 0031 now separates native Oteryn-v2 integration from Legacy Canary Compatibility.
- Oteryn-v2 is the current native client/game/protocol implementation authority; historical OTClient/Canary are not the native target.

### `DERIVED`

- The harness can be reused in a future fresh current-main research task after revalidation without carrying forward the old target-repository assumptions.
- Any future official observation should be converted into bounded behavioral questions/requirements and routed to the current owner rather than copied as an implementation design.

### `UNKNOWN`

- Whether the owner will resume this research programme.
- Whether a future accepted Linux environment will launch the unmodified official client/BattlEye.
- What specific native-v2 interoperability gap, if any, would justify a live official observation.

### `CONFLICT`

- The historical PR/task wording that routed native Rust follow-up to `blakinio/otclient` conflicts with the accepted Oteryn-v2 authority. Issue #886 repairs that wording and PR lifecycle; it does not invalidate the synthetic harness results.

## Current phase decision

`external_service_execution_ready = false`

No official login is attempted as part of Issue #886. The blockers remain:

- encrypted private evidence storage not proven;
- exact approved official package identity unavailable;
- no successful no-authentication official component gate;
- no current owner-gated live observation request tied to a specific native-v2 evidence gap.

## Current handoff rule

If the research is resumed later:

- Platform Identity/login/ticket/Gateway findings stay in `blakinio/Oteryn-Platform`;
- native Rust client, native game server and `protocol-oteryn` findings go to a separately authorized `blakinio/Oteryn-v2` task;
- historical OTClient/Canary/Tibia observations remain reference/compatibility evidence;
- no external repository is mutated by this Platform research task.
