# Issue #365 exact-frozen Synology execution attempts

## Terminal classification

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
control_head: 8c58035cacb9fd4675d898a1652036fc8b9d4357
workflow_run: 30763456046
job: 91537990755
runner: oteryn-synology-staging
workers: 1
retries: 0
job_conclusion: cancelled
matrix_conclusion: cancelled
cleanup_conclusion: success
artifact_count: 0
classification: INVALID_TECHNICAL_FAILURE
root_cause_status: UNKNOWN
session_serialization_remediation: NOT_PROVEN_REMEDIATED
product_failure_inferred: false
production_action: none
```

Run `30763456046` is terminal. It reached the corrected exact-frozen matrix, but did not produce valid product evidence.

## Exact terminal evidence

- Validator-control checkout, frozen-target checkout, generator preparation, environment bootstrap and Laravel 13.20.0 observer installation all passed.
- The frozen target was exactly `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608`.
- Playwright used one worker and zero retries.
- Six clean samples were attempted: `clean-immediate-1..3` and `clean-prescroll-1..3`.
- Every attempted sample failed before the browser scenario because `helpers.mjs:93` invoked `php artisan cache:clear` with PHP `8.3.6`, while the installed frozen dependencies require PHP `>=8.5.0`.
- The Playwright command therefore never reached a valid publication-flow observation, request/session correlation or flash assertion.
- No exactly-one-corrupt sample completed. The run was cancelled while preparing the next sample after the six clean failures.
- The cancellation occurred at the bounded job timeout: the matrix started at `2026-08-02T20:10:06Z` and the operation was cancelled at `2026-08-02T21:29:07Z`.
- The upload action executed after cancellation but warned that none of the expected evidence files existed. GitHub's run artifact inventory is empty (`0` artifacts).
- Isolated containers, network, volumes, images, temporary files and both checkouts were cleaned successfully.

## First technical failure

```text
Composer detected issues in your platform:
Your Composer dependencies require a PHP version ">= 8.5.0".
You are running 8.3.6.
```

The temporary repair installed the Ubuntu distribution `php-cli`, which resolved the previous `spawnSync php ENOENT` failure but supplied PHP 8.3 rather than the lockfile-compatible PHP 8.5 runtime used by the application image. Reinstalling that package inside every sample also consumed most of the bounded execution window.

This is a validator/runtime compatibility defect, not an Oteryn portal product defect.

## Matrix accounting

| Fixture | Mode | Attempted | Valid browser flows | Result |
|---|---|---:|---:|---|
| clean | immediate | 3 | 0 | PHP platform check failure |
| clean | pre-scroll | 3 | 0 | PHP platform check failure |
| one-corrupt | immediate | 0 | 0 | not reached |
| one-corrupt | pre-scroll | 0 | 0 | not reached |

No clean-versus-corrupt comparison exists. No causal relationship between damaged media, session locking and the responsive-mobile publication-flash loss can be inferred from this run.

## Authorized-attempt history

| Control head | Run | Artifact | Classification |
|---|---:|---|---|
| `5cf9fee49927bd0f887131fe7e5ea7cf678d369b` | `30752369856` | `8834980323` | generator quoting failure |
| `cddb7578d89101e90fac1f9b8bdd85e4739d28c8` | `30752964863` | `8835208891` | Laravel observer pattern mismatch |
| `2bd32af496894403e0dec84efeca21b0642dcecd` | `30753618275` | none | wrong generated-layer repair |
| `e76f31cd9bf0dc7a5a8ffd73bda94bec6e1c9d9b` | `30756664833` | none | diagnostic-only gate |
| `ce9aac5865ee893150ac88e11123601362eaaf28` | `30756859088` | none | generated Python indentation failure |
| `7d8eed05826363baed47487ca71203caf1c993a9` | `30756908549` | none | `START_SESSION` scope failure |
| `613db96cda9d3ef513a033aff4a09b5e588798e9` | `30758971408` | `8837189083` | invalid technical execution: PHP absent |
| `8c58035cacb9fd4675d898a1652036fc8b9d4357` | `30763456046` | none | invalid technical execution: PHP 8.3 vs required 8.5; timeout |

The previous artifact `8837189083` remains the only retained partial package. Its ZIP digest is `sha256:03ced224c4e14b649f62a77e512821cffc5df679c425610da603137040f66fa0` and it proves the one-corrupt fixture state is `storage_exists=true`, `thumbnail_exists=false`; it does not contain valid product-flow evidence.

## Final audit boundary

```yaml
exact_frozen_matrix_gate: NOT_SATISFIED
reason: repeated validator runtime incompatibility
issue_365_root_cause: UNKNOWN
causal_link_to_damaged_media: UNKNOWN
responsive_mobile_flash_loss: REPRODUCED_INTERMITTENT_FROM_PRIOR_VALID_RUNS
session_serialization_remediation: NOT_PROVEN_REMEDIATED
audit_verdict: VALIDATED_WITH_CORRECTIONS
additional_rerun_authorized: false
```

The bounded audit execution budget is exhausted and no further matrix rerun is authorized by this task. A future implementation/validation task may create a new governed run only after supplying a lockfile-compatible PHP 8.5 executable to Playwright without per-sample package installation and preserving immutable evidence on cancellation.

No application code, dependency, production configuration, deployment, Issue lifecycle or external repository was changed by these attempts.