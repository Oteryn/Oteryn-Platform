# Oteryn Platform Agent Instructions

## Organization policy binding

Resolve `docs/agents/META_AGENT_POLICY_BINDING.json` before substantial planning, mutation, prompt authoring, validation or integration. The binding selects an immutable reviewed Oteryn META policy; it does not grant repository, production, credential or merge authority and it does not imply that a client automatically loads remote instructions.

Keep the local bootstrap below available for ordinary Platform work. Load a bound META source only when its subject applies, using the exact bound commit. If applicable authority cannot be established, fail closed for that operation while continuing safe independent work.

## Bounded instruction loading

Load only:

1. this file;
2. `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`;
3. the nearest nested `AGENTS.md` for each path that may be touched;
4. the governing live GitHub Issue/task and live PR when present;
5. one route selected from `docs/agents/CONTEXT_ROUTING.md`.

Do not recursively follow references. Expand context only when the selected route, a governing instruction, or a safety or validation trigger requires it.

## Repository and authority boundary

- The only repository where autonomous write operations are allowed by this file is `Oteryn/Oteryn-Platform`.
- Before every GitHub write operation, verify that `repository_full_name` is exactly `Oteryn/Oteryn-Platform`, unless the user explicitly authorized another repository in the current task.
- Treat all other repositories as read-only unless the user explicitly authorizes that exact repository and task. Do not push Platform code into a game-server repository.
- GitHub live state is authoritative for repository, default branch, Issue, PR, review, check and merge facts. Refresh a fact before a material decision when it may have changed.
- Use one dedicated task branch and workspace per active writer. Preserve unrelated changes and resolve path overlap before editing.
- Never push task work directly to `main`, force-push, bypass protection, weaken a gate, or mark a failed check successful.

For substantial work, create an active packet from `docs/agents/tasks/TASK_TEMPLATE.md`. The live Issue and PR govern lifecycle; the packet stores ownership, evidence and recovery context. Use the repository's existing task, validation and closeout tools rather than inventing a parallel lifecycle.

## Product boundary

Oteryn Platform owns the web application, CMS, accounts, authentication, authorization, administration, APIs and future payment/business modules. Game-server runtime, gameplay protocol and server persistence remain outside this repository.

- Treat shared database, login and protocol behavior as explicit cross-repository contracts.
- Do not change game-server schema assumptions silently.
- Prefer read-only paths for public game data. Privileged state changes require explicit authorization and transactional integrity.
- Keep payment functionality modular. Core account/auth code must not depend on a payment provider.

## Security and data invariants

Authentication, sessions, MFA, recovery, verification, admin/RBAC, account/player mutation, API tokens, uploads, webhooks and payments are security-sensitive.

- Use framework security mechanisms and modern one-way password hashing.
- Preserve CSRF protection for browser mutations; validate and authorize every mutation server-side.
- Escape untrusted output and use ORM, query-builder or parameterized SQL.
- Rate-limit authentication, recovery and abuse-prone endpoints; deny authorization when state is ambiguous.
- Rotate or revoke sessions when sensitive account state requires it.
- Use transactions and appropriate locking for currency and concurrency-sensitive mutations.
- Require idempotency and replay protection for payment/webhook effects.
- Add a regression test for a fixed security defect when practical.

Treat migrations as durable production contracts. Prefer backward-compatible reversible changes; never assume production data is empty. Destructive migration, live data, production deployment, payments, authentication/session mutation, secrets and protected-environment operations require the task-specific authority and evidence applicable to that operation.

Never commit or reproduce `.env` contents, tokens, passwords, private keys, production connection strings, cookies, personal data, dumps or backups. Commit only safe templates with placeholders.

## Laravel and repository implementation

- Follow the Laravel/PHP versions and commands present in this repository.
- Prefer framework validation, middleware, policies, the service container, queues/events and database transactions over ad hoc equivalents.
- Keep controllers thin and durable business rules in the existing owning service/domain boundary.
- Avoid request or security context in static global state.
- Reuse existing packages and abstractions before adding dependencies or new reusable layers.
- Do not edit generated dependency directories.

Before readiness, inspect the complete changed-file list and diff, then run the smallest relevant focused checks plus every repository-required exact-candidate check. Product-facing work must prove the applicable real actor-to-result path. Governance-only work may record product runtime E2E as `NOT_APPLICABLE` with a concrete reason.

Use Conventional Commit style: `<type>(optional-scope): <summary>`.
