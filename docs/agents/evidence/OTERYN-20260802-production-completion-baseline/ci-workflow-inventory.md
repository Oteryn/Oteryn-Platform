# CI workflow inventory

Observed from current `main` during Issue #452.

## Confirmed component-scoped workflows

### Game Gateway CI

File: `.github/workflows/game-gateway-ci.yml`

Triggers only for:

- `services/game-gateway/**`;
- `.github/workflows/game-gateway-ci.yml`.

Heavy work:

- Go formatting scan;
- `go test ./...`;
- `go vet ./...`;
- gateway binary build.

Assessment: **correctly scoped**. PR #225 should run this affected component gate after rebase. It should not trigger for unrelated documentation or Platform-only changes.

### Repair Synology Autostart

File: `.github/workflows/repair-synology-autostart.yml`

Triggers on `main` changes to:

- the workflow itself;
- `deploy/synology/compose.yml`;
- `deploy/synology/runner/compose.yml`;

and supports manual dispatch. It runs on `oteryn-staging`, enforces `restart=always`, starts the exact runner/service set and verifies policy/state.

Assessment: **appropriate operationally scoped workflow**. It supersedes the unmerged parallel boot-repair mechanism from PR #335.

## Required audit questions for broad workflows

For every broad required workflow determine:

1. Does `pull_request` use `paths`/`paths-ignore`?
2. If a stable required check name must always exist, does the workflow classify changes and make expensive jobs no-op while still emitting a successful required check?
3. Are Composer/npm installs, asset builds, database bootstraps, Docker builds and browser matrices repeated across workflows for the same PR head?
4. Can reusable workflows/artifacts remove identical setup without coupling unrelated gates?
5. Do documentation-only PRs execute application, container or browser work contrary to `BUILD_TEST_MATRIX.md`?
6. Are security-sensitive workflows path-filtered too narrowly, creating a false negative risk?
7. Are scheduled/manual evidence workflows non-blocking and excluded from normal PR execution?

## Safe target pattern

- One cheap change-classification/governance check always runs.
- Component jobs run only when their owned paths or shared dependencies change.
- Required check names remain stable through successful skip/no-op aggregator jobs where branch protection requires them.
- Full production-like, deployment, failure-mode and browser matrices run once on a coherent exact-final-head candidate when affected—not after every documentation checkpoint.
- Auth, payment, migration, contract and deployment changes retain fail-closed broad validation.

## Pending files

The audit still needs exact definitions for the broad workflow families repeatedly reported by documentation-only PRs:

- CI;
- Agent Governance;
- Phase 7 Production-Like Validation;
- Edge Security Emulation;
- Platform DB Outage Validation;
- Game Auth Ticket Concurrency;
- Acceptance E2E / Visual UX;
- Portal Acceptance Contract;
- Synology image build/deployment validation.
