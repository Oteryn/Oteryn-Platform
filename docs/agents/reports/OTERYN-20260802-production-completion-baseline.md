# Oteryn Platform production-completion baseline

Status: **IN PROGRESS**  
Programme: #451  
Audit task: #452  
Branch: `audit/OTERYN-20260802-production-completion-baseline`

## Scope and evidence policy

This report reconciles the architecture/module plan, product-completeness evidence, current open pull requests and GitHub Actions validation policy. It records only repository/GitHub evidence available without a local checkout. Application-code remediation, local commands and real browser/runtime execution remain separate checkout-capable phases.

## Initial open-PR inventory

The invocation began with eleven open pull requests. The following dispositions are already proven:

| PR | Disposition | Evidence |
|---:|---|---|
| #182 | `close_obsolete` | Historical one-shot Liquid20 retry request; replaced by later retry/evidence work and no longer represents a current executable task. Closed 2026-08-02. |
| #189 | `close_obsolete` | Historical Liquid20 attempt record authorizing the same retry class; replaced by later programme state. Closed 2026-08-02. |
| #335 | `close_superseded` | `main` already uses `restart: always` for persistent Synology services and contains `.github/workflows/repair-synology-autostart.yml`, which enforces and verifies restart policies on the `oteryn-staging` runner. The old parallel DSM boot script would duplicate the current mechanism. Closed 2026-08-02. |
| #225 | `merge_ready_after_gate` | Narrow major update `actions/setup-go@v6` to `@v7` in the path-filtered Game Gateway workflow. Branch is stale/conflicted and requires Dependabot rebase/recreation plus the affected workflow gate. |
| #116 | `blocked_required_with_exact_dependency` | Evidence-only task for scheduled soak/stability runs under issue #114. Its original future-time blocker has expired, but actual scheduled run/artifact evidence is not available through the current connector view; do not close or merge until run history is checked. |

Remaining PRs requiring exact disposition review: #328, #338, #381, #387, #391, #405 and #412.

## Confirmed CI policy facts

The repository already defines the correct high-level strategy in `docs/agents/BUILD_TEST_MATRIX.md`:

- focused checks during implementation;
- component validation after a coherent milestone;
- one heavy exact-final-head validation after the implementation package is coherent;
- no application or container builds for documentation/task-record-only changes;
- early heavy validation only for dependencies, build tooling, generated output, migrations, protocol contracts, Docker/deployment or equivalent primary risks.

The current `Game Gateway CI` workflow is appropriately path-filtered to:

- `services/game-gateway/**`;
- `.github/workflows/game-gateway-ci.yml`.

It runs formatting, `go test ./...`, `go vet ./...` and a gateway build only when that component or its workflow changes. This is the desired model for component-heavy checks.

## Confirmed CI/build problem statement

Multiple documentation/evidence PR descriptions report the same broad workflow families—full CI, production-like validation, edge emulation, DB outage and game-auth concurrency—even when the remaining changed paths are documentation/task/evidence only. That conflicts with the repository build/test matrix unless those workflows themselves use reliable internal change classification or required-check no-op jobs.

This is not yet proof that each run performs all heavy steps; workflow definitions and job-level `if` conditions must be inspected before changing required checks. The remediation must preserve stable required check names while skipping expensive internals for unrelated path classes.

## Target validation matrix

| Change class | During work | Final required gate |
|---|---|---|
| Task records, reports, Markdown, evidence indexes | path/schema/link checks, `git diff --check` | governance/docs checks only; heavy app/browser/container jobs must become explicit successful no-ops or not trigger where branch protection permits |
| PHP/domain/backend | syntax, formatter/static analysis, focused unit/feature tests | affected component/integration suite; full CI once on exact final head |
| Blade/JS/CSS/user-facing UI | template/lint checks and focused UI tests | production asset build plus relevant zero-retry browser/E2E profiles |
| Auth, sessions, RBAC, payments, balances | immediate focused security/concurrency regression | broader security/integration and real E2E exact-final-head gate |
| Migration/schema/shared contract | syntax, isolated migration/rollback and contract tests | clean database integration plus compatibility gate |
| Go gateway only | Game Gateway CI | gateway test/vet/build only, plus cross-component contract gate when protocol changes |
| Docker/Synology/deployment/workflow | YAML/script-focused tests | image/health/rollback/staging checks only for affected deployment boundary |
| Dependency or lockfile | manifest/lock consistency and advisory review | clean install/audit and affected full build/test suite |

## Architecture/module baseline

The authoritative roadmap marks engineering phases 0–7 complete but explicitly leaves:

- the Production Go-Live Gate pending direct production verification;
- Phase 8 Payments, coins and shop deferred;
- authoritative game-login migration outside the completed Phase 5 boundary.

The module catalogue also distinguishes `AVAILABLE` from complete product coverage. Important known incomplete or blocked domains include:

- Payments and regulated commerce;
- stable Platform API;
- character rename and delete/restore implementation despite separate contract work;
- remaining authoritative Game Catalog producer/activation/public projections;
- exact private-production mail, queue, cache/session, observability, backup/restore and deployment evidence;
- exhaustive backend/frontend/state/browser closure tracked by #326/#365.

For programme #451, PLN and EUR for Poland/EU are accepted product constraints. Real charging remains a separate fail-closed activation gate.

## Immediate next actions

1. Finish evidence-backed dispositions for #328, #338, #381, #387, #391, #405 and #412.
2. Inspect all broad required workflow definitions, especially their `on.paths`, change classification and job-level `if` behavior.
3. Produce a workflow-by-change-class table and identify duplicate installs, builds, database bootstraps and browser matrices.
4. Create a bounded CI-remediation child task only after required-check compatibility is proven.
5. Reconcile the machine-readable backend/frontend ledger with later merged modules and classify the highest-priority READY implementation slice for #451.
