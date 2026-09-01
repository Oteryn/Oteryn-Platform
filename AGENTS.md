# Oteryn Platform Agent Instructions

## AI review policy — META-owned

Platform adopts the organization-wide AI review policy from `Oteryn/Oteryn:docs/governance/AI_REVIEW_POLICY.md` by reference and does not maintain a competing local review-routing system.

- Default to no external AI review.
- Prefer Codex Spark when an ordinary code change has clear independent-review value.
- Use one Codex deep review on a stable candidate for material high-risk/control-plane changes.
- Trivial docs, formatting, generated evidence, metadata and other low-risk changes need no external AI review.
- AI review is advisory and is never a required status or merge authority; `platform-gate`, GitHub protection and Merge Queue remain the enforcement path.
- Do not recreate local R0/R1/R2 classification, review fingerprints, review envelopes, attestations, standing review controllers or equivalent merge authority.
- Never expose, copy or use owner-supplied secrets, credentials, API keys or session material merely because they are available; credential/service use still requires the authority applicable to that task.

## Bounded instruction loading

Root `AGENTS.md` is the repository entry point. Mandatory bootstrap loading is bounded to:

1. this root `AGENTS.md`;
2. `docs/agents/PLATFORM_AGENT_BOOTSTRAP.md`;
3. the nearest nested `AGENTS.md` governing each path that may be touched, when present;
4. the governing live GitHub Issue/task and live PR state for the current task, when present.

After that bootstrap, consult `docs/agents/CONTEXT_ROUTING.md` once to select only task-relevant context. `docs/agents/REPOSITORY_MAP.md`, project-state snapshots, build/test references, architecture/contracts and other documentation are task-routed or optional/reference material unless a governing instruction or the selected route explicitly requires them.

Do not recursively follow documentation links or reload a higher-priority rule merely because a lower-priority document references it. A referenced document is mandatory only when this root/bootstrap, the nearest governing `AGENTS.md`, the selected task route or an explicit safety/validation trigger requires it.

When rules conflict, follow the more restrictive safety rule.

## External execution-skill precedence

Repository and user authority govern execution. Agent skills, plugins and workflow frameworks such as Superpowers are subordinate execution aids, not independent task or lifecycle authority.

For an already-authorized Oteryn programme or task with an approved canonical design, implementation plan, checkpoint, or explicit continuation directive, Superpowers workflows MUST NOT introduce additional approval gates, re-brainstorm an approved design, require duplicate planning artifacts, replace canonical authority, or interrupt autonomous continuation solely because the skill's default workflow would do so. Relevant skills MAY still be used internally for implementation, testing, debugging, review, isolation, or verification when they do not conflict with the governing Oteryn authority.

A skill or plugin MUST NOT weaken repository safety, validation, review, GitHub-first, owner-funded AI, or authorization requirements. When a skill workflow conflicts with applicable user instructions, this `AGENTS.md`, repository policy, or canonical task authority, the applicable higher-priority Oteryn authority controls.

## GitHub-first execution gate — mandatory

GitHub is the authoritative repository control plane for repository identity, `main`, Issue/task status, PR, task branch, exact remote SHA, checks, reviews and merge state. The governing live GitHub Issue/task is the canonical authority for task lifecycle state. The live GitHub PR is authoritative for PR head/base/check/review/merge state. Repository task/context documents preserve durable context, evidence, ownership, handoff, next action and history; they are not an independent competing mutable lifecycle authority, and stale document fields never override newer live Issue/PR state.

Before any local/remote repository mutation, including work through Remote Desktop/Desktop Commander, Synology, WSL, Docker or a local worktree, the agent MUST first resolve from GitHub the exact repository, current `main` SHA, governing Issue/task (or explicit `NOT_APPLICABLE` for bounded trivial/read-only work), active PR/task branch, exact base/head SHAs and material overlapping work.

Only after that GitHub preflight may host-local tooling be used for implementation, builds, tests, containers, Playwright, artifacts or other execution. Local clones, filesystems, worktrees, containers, shell history and cached state are execution/cache planes only and MUST NOT be treated as authority or used to bypass GitHub lifecycle.

## META execution-routing policy

The canonical organization policy is [`Oteryn/Oteryn@8fac1d55805fc3372351ea0a55ad7728b3570ebc:ecosystem/agent-execution-routing-policy.json`](https://github.com/Oteryn/Oteryn/blob/8fac1d55805fc3372351ea0a55ad7728b3570ebc/ecosystem/agent-execution-routing-policy.json). Platform adopts it by reference and does not maintain a conflicting local copy.

GitHub state, GitHub Actions or an approved runner, and an isolated worktree are the default execution route. Remote Desktop/Desktop Commander is default-deny and may be used only under a recorded closed exception reason with a recorded least-privilege action. It MUST NOT be used for routine implementation, validation, Git inspection or polling when equivalent CI exists. Resume only after a fresh GitHub preflight for repository, default branch SHA, governing Issue, PR and task-head identities.

Every substantial task record must use parallel-first planning: named independent lanes, exclusive branch/worktree and owned paths, dependencies, shared-resource leases where needed, and an integration order. Serial execution requires a recorded reason.

Before editing locally, verify remote URL, branch/worktree identity, HEAD and working-tree state against the GitHub-resolved task. Preserve unrelated dirty work. After durable local changes, commit on the authorized task branch, push to GitHub, verify the remote head equals the intended commit, update the PR/task when applicable, and use exact-head GitHub CI/review state for readiness and completion.

Local-only work receives no completion credit until the durable result exists on the approved GitHub branch/PR. If GitHub is genuinely unavailable, continue safe read-only analysis/patch preparation but do not start new product mutations merely to bypass the control plane unless the owner explicitly authorizes an emergency exception.
## Repository allowlist — highest priority

- The only repository where autonomous write operations are allowed by this file is `Oteryn/Oteryn-Platform`.
- Treat `blakinio/canary`, `opentibiabr/canary`, MyAAC repositories and all other repositories as read-only unless the user explicitly authorizes a write task for that repository.
- Before every GitHub write operation, verify that `repository_full_name` is exactly `Oteryn/Oteryn-Platform`, unless the user explicitly authorized another repository in the current task.
- Do not push Oteryn Platform code into the Canary repository.
- Cross-repository compatibility work must be documented as a contract; do not silently change both sides.

## Global context efficiency baseline

- Work autonomously until the bounded task is complete or a real blocker/required decision is reached.
- Do not narrate routine file reads, searches, tool calls, commands, or unchanged checks.
- Send user-facing progress only for a material milestone, blocker, required decision, or material scope/risk change; keep each update to at most three short sentences.
- Run the full repository/task preflight once per bounded task or continuation session. Afterwards verify only state that may have changed and can invalidate the next action.
- Repeat the full preflight only after a material external repository-state change, a long interruption/session replacement, or evidence that durable task state conflicts with live state.
- Search before reading large indexes or documents in full and load only task-relevant documentation/source evidence.
- Do not paste full logs, diffs, artifacts, or whole source files when exact identifiers and focused excerpts are sufficient.
- Treat chat history as disposable. Keep durable task/handoff state compact and leave exactly one concrete next action when handing work off.
- When the next action is safe and autonomous, continue without waiting for acknowledgement.

## Lean startup protocol

After the bounded instruction loading above and before substantial implementation:

1. Resolve the governing live GitHub Issue/task, current `main`, current live PR/head when present, and material overlapping work.
2. Consult `docs/agents/CONTEXT_ROUTING.md` once, classify the task, and load only the matching task-routed context.
3. If continuing existing work, read its active task `## Context checkpoint` as durable context/evidence and reconcile stale lifecycle or PR fields against live GitHub before acting.
4. Search active tasks, Issues and open PRs narrowly for overlapping paths, modules, identifiers or contracts.
5. Search the repository for reusable code before designing a new reusable abstraction.
6. When a local checkout exists, verify Git branch, working tree, remotes and worktrees before editing.
7. Record uncertainty instead of inventing repository, deployment, database or cross-repository state.

Do not recursively load unrelated documentation or preload optional/reference material.

## Durable context and continuation — mandatory

- Chat history is disposable and never authoritative project state.
- The governing live GitHub Issue/task is canonical for task lifecycle state, and the live GitHub PR is authoritative for PR head/base/check/review/merge state.
- Repository task/context records and deterministic evidence are durable context and proof, not an independent competing mutable lifecycle authority; reconcile stale lifecycle or PR fields to newer live GitHub state.
- Follow `docs/agents/CONTEXT_HANDOFF.md` when context grows materially, work is interrupted or another agent must continue.
- Maintain a compact `## Context checkpoint` in every substantial active task.
- Update the checkpoint after material discoveries, patches, validation changes, review changes, head changes, blockers and before session exhaustion.
- Use evidence states consistently: `PROVEN`, `DERIVED`, `UNKNOWN`, `CONFLICT`.
- Never convert `UNKNOWN` into an assumption.
- Leave exactly one concrete `next_action` in a handoff.

## Work visibility and task records — mandatory

For substantial work:

- create `docs/agents/tasks/active/OTERYN-YYYYMMDD-short-slug.md` from `docs/agents/tasks/TASK_TEMPLATE.md`;
- declare `owned_paths`, modules, dependencies, blockers and cross-repository dependencies;
- use one task branch per substantial task;
- open a draft PR early when GitHub PR workflow is available;
- treat the governing live GitHub Issue/task as canonical lifecycle authority and the live PR as PR-state authority; keep the task record as a durable context/evidence/ownership/handoff mirror that is reconciled when stale;
- move completed task records to `docs/agents/tasks/archive/` after merge/completion;
- create an ADR under `docs/architecture/adr/` for architectural decisions expected to outlive one task;
- document public integration contracts under `docs/contracts/`.

Before creating a service, repository abstraction, API client, auth provider, policy layer, payment abstraction, queue job family or reusable UI component, search for an existing implementation first. Prefer reuse or extension. If reuse is rejected, record the concrete reason.

## Multi-agent concurrency

- One agent uses one task branch/worktree.
- Never share a branch or worktree between active agents.
- `owned_paths` are advisory locks; resolve overlaps before editing.
- Keep `admission_main_sha`, `task_head_sha`, and `integration_main_sha` distinct for substantial mutating work: admission is immutable provenance, task head is the current branch head, and integration main is selected only for final reconciliation.
- If protected `main` advances after admission, classify it as `UPSTREAM_ADVANCED`; that movement alone does not invalidate work and is not a reason to restart, reset, recreate, rebase, force-push, or discard still-applicable implementation.
- If the upstream delta changes an applicable instruction, safety/security/provenance rule, architecture authority, compatibility contract, or invariant, reload and reconcile that governing authority before further mutation while preserving unaffected work.
- Preserve published task history by default. At final integration, refresh to current `integration_main_sha` with a normal non-force merge-up, resolve only authorized conflicts, review the resulting diff, and rerun every validation/review layer invalidated by the resulting `task_head_sha`.
- A lost merge race returns the task to integration/reconciliation, not to implementation from scratch.
- Invalidate affected work only after verified task cancellation/supersession/rescope, incompatible governing authority, semantic contract/API/schema/invariant conflict, failed authorized reconciliation, or required tests prove prior assumptions no longer hold; textual overlap or a changed filename alone is not sufficient proof.
- Do not perform unrelated cleanup or broad refactors.
- Do not edit another active task record except to resolve an explicitly coordinated ownership conflict.
- Shared indexes and architecture documents must be edited narrowly.

## Execution resource hygiene — mandatory

- Read and follow `docs/agents/EXECUTION_RESOURCE_HYGIENE.md` whenever a task creates, starts, reuses, or controls temporary containers, Compose projects, networks, disposable volumes, images, helper services, runners, test deployments, or equivalent execution scaffolding.
- A task-owned temporary resource must have a deterministic identity and lifecycle owner before creation. Record enough non-secret identity to find and remove it safely later.
- Remove task-owned ephemeral resources as soon as they are no longer needed; do not leave cleanup until a later agent or an unspecified future closeout.
- At terminal closeout, verify that no unintended task-owned temporary resource remains. A task is not `DONE` while such a resource remains unintentionally and cleanup is still executable.
- Cleanup must be exact and ownership-scoped. Blanket Docker cleanup such as system/container/volume/network/image prune is forbidden on shared hosts unless the repository owner explicitly authorizes that exact destructive scope.
- Never remove unrelated workloads, canonical shared services, runner infrastructure, persistent/named volumes, durable data, shared networks, or production resources merely because they are stopped, old, unused by the current command, or not recognized by the agent.
- Persistent or shared resource deletion requires explicit scope plus proven ownership/disposability; named volumes are persistent by default.
- If cleanup cannot be completed because access, permissions, connectivity, environment protection, or resource ownership is unresolved, record the exact remaining resources and blocker/next action instead of silently declaring cleanup complete.

## Delivery workflow

Default workflow:

1. inspect current repository/task/PR state;
2. claim the task and affected paths;
3. create or update the task record;
4. create a dedicated task branch;
5. implement the smallest complete change;
6. run relevant validation;
7. create or update the PR;
8. inspect CI results and logs;
9. fix root causes and repeat until required checks pass;
10. update checkpoint/docs/contracts as required;
11. merge only when the merge gate is satisfied.

Never push feature/fix work directly to `main` after the repository bootstrap phase.

## Merge gate

Merge only when all are true:

- base/head repositories are the approved user-owned repository;
- base is `main` and head is a dedicated task branch;
- changed files contain no unrelated or forbidden changes;
- acceptance criteria are satisfied;
- relevant local validation ran, or unavailable environment is documented exactly;
- required GitHub checks pass on the current head;
- no unresolved blocker, requested change, ownership conflict or migration hold remains;
- security-critical changes have appropriate regression tests;
- task record and relevant contracts/architecture docs are current.

Use squash merge unless repository policy requires another method. Never bypass branch protection, weaken tests, remove safety checks or mark failures successful.

## CI repair loop

When CI fails:

1. identify workflow, job, step, current commit SHA and exact error;
2. inspect logs/artifacts before rerunning;
3. classify the cause as task code, stale base, CI configuration or external infrastructure;
4. fix the root cause in the same PR when it belongs to the task;
5. use a separate narrow task when an unrelated CI repair would obscure the change;
6. rerun failed jobs only when appropriate;
7. record failure and fix in the task checkpoint.

A repeated identical failure must be investigated. Do not silence or loosen a check merely to obtain green CI.

## Mandatory stop conditions

Stop automatic merge and document the blocker for:

- secrets, credentials, private keys, database dumps, backups or personal data;
- destructive production migration without a tested rollback path;
- production deployment or irreversible external action outside the repository PR;
- unresolved overlapping path ownership;
- an atomic cross-repository contract where both sides are not ready;
- changes that would silently break Canary/login-server compatibility;
- payment processing changes without explicit task scope and security review;
- authentication/session changes whose compatibility or revocation behavior remains `UNKNOWN`.

## Architecture boundaries

Oteryn Platform is the web/application platform. Canary is the game server.

Default responsibility split:

- Oteryn Platform: web UI, CMS, accounts, authentication, authorization, admin, API and future payment/business modules.
- Canary: game runtime and game-server behavior.
- Shared database or protocol behavior is an explicit integration contract, not an implicit assumption.

Rules:

- Do not change Canary schema assumptions silently.
- Do not duplicate authentication policy across multiple components without documenting the source of truth.
- Prefer explicit service/domain boundaries for security-critical logic.
- Keep future payment functionality modular; core account/auth code must not depend on a payment provider.
- Read-only game data such as highscores, characters and guilds may use optimized read paths, but privileged state changes require explicit authorization and transactional integrity.

## Security policy — mandatory

Security-sensitive surfaces include authentication, sessions, MFA, password recovery, email verification, admin/RBAC, account/player mutations, API tokens, file uploads, webhooks and future payments.

For these surfaces:

- use framework-provided security mechanisms before custom cryptography or custom session logic;
- use modern password hashing supported by the selected Laravel/PHP stack; never introduce plaintext or reversible password storage;
- preserve CSRF protection for browser state-changing requests;
- validate and authorize every state-changing operation server-side;
- escape untrusted output by default;
- use parameterized queries/ORM/query builder; never concatenate untrusted SQL;
- apply rate limiting to authentication, recovery and abuse-prone endpoints;
- require explicit authorization policies for privileged operations;
- deny by default when authorization state is ambiguous;
- rotate/revoke sessions when security-sensitive account state requires it;
- require transactions and appropriate locking for balance/currency or other concurrency-sensitive mutations;
- use idempotency for future payment/webhook operations;
- add regression tests for every fixed security vulnerability when practical.

Do not claim an endpoint is secure merely because Cloudflare, a WAF or a reverse proxy is present. Application-layer security remains mandatory.

## Secrets and sensitive data

- Never commit `.env`, tokens, passwords, private keys, production connection strings, cookies, personal data, database dumps or backups.
- Commit only safe templates such as `.env.example` with placeholders.
- If sensitive data is discovered, stop and report it without reproducing the secret.
- Do not put secrets in task records, PR bodies, comments, logs, screenshots or test fixtures.
- Production secrets belong in an approved secret-management/deployment system outside Git.

## Database and migration safety

- Treat migrations as durable production contracts.
- Prefer backward-compatible, reversible migrations.
- Never assume a production database is empty.
- Destructive operations require explicit task scope, data-impact analysis and rollback/backup strategy.
- Account, session and currency mutations must preserve transactional integrity.
- Concurrency-sensitive operations require deterministic tests where practical.
- Canary-owned or shared tables must be treated as cross-repository contracts and documented before incompatible schema changes.

## Laravel / PHP implementation policy

- Follow the Laravel version and conventions actually present in the repository; do not assume a version before `composer.json` exists.
- Prefer framework validation, middleware, policies/gates, service container, queues/events and database transactions over ad-hoc equivalents.
- Keep controllers thin; place durable business logic in appropriately scoped services/actions/domain classes.
- Avoid static global state for request/user/security context.
- Do not add a dependency when the framework or an existing package already provides the needed capability.
- Pin and update dependencies deliberately; inspect security and maintenance implications of new packages.
- Do not edit `vendor/**` or generated dependency directories.

## Validation policy

Before readiness, inspect the full changed-file list and diff.

Run the smallest relevant validation supported by the repository. Depending on installed tooling this may include:

- PHP syntax/static analysis;
- Laravel/PHPUnit/Pest unit and feature tests;
- focused auth/security regression tests;
- migration tests against an isolated test database;
- formatter/linter checks;
- API contract tests;
- CI workflows.

Discover actual project commands from `composer.json`, repository docs and workflows before running them. Do not invent successful test results and do not claim CI passed unless verified on the current head.

## Cross-repository Canary/login-server work

When a task depends on Canary or login-server behavior:

- treat external repositories as read-only unless separately authorized;
- verify current source/schema rather than relying on memory;
- record the contract under `docs/contracts/` when durable;
- identify whether rollout order matters;
- label facts as `PROVEN`, `DERIVED`, `UNKNOWN` or `CONFLICT`;
- stop merge when an atomic compatibility requirement is unresolved.

## Git safety

- Never assume a local checkout is synchronized with GitHub.
- Before editing from a local checkout, inspect branch, working tree, remotes and worktrees.
- Do not automatically discard, clean, reset or overwrite uncommitted work.
- Use one dedicated branch per task.
- Never push task commits directly to `main`.
- Prefer explicit push targets.
- Use `--force-with-lease` only when history rewrite is justified; never plain `--force`.
- Use Conventional Commit style: `<type>(optional-scope): <summary>`.

Preferred types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`, `revert`.
## Current bootstrap note

The repository may initially contain only governance/documentation scaffolding. Agents must inspect actual repository state before assuming Laravel, Composer, Node or database tooling has already been initialized.

The first application bootstrap task must establish and document the chosen Laravel/PHP versions, local development workflow, test database strategy and baseline CI before feature work expands broadly.

## GitHub connector routing — mandatory

- For GitHub repository, pull request, issue, review, and remote-file tasks, inspect and use the connected GitHub plugin or connector before falling back to local `git` or `gh`.
- Treat an explicit `@GitHub` selection as a request to use the connected GitHub plugin.
- Local `git` may be used for checkout, worktree, diff, branch, and commit operations. Use `gh` only for operations the connector does not support or when repository policy explicitly requires it.
- A missing local checkout, missing `gh` binary, or unauthenticated local `gh` session is not evidence that the GitHub connector is unavailable.

Before claiming that GitHub access is unavailable:

1. Inspect the available GitHub connector tools and determine whether the connector is registered and enabled and whether the required operations exist.
2. If an authenticated-identity operation exists and the connector is callable, call `github_get_user_login` or its equivalent; otherwise record the confirmed missing or disabled connector or missing identity operation.
3. If a repository lookup or listing operation exists and the connector is callable, call `github_get_repo` or `github_list_repositories` for the requested repository scope; otherwise record the missing capability.
4. If the required read operation exists and is callable, attempt it through the connector when it is safe and within the task's authority; otherwise record the unavailable capability.

Report a GitHub access blocker only after the applicable availability and capability checks above and, when an applicable operation exists and is safe to attempt, an actual connector call. Authentication or permission errors, a confirmed missing or disabled connector, a missing required operation, rate limiting, and transport or service failures are valid blockers when they prevent the task and no safe permitted connector, local `git`, or `gh` fallback can complete it. Include the exact availability and capability verification performed. When a call was attempted, include the failed operation and returned error; when no call was possible, identify the missing or disabled connector or unavailable operation instead.