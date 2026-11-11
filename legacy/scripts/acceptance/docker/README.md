# Local Docker portal E2E

This runner executes the repository-owned Oteryn Portal Playwright acceptance suite on a normal Docker host. It reuses the pinned PHP/Node/Playwright toolchain from `deploy/ci/playwright-php.Dockerfile` and starts isolated MariaDB, Redis and MailHog dependencies.

From PowerShell at the repository root:

```powershell
.\scripts\acceptance\docker\run.ps1 smoke
.\scripts\acceptance\docker\run.ps1 critical
```

Supported profiles: `smoke`, `critical`, `full`, `account-lifecycle`, `portability`, `responsive`, `resilience`, `accessibility`, and `coverage-strict`.

The `critical` profile mirrors the browser portions of the repository critical acceptance workflow: smoke, portability, responsive, resilience and accessibility. `full` runs the primary full Chromium baseline followed by resilience and accessibility.

Results are written under `artifacts/docker-portal-e2e/`. The Compose project uses task-scoped temporary containers, network and volumes; `run.ps1` removes them in `finally`, including after a failing test.

No staging/production endpoint, credential, database or persistent Docker volume is used by this runner.
