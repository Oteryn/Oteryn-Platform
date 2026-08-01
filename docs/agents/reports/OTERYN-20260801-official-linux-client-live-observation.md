# Official Tibia Linux client local-harness observation

## Scope

Phase: `local-harness-readiness`. No official-service authentication, world entry, gameplay or raw
traffic collection is part of this report.

## Environment discovery

### `PROVEN`

- The implementation host is Windows 11 Pro build 26200 on x86-64 with an AMD Ryzen 7 9800X3D,
  64 GB RAM, integrated AMD graphics and an AMD Radeon RX 9070 XT.
- The available Linux boundary is Ubuntu 26.04 x86-64 on WSL2 kernel
  `6.18.33.2-microsoft-standard-WSL2` under a non-root user.
- WSLg exposes X11 (`DISPLAY=:0`), Wayland and `/dev/dxg` GPU integration.
- `unshare -Urn true` succeeds for the non-root user, proving an unprivileged user/network
  namespace control is available for the fake-client dry run.
- The guest root filesystem is ext4 on a WSL virtual block device. The current process cannot query
  host BitLocker state.
- The populated official package and executable are absent from this checkout. No official binary
  or proprietary asset was copied into Git.

### `DERIVED`

- WSL2/WSLg is sufficient to exercise the local fake-client, graphical lifecycle, leak scanning and
  cleanup controls.
- WSL2 cannot establish that BattlEye permits the virtualized environment; a refusal must not be
  worked around, and a normal dedicated interactive Linux host is the fallback.

### `UNKNOWN`

- Host-volume encryption cannot be proven from the current unprivileged process.
- The approved official package SHA-256 and private package path are unavailable.
- Whether the exact official client and BattlEye start without modification in WSL2 remains unknown.

### `CONFLICT`

- None.

## Harness implementation

The harness under `tools/tibia-linux-reference/` provides:

- fail-closed Linux, display, environment, tracing, injection, storage and network preflight;
- exact package SHA-256, executable SHA-256 and ELF Build ID verification without execution;
- a deterministic graphical fake client launched inside a no-network namespace;
- synthetic login/password/token/authenticator transfer through an anonymous pipe;
- bounded process, window, network-denial, timing and temporary-filesystem evidence;
- a redacted manifest schema/example, exact-value leak scanner and deterministic cleanup verifier;
- an exact-client component command that forbids authentication and outbound networking;
- focused unit tests and a Linux CI workflow.

The retained manifest intentionally contains no raw stdout/stderr, environment values, process
arguments, local private paths, screenshots or captures.

## Validation findings

### Focused validation

- `python tools/agents/checkpoint.py ... --require-checkpoint`: `PASS`.
- `python3 -m compileall -q tools/tibia-linux-reference`: `PASS`.
- `PYTHONPATH=tools/tibia-linux-reference python3 -m unittest discover -s
  tools/tibia-linux-reference/tests -v`: `PASS`, 11 tests.
- `python3 tools/tibia-linux-reference/run.py validate-manifest
  tools/tibia-linux-reference/examples/session-manifest.synthetic.json`: `PASS`.
- PyYAML parse of `.github/workflows/tibia-linux-live-reference.yml`: `PASS`.
- High-confidence tracked-file token/key scan and `git diff --check`: `PASS`.

### Component validation

- WSL2/WSLg graphical fake-client no-network run: `PASS`, retained only mode `0600` redacted
  `session-manifest.json` and `cleanup-report.json` outside Git.
- The fake client ran in a distinct network namespace with only `lo`; its reserved `TEST-NET-2:443`
  connection was denied and no official endpoint was contacted.
- The X11 window lifecycle was `mapped` then `destroyed`; the process lifecycle was `started` then
  `exiting` with status `0`.
- The exact synthetic corpus was absent from Git diff, tracked files, process arguments, retained
  environment report, stdout/stderr, evidence/artifacts, temporary files, shell history and cleanup
  report. The scanner inspected 1,308 files in the observed run.
- Failure-path probe with an invalid X display: expected `HarnessError`; no process or protected
  temporary file remained.
- Official-mode preflight on the current evidence path: expected fail-closed `HarnessError` because
  encryption is not proven.
- Identity verification with the intentionally incomplete template: expected fail-closed
  `HarnessError`; no client execution occurred.
- The first post-commit component attempt failed because generic token scanning included three
  unchanged repository fixtures. A filename-only diagnostic proved no run value leaked. The repair
  keeps exact-value scans across every tracked file and applies generic token detection to the full
  branch diff and new retained outputs; a focused regression test passes.

### Heavy/exact-head validation

The restacked exact code head `cabad487a139aaf0983dfc55cfb18d9f43720633` passed the 12-test
focused suite and the WSL2/WSLg graphical synthetic no-network component gate. Repository PR checks
remain to be observed after push. The official-client/BattlEye component launch is blocked locally
by unavailable private package identity and unproven encrypted storage; it is not reported as
successful.

## Phase decision

`external_service_execution_ready` remains `false`. The synthetic harness is locally proven, but
the exact official identity, encrypted-volume and no-authentication official component gates remain
blocked. No official login was attempted in this phase.
