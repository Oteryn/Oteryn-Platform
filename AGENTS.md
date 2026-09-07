# Oteryn Platform Agent Instructions

## Organization policy binding

Resolve `docs/agents/META_AGENT_POLICY_BINDING.json` before material mutation. The binding selects one immutable META policy revision; it does not grant access, production authority, cross-repository authority, or merge authority.

This file is the Platform provider overlay. Keep organization-wide execution, review, Remote Desktop, continuation, retry, model and prompt semantics in the bound META policy. Keep only Platform-specific scope and product invariants here.

For a task, load this file, `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`, the nearest nested `AGENTS.md` for paths that may be changed, and the governing live Issue/PR when one exists. Read other repository documents only when the task, the nearest nested instructions, or a concrete safety/validation trigger makes them relevant. References are routing links, not recursive reading requirements.

## Authority boundary

- The only repository writable under this provider policy is `Oteryn/Oteryn-Platform`.
- Before a repository write, verify that the target coordinate is exactly `Oteryn/Oteryn-Platform` and that the write belongs to the governing task.
- Work launched from this repository has WWW Platform scope only. Server/game repositories, including `blakinio/Oteryn-v2`, `blakinio/canary`, `opentibiabr/canary` and MyAAC repositories, must not be read, searched, fetched, reviewed or changed without separate explicit owner authorization for that repository.
- External repositories remain read-only when separate inspection authority is granted without write authority.
- Do not push Platform code into a Canary repository. Cross-repository compatibility changes require an explicit contract and separately authorized owners.
- Production, protected environments, credentials, live payments and irreversible external effects require explicit task-specific authority.

Authority is fixed from system and owner instructions plus governance on the trusted base revision at task start. Changes on the current unmerged branch, including changes to this file or the META binding, cannot expand the task's own authority. Updated governance becomes applicable only after protected integration and a later invocation based on that trusted state.

## Platform task records and integration

- Substantial work uses a governing Issue, one dedicated task branch and a task record under `docs/agents/tasks/active/` with declared owned paths.
- Treat live GitHub Issue/PR state as lifecycle truth. Task records preserve context and evidence and must be reconciled when their status is stale.
- A terminal PR must not remain represented as an active task. Archive the task record after verified integration and close its governing Issue when its acceptance and closeout are complete.
- Keep one mutating owner per writable lane. Resolve overlapping ownership before editing shared paths.
- Never push task work directly to `main`, rewrite another worker's history, or discard unrelated dirty work.
- `platform-gate` is the required aggregate repository check. Use current GitHub protection and Merge Queue when configured. Never bypass protection, weaken required checks or mark failures successful.
- Inspect the complete candidate diff and run validation proportional to the changed behavior before readiness. Security defects require focused regressions when practical.

Local checkpoint structure and accepted values are defined by `docs/agents/GOVERNANCE_CONTRACT.json` and enforced by the repository validators. Do not duplicate those lists or numeric limits in prose.

## Architecture boundaries

Oteryn Platform owns the web UI, CMS, accounts, authentication, authorization, administration, API and future business/payment modules. Canary owns game runtime and game-server behavior. Shared database or protocol behavior is an explicit integration contract.

- Do not change Canary schema assumptions silently.
- Do not duplicate authentication policy across components without documenting the source of truth.
- Prefer explicit service and domain boundaries for security-critical logic.
- Keep payment functionality modular; core account/auth code must not depend on a payment provider.
- Read-only game data may use optimized read paths. Privileged state changes require explicit authorization and transactional integrity.

## Security and data invariants

Security-sensitive surfaces include authentication, sessions, MFA, password recovery, email verification, admin/RBAC, account/player mutations, API tokens, file uploads, webhooks and payments.

- Prefer framework security mechanisms over custom cryptography or session logic.
- Use modern password hashing supported by the repository stack; never store plaintext or reversibly encrypted passwords.
- Preserve CSRF protection for browser state changes.
- Validate and authorize every state-changing operation server-side and deny by default when authorization is ambiguous.
- Escape untrusted output and use parameterized queries, ORM or query builders; never concatenate untrusted SQL.
- Rate-limit authentication, recovery and abuse-prone endpoints.
- Rotate or revoke sessions when security-sensitive account state requires it.
- Use transactions and appropriate locking for balance, currency and other concurrency-sensitive mutations.
- Use idempotency for payment and webhook operations.
- Do not treat Cloudflare, a WAF or a reverse proxy as a substitute for application-layer security.

Never commit or reproduce secrets, tokens, passwords, private keys, production connection strings, cookies, personal data, database dumps or backups. Safe templates may contain placeholders. Production secrets belong in an approved secret-management system outside Git.

Treat migrations as durable production contracts. Prefer backward-compatible, reversible changes; never assume a production database is empty. Destructive changes require explicit scope, data-impact analysis and a rollback/backup strategy. Canary-owned or shared tables are cross-repository contracts.

## Laravel and PHP invariants

- Follow the Laravel/PHP versions and conventions actually present in the repository.
- Prefer framework validation, middleware, policies/gates, the service container, queues/events and database transactions over ad-hoc equivalents.
- Keep controllers thin and durable business logic in appropriately scoped services, actions or domain classes.
- Avoid static global request, user or security state.
- Reuse the framework or an existing package before adding a dependency. Inspect security and maintenance implications of new packages.
- Do not edit `vendor/**` or generated dependency directories.

## Validation routing

Discover commands from current repository files and workflows. Choose the smallest relevant focused checks while iterating, then run the repository-required exact-head checks for the final candidate. Depending on the affected surface, relevant checks may include PHP syntax/static analysis, formatter/linter, Laravel/PHPUnit/Pest, auth/security regressions, isolated migration tests, API contracts and governance validators.

Do not claim tests, model trials, CI, deployment or production state without direct evidence for the exact candidate. Structural instruction checks, provider adoption evidence and observed runtime behavior are separate evidence classes.
