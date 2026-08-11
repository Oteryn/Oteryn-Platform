# OTERYN-20260810 Tibia Linux reference harness — observation

## Scope

Current phase: `local-synthetic-readiness`.

No official Tibia login, official-service contact, official-client execution, BattlEye execution,
real credential use, packet capture, traffic alteration, deployment or cross-repository write is
authorized or claimed in this report.

## Current evidence

### PROVEN

- Historical PR #391 remains closed and superseded; PR #961 is the bounded continuation.
- Current `main` used by the successful PR merge ref is `b54de1859cfdb3ca12ff8904e0b0ead82449f613`.
- Runtime implementation head `cd4ab6c68e2cd20e19f80ddbb9a4c14535223f3d` was validated by `Tibia Linux Reference Harness` run `31469809872`, job `93710431162`, conclusion `success`.
- That exact run passed `compileall`, all 29 focused unit tests, the schema/manifest check, the graphical no-network component, and the final no-raw-artifact/clean-worktree check.
- The synthetic retained summary reported Ubuntu 24.04.4 LTS, Linux x86-64, X11 present, shell tracing disabled, `LD_PRELOAD`/`LD_AUDIT` absent, ordinary environment secret scan `PASS`, and the privileged namespace setup with UID/GID drop.
- The network-denial summary reported only interface `lo`, a distinct network namespace, blocked IPv4/IPv6/`.invalid` classifications, `proven=true`, `official_endpoint_contacted=false`, and `raw_capture_created=false`.
- Retained synthetic evidence reported `leak_scan=PASS` and `cleanup=PASS`.
- The runner evidence filesystem was ordinary ext4 on a partition and correctly reported `encryption_proven=false`; this is acceptable for synthetic mode and demonstrates that future official mode will fail closed rather than treating an arbitrary block device as encrypted.
- `actions/checkout` used `persist-credentials: false`; checkout removed its temporary Git credential configuration before the harness/test Python steps ran.
- The successful repair declares pointer-safe `ctypes` signatures for every used X11 function that accepts a `Display*`; two dedicated ABI regression tests passed in the successful run.

### DERIVED

- The prior `fake-client-unknown` failure is consistent with native pointer truncation in untyped `ctypes` X11 calls: the failing run took long enough to execute the bounded network probes and emitted no Python failure event, while the pointer-safe ABI repair changed that component from FAIL to PASS without weakening any network or secret control.
- The current synthetic harness is suitable as a reusable local/offline research foundation. This does not imply that an official Tibia client or BattlEye will accept any future host.

### UNKNOWN

- Whether a future separately owner-gated exact official client and BattlEye can run on a dedicated normal Linux host.
- Whether a future official-component host provides a proven encrypted private evidence volume and a separately owner-approved exact package identity.

### CONFLICT

None.

## Implemented controls

- per-run synthetic login/password/token/authenticator corpus;
- anonymous-pipe transfer through process stdin, never arguments or ordinary environment variables;
- fail-closed secret-like environment and injection checks;
- filesystem X11 transport (`DISPLAY=unix/:99`) with Xvfb TCP disabled;
- explicit X11 ABI signatures for 64-bit pointers;
- distinct OS network namespace with only loopback and bounded IPv4/IPv6/DNS denial probes;
- fail-closed manifest contract for network, credential and safety fields;
- exact/high-confidence leak scans across enumerated Git/process/output/evidence/temp/history surfaces;
- deterministic profile and Xvfb diagnostic cleanup;
- placeholder-only identity template;
- future official-mode storage check that requires real crypt evidence rather than `/dev/mapper` naming alone;
- synthetic-only GitHub Actions with no official package download, official component invocation, or artifact upload.

## Current security disposition

`external_service_validation_authorized=false`

`external_service_execution_ready=false`

Only the deterministic fake client is permitted in this task. Any official-client component or
live-service phase requires a new, explicit owner-gated task and must not inherit authorization from
this synthetic closeout.
