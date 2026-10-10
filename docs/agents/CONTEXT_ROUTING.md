# Oteryn Platform Agent Context Routing

## Goal

Load the smallest required context for the current task. Do not preload broad repository documentation when targeted search can identify the relevant records.

## Governance contract boundary

- `docs/agents/GOVERNANCE_CONTRACT.json` is the machine-readable version marker for the shared checkpoint/handoff contract.
- `docs/agents/CONTEXT_HANDOFF.md` is the human-readable checkpoint/handoff contract used by Oteryn Platform.
- This routing document is repository-specific. It is not required to match Canary routing content or Canary's machine-readable router.
- A change to shared checkpoint/handoff structure must follow the upgrade process in `GOVERNANCE_CONTRACT.json`; repository-specific routing changes do not by themselves require a shared contract version bump.

## On-demand task routing

Root `AGENTS.md` defines the small mandatory instruction set. This file is an optional selector for a task whose domain context is not already clear from the governing Issue and changed paths.

When used, classify the task and load only the matching route context. The active task record is durable context/evidence/ownership/handoff, not a competing lifecycle authority; reconcile stale lifecycle or PR fields against live GitHub before acting.

`REPOSITORY_MAP.md`, `PROJECT_STATE.md`, `BUILD_TEST_MATRIX.md` and other references are task-routed or optional/reference context unless the selected route or an explicit safety/validation trigger requires them. Do not recursively follow links merely because a loaded document mentions another document.

## Routing table

| Route | Trigger | Load / search |
|---|---|---|
| `agent-governance` | `AGENTS.md`, `docs/agents/**`, `tools/agents/**`, ownership or handoff | Read `GOVERNANCE_CONTRACT.json`, relevant governance/handoff records, `tasks/TASK_TEMPLATE.md` and targeted overlapping active tasks. |
| `architecture` | new module, durable boundary, major dependency, product architecture | Read `ARCHITECTURE_AUTHORITY.md` first, then the focused owner named there, relevant ADRs and contracts. Create or supersede an ADR when a decision outlives one task. |
| `web-cms` | Blade/views/CMS/news/public pages | Read relevant module catalog section; search affected routes, controllers, views and tests. Check escaping, sanitization, authorization and CSRF boundaries. |
| `auth-identity` | login, password, sessions, MFA, verification, recovery | Read `SECURITY_ARCHITECTURE.md` and `AUTH_GAME_LOGIN_CONTRACT.md`, then relevant auth config/code/tests. Treat unresolved game-login compatibility as a blocker for global-security claims. |
| `native-character-portfolio` | native Oteryn-v2 authenticated character-portfolio reads (Account Center composition, capability provenance, freshness, privacy, failure states) | Read `docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md` first, then `DATA_OWNERSHIP.md` (section `Native Oteryn-v2 Character Authority boundary`). A Platform cache or read model is non-authoritative and never proves ownership. |
| `native-characters` | native Oteryn-v2 character mutations: create, rename, delete/restore, world or account transfer | Read `DATA_OWNERSHIP.md` (section `Native Oteryn-v2 Character Authority boundary`) first, then `docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md`. Native target is the Game character authority boundary, not Canary rows. Do not use `CHARACTER_CREATION_CONTRACT.md` as native authority; it governs the Canary compatibility path. |
| `native-public-game-data` | native Oteryn-v2 highscores, characters, guilds and individual character presence read models | Game-owned authoritative source facts feed Platform-owned projections/read models; the Platform owns projection, indexing and freshness (see `DATA_OWNERSHIP.md` and `docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md`). Prefer read-only boundaries. |
| `native-runtime-status` | native Oteryn-v2 world/channel runtime status: health, readiness, capacity, player-count aggregates, public runtime truthfulness | Read `docs/architecture/LIVEOPS_ARCHITECTURE.md` (live operational state, freshness, maintenance versus observed runtime state, LiveOps query boundary) and `docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md`, which distinguishes stale or unavailable evidence from authoritative offline state; read `DATA_OWNERSHIP.md` for the ownership split. |
| `accounts-characters` | **LEGACY / COMPATIBILITY** — account/player creation or management on the Canary-compatible path | Read `docs/contracts/CHARACTER_CREATION_CONTRACT.md` and `CANARY_DATA_CONTRACT.md` as compatibility evidence, plus the narrower operation contracts it delegates to (`docs/contracts/PLATFORM_CANARY_ACCOUNT_PROVISIONING_CONTRACT.md`, `docs/contracts/IDENTITY_CANARY_ACCOUNT_BINDING_CONTRACT.md`, `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`, and `docs/contracts/CHARACTER_DELETION_CONTRACT.md` for any character deletion, restoration or finalization, where direct Platform deletion is NO-GO) for the operations in scope; read `DATA_OWNERSHIP.md` (section `Native Oteryn-v2 Character Authority boundary`) for the native boundary. Load affected models/services/tests only after required contract fields are proven. |
| `public-game-data` | **LEGACY / COMPATIBILITY** — highscores, characters, guilds, online/status read from Canary tables | Read relevant `CANARY_DATA_CONTRACT.md` sections only as compatibility evidence and query/read-model code; the native target is the `native-public-game-data` route for individual character presence and the `native-runtime-status` route for world/channel status, health, readiness, capacity and aggregate counts. Prefer read-only boundaries. |
| `canary-integration` | **LEGACY / COMPATIBILITY** — shared DB, login-server, Canary schema/protocol contract | Native replacements are split by owner: Platform owns `AccountId`, credential security, login-ticket issuance/redemption, routing policy and Gateway pre-admission (route native account-identity and login work to `docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md` and `docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md`), while Game owns final admission and gameplay-session authority (see `docs/contracts/GAME_SESSION_CANARY_CONTRACT.md`, LCFA and the character authority boundary); read matching `*CANARY*` documents in `docs/contracts/**` plus `docs/contracts/CHARACTER_CREATION_CONTRACT.md`, `docs/contracts/CHARACTER_TRANSFER_CONTRACT.md`, `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md` and `docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md` only for the compatibility path; verify live Canary/login-server evidence before compatibility claims. |
| `database` | migration, transaction, locking, schema, query behavior | Read `DATA_OWNERSHIP.md`, matching migrations/models and concurrency tests. Require rollback thinking for destructive changes. |
| `admin-rbac` | admin panel, roles, policies, privileged actions | Read `SECURITY_ARCHITECTURE.md`, Admin module catalog section, authorization policies and audit requirements. Deny by default. |
| `api` | REST/API endpoints, external clients | Load routes, request validation, auth middleware, rate limits and API tests. Reuse module services rather than duplicating business logic. |
| `security` | vulnerability, secret, traversal, XSS, CSRF, SSRF, injection, abuse | Read `SECURITY_ARCHITECTURE.md` and affected surface/tests. Record threat assumptions explicitly and add regression tests where practical. |
| `testing` | test infrastructure, CI validation, E2E | Apply `BUILD_TEST_MATRIX.md`, then read `TEST_STRATEGY.md` and affected contracts/modules. Tie compatibility evidence to exact versions/SHAs where practical. |
| `payments` | payment provider, coins, premium currency, webhook, shop | Read `docs/architecture/adr/0021-provider-neutral-payment-security-core.md`, `docs/architecture/SECURITY_ARCHITECTURE.md` section `Future payments`, and `docs/architecture/DATA_OWNERSHIP.md` section `Future financial data`. Load `docs/operations/PAYMENTS_SECURITY_FOUNDATION.md` only for provider-specific operations, activation, reconciliation/alerting or production-readiness detail. For the payment-domain route itself, do not add an agent/META detour unless a separate governance or authority trigger requires it. |
| `ci-repair` | required GitHub check fails | Read the failing workflow/job/step and current task. Investigate root cause before rerun. |
| `execution-resources` | create or control containers, services, runners, volumes or persistent temporary infrastructure | Read `EXECUTION_RESOURCE_HYGIENE.md` and apply its ownership-scoped inventory and cleanup rules. |

Multiple routes may apply, but each must be justified by task scope or evidence.

## Canary routes are legacy

The `accounts-characters`, `public-game-data` and `canary-integration` routes cover the Canary compatibility path only. The native target is defined by the Game contracts: LCFA, native login and the character authority boundary (`DATA_OWNERSHIP.md`, section `Native Oteryn-v2 Character Authority boundary`). Do not extend the shared-DB path for new work without an explicit task.

Model and tier policy is not set in this repository; follow `docs/agents/META_AGENT_POLICY_BINDING.json`.

## Authoritative architecture documents

Start architecture-wide work at `docs/architecture/ARCHITECTURE_AUTHORITY.md`. It defines precedence and routes each concern to its focused owner.

Use targeted sections, not automatic full-document loading:

- `docs/architecture/ARCHITECTURE_AUTHORITY.md` — authority, precedence, source state and conflict handling;
- `docs/architecture/SYSTEM_ARCHITECTURE.md` — system boundaries and target topology;
- `docs/architecture/MODULE_CATALOG.md` — responsibility and dependency ownership;
- `docs/architecture/SECURITY_ARCHITECTURE.md` — mandatory security invariants;
- `docs/architecture/DATA_OWNERSHIP.md` — persistent data ownership and shared write rules;
- `docs/architecture/TEST_STRATEGY.md` — validation layers and production E2E expectations;
- `docs/architecture/ROADMAP.md` — phase ordering and gates;
- `docs/architecture/adr/**` — durable decisions and supersession history;
- `docs/contracts/**` — cross-component compatibility contracts.

Implementation evidence proves delivered state but does not silently supersede accepted decisions. Historical or explicitly superseded sections are context only.

## Search before read

Before creating a new abstraction or integration point, search:

1. governing or related GitHub Issues, active task context records and open PRs for overlapping paths or intent;
2. `ARCHITECTURE_AUTHORITY.md` for the focused canonical owner;
3. `MODULE_CATALOG.md` for an existing module owner;
4. repository source/tests for existing implementations;
5. relevant ADRs and contracts.

Do not recursively follow documentation links without evidence that they are relevant.

## Context expansion rule

Expand context only when:

- a search result points to another authoritative source;
- a required fact remains `UNKNOWN` after targeted search;
- authoritative sources conflict;
- validation or a safety gate explicitly requires more context.

## Working-set discipline

Keep the active working set limited to:

- current task goal and acceptance criteria from the governing live GitHub Issue/task;
- live branch/head/PR state;
- affected paths and ownership;
- `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT` facts;
- relevant source excerpts;
- latest validation evidence;
- exactly one next concrete action.

Chat history is not authoritative project memory.
