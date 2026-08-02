# Issue #365 exact-frozen Synology execution attempts

## Current status

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
environment_status: READY
observer_generation_status: PASS
observer_installation_status: PASS
current_control_head: 8c58035cacb9fd4675d898a1652036fc8b9d4357
current_run: 30763456046
current_job: 91537990755
matrix_launch_status: PASS
matrix_step_status: CANCELLED
artifact_upload_status: PASS
job_terminal_status: UNKNOWN_NON_TERMINAL_AT_LAST_ALLOWED_CHECK
root_cause_status: UNKNOWN
production_action: none
```

At the second and final permitted state check of the current invocation, run `30763456046` was still non-terminal. The corrected exact-frozen 12-sample matrix step had completed with conclusion `cancelled`; `Upload immutable Issue 365 evidence` had succeeded; isolated runtime cleanup was still in progress. The artifact has not been inspected because the run is not terminal. No causal or remediation conclusion may be inferred yet.

Do not poll or rerun this run again in the current invocation. When it becomes terminal in a later invocation, inspect the run once, verify the uploaded artifact and digest, classify what samples actually completed, synchronize Issue `#365` and PR `#381`, and close temporary PR `#476` without merge.

## Current corrected run

| Control head | Workflow run | Job | Artifact | Observed result |
|---|---:|---:|---|---|
| `8c58035cacb9fd4675d898a1652036fc8b9d4357` | `30763456046` | `91537990755` | uploaded, identifier and digest pending terminal inspection | Matrix step `cancelled`; artifact upload `success`; job cleanup still in progress at last allowed check. |

Proven before terminal inspection:

- validator control checkout passed;
- exact frozen target checkout passed;
- isolated validator preparation passed;
- the corrected matrix entered its runtime step with workers `1` and retries `0`;
- the matrix step ended as `cancelled`;
- immutable evidence upload succeeded;
- cleanup had not completed at the last permitted check;
- no rerun is authorized;
- no product, deployment, production or external-repository mutation occurred.

Unknown until terminal artifact inspection:

- why the matrix step was cancelled;
- how many of the 12 samples completed;
- whether clean and exactly-one-corrupt fixtures were both represented;
- whether request/session correlation data was preserved;
- artifact identifier, digest and internal completeness;
- whether the run can be classified as valid product evidence, partial diagnostic evidence or invalid technical execution.

## Previous terminal authorized run

Run `30758971408` reached the matrix stage on the Synology runner. The generated validator passed `bash -n`, the frozen target bootstrapped, and the Laravel 13.20.0 observer installed and linted. The package is not valid product evidence because every completed clean sample failed before the browser flow at `spawnSync php ENOENT`, and the first corrupt-fixture setup then failed its own snapshot invariant.

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

## Isolation and harness repair cycles

| Cycle | Control head | Workflow run | Result |
|---:|---|---:|---|
| 1 | `e76f31cd9bf0dc7a5a8ffd73bda94bec6e1c9d9b` | `30756664833` | Diagnostic-only gate proved the parent validator contains zero `saveSession` matches; the pattern belongs to generated `runtime/02-observer-patch.sh`. Matrix intentionally did not start. |
| 2 | `ce9aac5865ee893150ac88e11123601362eaaf28` | `30756859088` | Cheap preparation gate rejected an invalid nested Python string with `IndentationError`; matrix skipped. |
| 3 | `7d8eed05826363baed47487ca71203caf1c993a9` | `30756908549` | Preparation passed, environment bootstrapped, the generated runtime observer was corrected and installed, then post-install verification failed with `START_SESSION: unbound variable` before sample creation. |
| 4 | `613db96cda9d3ef513a033aff4a09b5e588798e9` | `30758971408` | START_SESSION repair passed; matrix began but the Playwright container lacked a callable `php` binary and the corrupt-fixture invariant also failed. |
| 5 | `8c58035cacb9fd4675d898a1652036fc8b9d4357` | `30763456046` | PHP and corrupt-fixture state repairs passed preparation; matrix step later became `cancelled`, artifact upload succeeded, terminal classification pending. |

## Current classification

```yaml
previous_exact_frozen_package: INVALID_TECHNICAL_FAILURE
current_exact_frozen_package: UNKNOWN_PENDING_TERMINAL_ARTIFACT_INSPECTION
issue_365_completion: WAITING_EXTERNAL_RUN_TERMINAL_STATE
session_serialization_remediation: NOT_PROVEN_REMEDIATED
causal_link_to_damaged_media: UNKNOWN
product_failure_inferred_from_current_run: false
```

The previous terminal run proves environment and observer installation but not the product hypothesis. The current run proves that the corrected harness reached the matrix and persisted an artifact, but cancellation reason and evidence completeness remain unknown.

## Rejected conclusions

- `spawnSync php ENOENT` is a product defect.
- Six failed clean samples satisfy the matrix gate.
- The previous corrupt fixture snapshot is acceptable despite reporting the original file still exists.
- A failed technical harness can establish session serialization order.
- A cancelled matrix step automatically proves product failure or remediation.
- Successful artifact upload means the evidence package is complete.
- Another matrix rerun is authorized by this task.

## Required next action

After run `30763456046` is terminal, inspect it exactly once, verify the uploaded artifact and its digest, classify completed samples and correlation evidence, update Issue `#365` and PR `#381`, and close PR `#476` without merge. Until then, task status remains `waiting` and root cause remains `UNKNOWN`.
