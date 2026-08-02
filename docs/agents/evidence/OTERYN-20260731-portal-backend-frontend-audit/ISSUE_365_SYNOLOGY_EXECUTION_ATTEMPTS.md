# Issue #365 exact-frozen Synology execution attempts

## Status

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
environment_status: READY
observer_generation_status: PASS
observer_installation_status: PASS
matrix_launch_status: PASS
matrix_status: INVALID_TECHNICAL_FAILURE
root_cause_status: UNKNOWN
production_action: none
```

Run `30758971408` reached the matrix stage on the Synology runner. The generated validator passed `bash -n`, the frozen target bootstrapped, and the Laravel 13.20.0 observer installed and linted. The package is not valid product evidence because every completed clean sample failed before the browser flow at `spawnSync php ENOENT`, and the first corrupt-fixture setup then failed its own snapshot invariant.

## Terminal authorized run

| Control head | Workflow run | Job | Artifact | Result |
|---|---:|---:|---|---|
| `613db96cda9d3ef513a033aff4a09b5e588798e9` | `30758971408` | `91526007975` | `8837189083`, `sha256:03ced224c4e14b649f62a77e512821cffc5df679c425610da603137040f66fa0` | Invalid technical execution; no causal Issue #365 result. |

Verified artifact facts:

- Downloaded ZIP digest exactly matched GitHub's artifact digest: `03ced224c4e14b649f62a77e512821cffc5df679c425610da603137040f66fa0`.
- The artifact contains only `issue365-synology-partial-30758971408.tar.gz`; the full evidence tarball, checksum file and matrix summary were not produced.
- `LAST_STAGE` is `matrix`.
- Six clean samples were attempted: three immediate and three pre-scroll.
- All six failed with Playwright exit code `1`, no browser trace, zero server events and no causal chain.
- The common first failure was `spawnSync php ENOENT` from `scripts/acceptance/tests/helpers.mjs:93` while calling `php artisan` inside the Playwright execution environment.
- The first one-corrupt fixture was seeded and marked `integrity_failed`, but fixture validation rejected the snapshot because the row still reported `storage_exists: true` and `thumbnail_exists: false`.
- No valid clean/corrupt comparison completed.
- Artifact upload and isolated runtime cleanup succeeded.
- No product code, production configuration, deployment or external repository was changed.

## Earlier bounded repair attempts

| Cycle | Control head | Workflow run | Last reached stage | Artifact | Result |
|---:|---|---:|---|---|---|
| 1 | `5cf9fee49927bd0f887131fe7e5ea7cf678d369b` | `30752369856` | `observer-generation` | `8834980323`, `sha256:08b677baf46d2d4a52ef7fe18234c05804a6f6e655901fb9dad42929ecee8783` | Generated selector lost quoting around `"$RUN_ROOT/media-snapshot.php"`. |
| 2 | `cddb7578d89101e90fac1f9b8bdd85e4739d28c8` | `30752964863` | `observer-install` | `8835208891`, `sha256:4861e421e4c4575f3f22ff5461ee16070c79114639ddb8c8f736afd1010d190c` | The retired runbook expected adjacent `saveSession` and `return` lines, but Laravel 13.20.0 contains a blank line. |
| 3 | `2bd32af496894403e0dec84efeca21b0642dcecd` | `30753618275` | validator preparation | none | Parent-level repair searched the wrong generated layer and found zero matches. |

## Current invocation isolation cycles

| Cycle | Control head | Workflow run | Result |
|---:|---|---:|---|
| 1 | `e76f31cd9bf0dc7a5a8ffd73bda94bec6e1c9d9b` | `30756664833` | Diagnostic-only gate proved the parent validator contains zero `saveSession` matches; the pattern belongs to generated `runtime/02-observer-patch.sh`. Matrix intentionally did not start. |
| 2 | `ce9aac5865ee893150ac88e11123601362eaaf28` | `30756859088` | Cheap preparation gate rejected an invalid nested Python string with `IndentationError`; matrix skipped. |
| 3 | `7d8eed05826363baed47487ca71203caf1c993a9` | `30756908549` | Preparation passed, environment bootstrapped, the generated runtime observer was corrected and installed, then post-install verification failed with `START_SESSION: unbound variable` before sample creation. |
| 4 | `613db96cda9d3ef513a033aff4a09b5e588798e9` | `30758971408` | START_SESSION repair passed; matrix began but the Playwright container lacked a callable `php` binary and the corrupt-fixture invariant also failed. |

## Classification

```yaml
exact_frozen_package: INVALID
reason: technical harness failures prevented a valid correlated 12-sample matrix
issue_365_completion: BLOCKED_EXTERNAL_DECISION
session_serialization_remediation: NOT_PROVEN_REMEDIATED
causal_link_to_damaged_media: UNKNOWN
product_failure_inferred: false
```

The terminal run proves the environment, observer installation and matrix entry point. It does not prove or disprove the Issue #365 product hypothesis.

## Rejected conclusions

- `spawnSync php ENOENT` is a product defect.
- Six failed clean samples satisfy the matrix gate.
- The corrupt fixture snapshot is acceptable despite reporting the original file still exists.
- A failed technical harness can establish session serialization order.
- Another matrix rerun is authorized by this task.

## Required owner decision

The authorized no-rerun budget is exhausted. A future task must explicitly authorize a harness-only repair that either exposes PHP inside the Playwright execution environment or routes `runArtisan` through the application container, and must correct the one-corrupt fixture invariant before any further matrix execution. No product implementation should be inferred from this run.
