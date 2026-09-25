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
5. a route from `docs/agents/CONTEXT_ROUTING.md` only when the task domain is not already clear or a concrete trigger requires specialist context.

Do not recursively follow references. Expand context only when the selected route, a governing instruction, or a safety or validation trigger requires it.

## Repository and authority boundary

- The only repository where autonomous write operations are allowed by this file is `Oteryn/Oteryn-Platform`.
- Before every GitHub write operation, verify that `repository_full_name` is exactly `Oteryn/Oteryn-Platform`, unless the user explicitly authorized another repository in the current task.
- Do not read, search, fetch, inspect, review or change server/game repositories unless the owner explicitly authorizes that exact repository and task. When that authorization covers inspection only, the external repository remains read-only. Do not push Platform code into a game-server repository.
- GitHub live state is authoritative for repository, default branch, Issue, PR, review, check and merge facts. Refresh a fact before a material decision when it may have changed.
- Use one dedicated task branch and workspace per active writer. Preserve unrelated changes; before editing shared paths, use targeted live Issue/PR and active-task checks to find current owners, then reuse the authorized task or resolve material overlap.
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

## Jira programme coordination

Oteryn programme coordination is mirrored in Jira project `KAN` at `https://oteryn.atlassian.net`; `KAN-23` is the programme overview. When an Atlassian/Jira connector is available in the current session, use it as a bounded programme-coordination surface.

- After the normal GitHub preflight for a substantial start or resume, resolve an **existing mapped Jira Story** for the current workstream. Prefer a native GitHub source link on the Jira item; otherwise require an exact repository/workstream label match. Do not map work by a similar title alone.
- Read only the mapped Story, its parent Epic, priority, status, fixVersion/milestone and readiness labels needed for the current decision. Do not bulk-load unrelated Jira history.
- **GitHub remains repository lifecycle and technical source of truth** for repository identity, Issues/tasks, branches, PRs, exact SHAs, checks, review, Merge Queue and integration. Repository contracts/task records remain implementation authority. Jira is the programme roadmap/readiness/milestone view and never grants repository, merge, production, secret or cross-repository mutation authority.
- Ordinary repository workers may update only their already-mapped programme Story after a verified material state transition. Broad Jira restructuring, new programme Epics/Versions, cross-workstream reprioritization and edits to `KAN-23` belong to the programme coordinator unless the owner explicitly delegates them.
- Use the established programme state convention: queued/blocked/stalled work stays `Do zrobienia` with the matching `readiness-queued`, `readiness-blocked` or `readiness-stalled` label; active work is `W toku` with `readiness-active`; completed work is `Gotowe` with `readiness-complete` only after the Story's full acceptance is verified. Use `W trakcie weryfikacji` when implementation is complete but required review/qualification is still pending.
- Fresh-read both Jira and the linked GitHub state before a Jira mutation. Do not spam comments or rewrite unchanged fields. A closed individual GitHub Issue/PR does not make an aggregate Jira Story complete while another linked acceptance source remains open.
- If the Jira connector is unavailable, the mapping is absent, or Jira write capability is unavailable, continue otherwise-authorized repository work. Record Jira synchronization as pending/unknown rather than inventing a mapping, creating duplicate programme items, or treating Jira availability as an implementation blocker.
