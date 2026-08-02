# CI remediation acceptance contract

A future CI-routing implementation is acceptable only when all of the following are proven on the same candidate head.

## Change classes

Representative fixtures or synthetic commits must cover:

1. docs/task/evidence only;
2. agent governance/tooling;
3. PHP backend/domain;
4. Blade/JS/CSS frontend;
5. Composer dependency/lockfile;
6. migration/database provisioning;
7. auth/session/RBAC/security;
8. payment/webhook/balance;
9. Go gateway;
10. Docker/Synology/deployment;
11. edge/proxy/security headers;
12. shared files that affect multiple classes;
13. workflow file self-change.

## Required outcomes

- docs-only: governance/docs checks run; heavy application, browser, database outage, edge and production-like internals skip.
- backend/frontend: CI and affected component/browser jobs run.
- shared dependency/framework/config: every plausibly affected gate runs fail-closed.
- auth/payment/migration/contracts/deployment: broad heavy gates remain mandatory.
- gateway-only: Game Gateway CI runs; unrelated Platform full matrices skip unless a shared protocol changes.
- stable branch-protection check names are emitted for every PR, either by actual affected validation or an explicit successful classified no-op.
- no skipped job is represented as product validation evidence.
- exact change classification is recorded in job summary/artifact.
- candidate and baseline are compared on the same representative cases.

## Cost/outcome evidence

Record for baseline and candidate:

- workflows/jobs started per change class;
- dependency installations;
- MariaDB/Redis/MailHog service starts;
- Docker image builds;
- Playwright browser installs/runs;
- total billed runner time where available;
- required-check result names;
- deliberately injected defects that must still be caught.

The change must be rolled back if it creates a false-green path or required-check ambiguity.
