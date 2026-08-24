# Oteryn Platform Repository Map

Navigation map for autonomous agents. This is not an exhaustive inventory. Confirm current paths before editing.

## Product/application paths

| Area | Typical paths | Responsibility / cautions |
|---|---|---|
| Application core | `app/**` | Laravel application logic. Keep domain/module responsibilities explicit and avoid unrelated refactors. |
| HTTP / API | `routes/**`, `app/Http/**` | Public routes, middleware, controllers, request validation and API boundaries. Treat auth-sensitive endpoints as security-critical. |
| Authentication / identity | planned module paths under `app/**` plus auth configuration | Passwords, sessions, MFA, verification, recovery. Read security + game-login contract before implementation. |
| Database | `database/migrations/**`, `database/seeders/**`, `database/factories/**` | Platform schema/data changes. Shared Canary schema is contract-controlled. |
| Canary integration | planned integration adapters, `config/**`, `docs/contracts/**` | Never assume generic TFS/MyAAC schema. Verify Oteryn Canary evidence. |
| Public web / CMS | `resources/views/**`, `resources/js/**`, `resources/css/**` | Blade/frontend/CMS. Escape untrusted output and preserve CSRF protections. |
| Player Companion | future module paths under `app/PlayerCompanion/**`, module routes/views/tests | Versioned calculators, planning, hunt guidance, private session analysis and recommendations. Read ADR 0025 and the focused architecture before implementation. |
| Admin | planned admin controllers/routes/policies/views | Privileged operations. Explicit policies, MFA target and auditability required. |
| Tests | `tests/**` | Unit, feature, integration, contract and security regression tests. |
| Configuration | `config/**`, `.env.example` | Commit examples only. Never commit real credentials, tokens or production secrets. |
| CI | `.github/workflows/**` | Required checks and deployment validation. Do not weaken checks to obtain green CI. |

## Architecture and durable project memory

| Area | Path | Purpose |
|---|---|---|
| Current project state | `docs/agents/PROJECT_STATE.md` | Compact context snapshot for current phase, capabilities, unknowns and next work; newer live GitHub Issue/PR/`main` state supersedes it. |
| Active work index | `docs/agents/ACTIVE_WORK.md` | Convenience index; verify the governing GitHub Issue, individual task context and live PR. |
| Active tasks | `docs/agents/tasks/active/**` | Durable context/checkpoint/ownership/handoff records; they mirror lifecycle state but do not override the governing live GitHub Issue or PR. |
| Archived tasks | `docs/agents/tasks/archive/**` | Completed historical task records. |
| Architecture authority | `docs/architecture/ARCHITECTURE_AUTHORITY.md` | Canonical entry point for architecture precedence, focused ownership and conflict handling. Read this first for architecture-wide work. |
| System architecture | `docs/architecture/SYSTEM_ARCHITECTURE.md` | Current system context, trust boundaries and high-level dependency rules; explicitly labelled historical sections are context only. |
| Module catalog | `docs/architecture/MODULE_CATALOG.md` | Module responsibility and ownership. |
| Portal completeness | `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md` | Current portal assessment, benchmark dispositions, remaining architectural gaps and release-scope completion gate. |
| Portal completion delivery plan | `docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md` | Risk-first implementation sequence, vertical-slice gates and release closeout; subordinate to accepted ADRs/contracts and focused architecture. |
| Portal completion programme | `docs/agents/programs/OTERYN_PORTAL_COMPLETION.md`, `docs/agents/prompts/OTERYN-PORTAL-COMPLETION-EXECUTION-PROMPT.md` | Durable live-selection routing and execution contract for the `PORTAL-CLOSEOUT` alias. |
| Player Companion | `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md` | Player calculators/planners/session-analysis boundary, versioning, privacy, API/client reuse and P0–P2 delivery priorities. |
| Security architecture | `docs/architecture/SECURITY_ARCHITECTURE.md` | Mandatory security invariants. |
| Data ownership | `docs/architecture/DATA_OWNERSHIP.md` | Platform/Canary/shared persistent data rules. |
| Test strategy | `docs/architecture/TEST_STRATEGY.md` | Unit/feature/integration/contract/E2E strategy. |
| Roadmap | `docs/architecture/ROADMAP.md` | Phased delivery order and exit gates; roadmap intent is not implementation proof. |
| ADRs | `docs/architecture/adr/**` | Durable decisions and supersession history. Use the directory inventory, not numeric assumptions. |
| Integration contracts | `docs/contracts/**` | Canary/login-server/shared schema/auth compatibility. |
| Public endpoint roles | `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md`, `deploy/synology/PUBLIC_ENDPOINTS.md` | Canonical mapping of `oteryn.molehill.cloud` to Platform WWW and `gateway.molehill.cloud` to Game Gateway/login; ADR 0020 records the hostname decision. |
| Agent governance | `AGENTS.md`, `docs/agents/CONTEXT_*` | Coordination, routing and handoff rules. |

## Mandatory discovery for shared data/auth

Before implementing shared auth/account/character mutations, search/read:

- `docs/architecture/ARCHITECTURE_AUTHORITY.md`;
- `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md`;
- `docs/contracts/CANARY_DATA_CONTRACT.md`;
- `docs/architecture/DATA_OWNERSHIP.md`;
- `docs/architecture/SECURITY_ARCHITECTURE.md`.

Before implementing portal-completeness or player-tools work, also read:

- `docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md`;
- `docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md`;
- `docs/architecture/adr/0025-player-companion-and-portal-tools-boundary.md`;
- `docs/architecture/MODULE_CATALOG.md`;
- the exact Game Catalog/Wiki/PublicGameData/LiveOps/Game Analytics contracts required by the slice.

For public-domain, Cloudflare Tunnel, website-origin or Game Gateway hostname work, also read:

- `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md`;
- `deploy/synology/PUBLIC_ENDPOINTS.md`;
- `docs/architecture/adr/0020-use-single-level-gateway-public-hostname.md`.

Then verify the actual external repository/schema evidence. Documentation placeholders do not prove compatibility.

## Discovery commands

```sh
find . -name AGENTS.md -print
find docs/agents/tasks/active -maxdepth 1 -type f -print
rg -n "UNKNOWN|CONFLICT|DISCOVERY" docs/architecture docs/contracts docs/agents
rg -n "auth|session|mfa|password|account|player|guild|canary|login-server" app routes config database tests docs
rg -n "PlayerCompanion|LiveOps|calculator|ruleset_version|catalog_snapshot" app routes config database tests docs
rg -n "Route::|middleware|Gate::|Policy|Hash::|DB::transaction" app routes tests
```

Use targeted discovery. Do not preload the whole repository when a narrow search can identify the relevant module or contract.