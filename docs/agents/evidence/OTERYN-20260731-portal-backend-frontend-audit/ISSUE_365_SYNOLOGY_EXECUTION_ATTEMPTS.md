# Issue #365 exact-frozen Synology execution attempts

## Status

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
environment_status: READY
observer_generation_status: PASS
observer_installation_status: PASS
matrix_launch_status: BLOCKED
matrix_status: NOT_RUN
root_cause_status: UNKNOWN
current_invocation_repair_budget: 3/3 exhausted
production_action: none
```

The Synology staging runner can build and bootstrap the production-like environment. The Laravel 13.20.0 observer is now generated, applied and syntax-checked successfully. The remaining blocker is narrower: the post-install verification opens a new `bash -lc` process and references `START_SESSION` without defining it in that shell. No mandatory browser sample started.

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

Run `30756908549` preserved artifact `8836419768` with digest `sha256:003f98c709141337255ca20b592faf74d237e38df3b3bf96b7d2e34429cb1144`.

## Proven facts from run 30756908549

- The exact frozen checkout remained separate from the validator-control branch.
- `Prepare isolated validator` passed, including `bash -n` of the generated validator.
- The production-like MariaDB, Redis and application bootstrap completed.
- Runtime configuration recorded Laravel `v13.20.0`, file sessions and file cache.
- `runtime/02-observer-patch.sh` contains the source-faithful blank-line pattern:

  ```text
  $this->saveSession($request);\n\n        return $response;
  ```

- `Issue365Trace.php` and instrumented `StartSession.php` both passed PHP syntax validation.
- `StartSession.sha256.instrumented` was created, proving the observer patch executed.
- `LAST_STAGE` is `observer-install`.
- The first terminal error is `bash: line 8: START_SESSION: unbound variable`.
- No `samples/` directory exists in the partial artifact; therefore zero mandatory samples started.
- Artifact upload and isolated runtime cleanup succeeded.
- No application, route, view, configuration, migration, dependency, deployment, production, authentication, authorization, MFA or publication-state change occurred.

## Classification

```yaml
exact_frozen_package: NOT_RUN
reason: post-install verification referenced START_SESSION in a fresh shell before the first sample
issue_365_completion: BLOCKED
session_serialization_remediation: NOT_PROVEN_REMEDIATED
causal_link_to_damaged_media: UNKNOWN
product_failure_inferred: false
```

This is a harness launch failure, not product evidence. Environment availability and source-faithful Laravel instrumentation are now proven.

## Rejected conclusions

- Synology or Docker availability remains the blocker.
- The Laravel observer still fails to match or install.
- A successful bootstrap or observer lint proves the publication defect remediated.
- The failed post-install shell proves a product failure.
- A partial or uncorrelated sample may satisfy the 12-sample completion gate.
- A fourth repair cycle is allowed in the current invocation.

## Required continuation

A fresh invocation must modify the generated post-install verification command so its inner `bash -lc` defines:

```bash
START_SESSION=vendor/laravel/framework/src/Illuminate/Session/Middleware/StartSession.php
```

before the first reference, or use that exact literal path in `observers-installed.txt`. Validate the generated script with `bash -n`, then execute at most one new Synology matrix run. The temporary validation PR must remain unmerged and must be closed after evidence is recorded.
