# Issue #365 exact-frozen Synology execution attempts

## Status

```yaml
frozen_target: b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608
environment_status: READY
harness_status: BLOCKED
matrix_status: NOT_RUN
root_cause_status: UNKNOWN
repair_budget: 3/3 exhausted
production_action: none
```

The Synology staging runner is available and can build the production-like Docker environment. The remaining blocker is not environment availability. It is a deterministic incompatibility in the temporary generator that prepares the ephemeral Laravel `StartSession` observer. No browser sample from the mandatory 12-sample matrix started.

## Bounded repair attempts

| Cycle | Control head | Workflow run | Last reached stage | Artifact | Result |
|---:|---|---:|---|---|---|
| 1 | `5cf9fee49927bd0f887131fe7e5ea7cf678d369b` | `30752369856` | `observer-generation` | `8834980323`, `sha256:08b677baf46d2d4a52ef7fe18234c05804a6f6e655901fb9dad42929ecee8783` | Generated Python selector lost quoting around `"$RUN_ROOT/media-snapshot.php"`. |
| 2 | `cddb7578d89101e90fac1f9b8bdd85e4739d28c8` | `30752964863` | `observer-install` | `8835208891`, `sha256:4861e421e4c4575f3f22ff5461ee16070c79114639ddb8c8f736afd1010d190c` | Exact Laravel 13.20.0 `StartSession.php` did not contain the expected adjacent `saveSession` / `return` source pattern. |
| 3 | `2bd32af496894403e0dec84efeca21b0642dcecd` | `30753618275` | validator preparation | none | Wrapper repair itself failed closed: `Laravel 13 StartSession save-pattern repair: expected one match, found 0`. Matrix step was skipped. |

## Proven execution facts

- All attempts checked out frozen target `b6f7b12a43aa72a52dc98c3fa07a7c4607fcb608` separately from the validator control branch.
- Runs 1 and 2 proved an initially clean frozen Git worktree.
- Run 2 built the platform and validator images, installed development dependencies, and reached a healthy production-like MariaDB/Redis/application bootstrap.
- Run 2 preserved the exact Laravel 13.20.0 framework file and generated observer scripts in the immutable partial artifact.
- The actual framework source contains a blank line between `$this->saveSession($request);` and `return $response;`; the retired runbook generator expected them to be adjacent.
- Artifact upload and isolated runtime cleanup succeeded after both runtime failures.
- Run 3 failed before runtime creation and therefore correctly produced no evidence archive.
- No application, route, view, configuration, migration, dependency, deployment, production, authentication, authorization, MFA, audit or publication-state change occurred.
- Temporary PR #412 remains non-mergeable validation infrastructure and must be closed without merge.

## Classification

The earlier `BLOCKED_ENVIRONMENT` classification is superseded. The environment is now proven capable. The exact-frozen causal gate remains incomplete because the bounded temporary harness could not install a source-faithful session observer within the three-cycle repair budget.

```yaml
exact_frozen_package: NOT_RUN
reason: no mandatory browser sample started because observer generation remained incompatible with the exact Laravel 13.20.0 source
issue_365_completion: BLOCKED
session_serialization_remediation: NOT_PROVEN_REMEDIATED
causal_link_to_damaged_media: UNKNOWN
```

## Rejected conclusions

- Synology or Docker availability is still the blocker.
- A successful production-like bootstrap proves the publication-flash defect remediated.
- The failed observer installation is product evidence.
- The historical thumbnail failures cause the flash loss.
- A partial or uncorrelated sample may satisfy the 12-sample completion gate.

## Required continuation

A fresh invocation must first generate and inspect `/tmp/issue365-validator.sh` in a cheap syntax-only job, then patch the exact generated `StartSession` observer search text against Laravel 13.20.0. Only after that cheap gate passes may one new Synology matrix run be executed. PR #412 must not be merged, and the repair must not alter the frozen application source outside the isolated runtime.
