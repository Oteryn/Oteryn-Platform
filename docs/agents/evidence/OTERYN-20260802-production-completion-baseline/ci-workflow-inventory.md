# CI workflow inventory

Observed from `main` at `52064fc880b4edbb2d479692f7c3e29530bbfaea` and from pull-request runs on documentation-only PR #453.

## Confirmed over-triggered workflows

| Workflow | PR trigger | Heavy boundary observed | Finding |
|---|---|---|---|
| CI | every PR to `main` | MariaDB, Composer install/audit, Pint, PHPStan, full PHP tests | Unrelated docs-only execution. |
| Phase 7 Production-Like Validation | every PR to `main` | MariaDB, Redis, MailHog, production/development installs, migrations, privilege, backup/restore and runtime validation | Broadest unrelated docs-only execution. |
| Edge Security Emulation | every PR to `main` | nginx/DNS/Docker tooling, Composer, DB, Laravel origin and edge evidence | Unrelated docs-only execution. |
| Platform DB Outage Validation | every PR to `main` | MariaDB, production install, migrations, live runtimes and outage/recovery proof | Unrelated docs-only execution. |
| Game Auth Ticket Concurrency | every PR to `main` | MariaDB, Composer and independent-process concurrency proof | Unrelated docs-only execution. |

All five families started on PR #453 despite its 13 changed files being limited to task/report/evidence paths. On head `2c1535c3e2a0b223ab2d704937e2bed1e4aa1744`, all five and Agent Governance completed successfully. These runs prove the routing defect; they are not evidence that runtime validation was necessary for the documentation change.

## Correctly scoped controls

- **Agent Governance** — agent-governance paths only; appropriate docs/governance gate.
- **Game Gateway CI** — `services/game-gateway/**` and its workflow; appropriate component gate.
- **Portal Acceptance Contract** — portal/application/ledger/dependency paths; no docs-only trigger defect proven.
- **Build Synology Staging Images** — application/gateway/deployment/dependency paths; no docs-only trigger defect proven.

## Safe target pattern

1. One cheap change classifier runs on every PR and records the class.
2. Stable required check names remain available through explicit successful no-op/aggregator jobs when unaffected.
3. No-op is labelled as routing evidence, never product-validation evidence.
4. Shared framework/config, auth, payments, migrations, security, deployment and workflow self-changes run broad gates fail-closed.
5. Component-only changes run the smallest sufficient component gate.
6. Deterministic positive, negative, boundary and deliberately defective fixtures prove that no false-green path exists.
7. Baseline and candidate compare workflow/job count, dependency installs, service starts, Docker builds, browser runs and runner time.

Exact implementation acceptance is in `ci-remediation-acceptance.md`.
