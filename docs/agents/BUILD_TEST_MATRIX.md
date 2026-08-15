# Oteryn Platform Build and Test Matrix

```yaml
actions_economy_policy_version: 2
workflow_lifecycle_policy: docs/agents/CI_WORKFLOW_LIFECYCLE.json
coverage_policy: docs/agents/CI_COVERAGE_POLICY.json
```

Validation must be proportional to changed paths, risk and the current project milestone. A commit or small task step does not by itself justify a full build or test suite.

## Actions economy invariants

- A coherent multi-file implementation should normally produce one reviewable push, not one Actions generation per file.
- Checkpoint, evidence and documentation updates are batched at material boundaries.
- Superseded runs for an older head of the same PR are cancelled automatically when the workflow is supersedable.
- Automatic domain workflows use trigger-level `paths`/`paths-ignore` whenever they are not intentionally repository-wide.
- Heavy workflows use trigger-level governance/checkpoint exclusions plus internal fail-closed path classification where applicable.
- Final applicable validation runs once on the exact candidate head; focused checks may run earlier.
- A checkpoint-only or agent-governance-only commit does not start unrelated edge, outage, production-like, browser or concurrency workflows.
- A workflow-file edit does not imply every runtime risk changed: ordinary workflow definitions route to core CI, heavy workflow definitions route to core CI plus their own lane, and only central routing-control changes fail closed to every gate.
- Workflow definitions are durable architecture, not task history. New or retained workflow files must satisfy `CI_WORKFLOW_LIFECYCLE.md` and the machine lifecycle registry.

## Validation timing and escalation

- During a multi-step task, run cheap focused checks after each coherent step: syntax, formatting, static analysis, schema/contract validation and directly affected tests.
- Defer dependency installation, asset compilation, container builds and broad test suites until the end of a coherent milestone, phase or implementation package. A five-step feature should normally receive one heavy validation pass after the five steps form one reviewable result, not after every step.
- Run heavy validation earlier only when a step changes dependency manifests or lockfiles, build tooling, generated assets, framework bootstrap, shared contracts, migrations needed by subsequent steps, container definitions, or when later work requires a verified artifact.
- Documentation, task-checkpoint, comment, metadata and other clearly non-runtime/non-build-affecting commits do not require application, browser or container builds.
- Security-sensitive behavior still requires focused regression tests as soon as the behavior exists; batching must not postpone detection of an unsafe auth, authorization, session, payment or data-integrity design.
- Run the full applicable final validation once on the exact final head before merge. A later runtime/build-affecting commit invalidates it; a later docs-only commit needs only the checks selected by repository path policy.
- Record why a heavy check was run early or skipped when the choice is not obvious from changed paths.

## Change-to-proof matrix

| Change | Minimum validation during the milestone | Heavy/final validation |
|---|---|---|
| Documentation/task records | Markdown/path/link review, `git diff --check` | Docs/fast checks; no application/browser/container build |
| PHP/Laravel implementation | Syntax/static checks and directly affected unit/feature tests | Relevant broader suite at milestone completion |
| Blade/JS/CSS/assets | Template/lint/type checks and focused UI behavior | Asset production build and browser/E2E when affected |
| Composer/npm dependency or lockfile | Manifest/lock consistency immediately | Clean install, audit and full affected build/test suite |
| Migration/schema | Migration syntax, rollback and isolated focused test | Clean database migration/integration validation |
| Auth/security/authorization | Focused regression test as soon as behavior exists | Broader security/integration suite before merge |
| API/cross-repo contract | Focused contract tests and exact dependency evidence | Compatible producer/consumer integration when authorized |
| Docker/deployment/workflow | YAML/config validation and focused script tests | Image/build/health/rollback/staging checks only when affected |
| Ordinary workflow definition | Workflow inventory, immutable-action pinning, trigger economy, core CI | The workflow's own affected lane; no unrelated heavy fan-out |
| Central CI router (`ci.yml`, `scripts/ci/**`, `tests/ci/**`) | Routing fixtures + workflow inventory/economy contracts | Fail-closed heavy-gate generation on the candidate head |
| CI/governance only | YAML/schema/check-name review | Observe emitted lightweight checks; no unrelated application build |

## Workflow lifecycle

Use `docs/agents/CI_WORKFLOW_LIFECYCLE.md` as the human contract and `docs/agents/CI_WORKFLOW_LIFECYCLE.json` as the machine registry.

- Prefer another job, matrix/profile or reusable workflow before adding a new workflow file.
- A feature/test/task name alone is not a durable workflow boundary.
- A temporary/task-specific workflow is removed before terminal merge unless a durable trigger/permission/environment/gate lifecycle is explicitly proven.
- Historical workflow runs remain provenance in GitHub/Git history; an executable workflow file is not an archive.
- The registry has a reviewed workflow budget. Adding a workflow requires explicit lifecycle review in the same PR rather than silent inventory growth.
- Retired workflow names fail closed if reintroduced without a deliberate policy change.

## Code coverage

Pull-request CI keeps coverage instrumentation disabled to protect fast feedback. Relevant `main` pushes measure PHP application statement/method coverage in the existing `CI` workflow and retain a bounded report artifact.

`docs/agents/CI_COVERAGE_POLICY.json` is intentionally `report_only` until a directly observed stable baseline exists. Historical CI used `coverage: none`, so any immediate numerical floor would be invented. After a verified baseline is observed, promote the policy to `enforce` with a reviewed ratchet floor.

Coverage does not replace security regression, database integration, concurrency, contract, browser or production proof.

Discover actual commands from current `composer.json`, package manifests, repository docs and workflows. Never invent a command or claim a result that was not observed.
