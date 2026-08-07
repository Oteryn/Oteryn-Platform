# Oteryn Platform Build and Test Matrix

```yaml
actions_economy_policy_version: 1
```

Validation must be proportional to changed paths, risk and the current project milestone. A commit or small task step does not by itself justify a full build or test suite.

## Actions economy invariants

- A coherent multi-file implementation should normally produce one reviewable push, not one Actions generation per file.
- Checkpoint, evidence and documentation updates are batched at material boundaries.
- Superseded runs for an older head of the same PR are cancelled automatically.
- Heavy workflows use trigger-level governance/checkpoint exclusions plus internal fail-closed path classification.
- Final applicable validation runs once on the exact candidate head; focused checks may run earlier.
- A checkpoint-only or agent-governance-only commit does not start unrelated edge, outage, production-like or concurrency workflows.

## Validation timing and escalation

- During a multi-step task, run cheap focused checks after each coherent step: syntax, formatting, static analysis, schema/contract validation and directly affected tests.
- Defer dependency installation, asset compilation, container builds and broad test suites until the end of a coherent milestone, phase or implementation package. A five-step feature should normally receive one heavy validation pass after the five steps form one reviewable result, not after every step.
- Run heavy validation earlier only when a step changes dependency manifests or lockfiles, build tooling, generated assets, framework bootstrap, shared contracts, migrations needed by subsequent steps, container definitions, or when later work requires a verified artifact.
- Documentation, task-checkpoint, comment, metadata and other clearly non-runtime/non-build-affecting commits do not require application or container builds.
- Security-sensitive behavior still requires focused regression tests as soon as the behavior exists; batching must not postpone detection of an unsafe auth, authorization, session, payment or data-integrity design.
- Run the full applicable final validation once on the exact final head before merge. A later runtime/build-affecting commit invalidates it; a later docs-only commit needs only the checks selected by repository path policy.
- Record why a heavy check was run early or skipped when the choice is not obvious from changed paths.

| Change | Minimum validation during the milestone | Heavy/final validation |
|---|---|---|
| Documentation/task records | Markdown/path/link review, `git diff --check` | Docs/fast checks; no application or container build |
| PHP/Laravel implementation | Syntax/static checks and directly affected unit/feature tests | Relevant broader suite at milestone completion |
| Blade/JS/CSS/assets | Template/lint/type checks and focused UI behavior | Asset production build and browser/E2E when affected |
| Composer/npm dependency or lockfile | Manifest/lock consistency immediately | Clean install, audit and full affected build/test suite |
| Migration/schema | Migration syntax, rollback and isolated focused test | Clean database migration/integration validation |
| Auth/security/authorization | Focused regression test as soon as behavior exists | Broader security/integration suite before merge |
| API/cross-repo contract | Focused contract tests and exact dependency evidence | Compatible Canary/client/platform integration |
| Docker/deployment/workflow | YAML/config validation and focused script tests | Image build, health/rollback/staging checks when affected |
| CI/governance only | YAML/schema/check-name review | Observe emitted lightweight checks; no unrelated application build |

Discover actual commands from current `composer.json`, package manifests, repository docs and workflows. Never invent a command or claim a result that was not observed.
