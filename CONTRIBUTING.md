# Contributing to Oteryn Platform

## Before starting

Read `AGENTS.md` and every nearer `AGENTS.md` governing the paths you intend to change. Search open Issues, pull requests and active task records for overlapping ownership before creating a branch.

For substantial work, create or claim one task record under `docs/agents/tasks/active/`, declare owned paths and use one dedicated branch. Do not push feature or repair work directly to `main`.

## Branches and commits

Use a short, descriptive branch prefix such as `feat/`, `fix/`, `docs/`, `test/`, `ci/`, `chore/`, `audit/` or `repair/`.

Commit messages and pull-request titles use Conventional Commit form:

```text
<type>(optional-scope): <imperative summary>
```

Preferred types are `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore` and `revert`.

Keep commits coherent. Do not mix unrelated cleanup, formatting or refactoring into a bounded change.

## Pull requests

Open a draft pull request early for substantial work. Complete the repository pull-request template and include:

- the goal and bounded scope;
- linked Issue or task record;
- changed layers and excluded areas;
- observable acceptance criteria;
- validation performed on the exact head;
- migration, rollout and rollback notes when applicable;
- security, compatibility and production impact;
- related or superseded pull requests.

The default merge method is squash. A pull request is not ready merely because CI is green: required review discussions, ownership, compatibility, audit, E2E and lifecycle closeout must also be satisfied.

## Validation

Run the smallest relevant checks during implementation and the complete applicable validation on the exact final head. Discover commands from current repository manifests and workflows; do not invent successful results.

Changes to authentication, authorization, sessions, payments, currency, migrations, contracts, deployment or other security-sensitive surfaces require focused regression coverage and an explicit risk review.

User-facing work is incomplete until applicable backend, frontend or client, persistence, integration, failure/recovery states and real E2E behavior work together.

## Security and sensitive data

Follow `SECURITY.md`. Never commit credentials, tokens, private keys, `.env` files, production connection strings, database dumps, backups, cookies, personal data or sensitive evidence. Use safe placeholders in examples.

## Dependencies and generated files

Prefer existing framework and repository capabilities. Explain new dependencies and assess maintenance and security impact. Do not edit vendored or generated dependency directories directly.

## Licensing

The project metadata in `composer.json` declares the package `proprietary`. Public repository visibility does not make the project open source and does not grant permission to use, modify, redistribute or sublicense the code. No external redistribution terms may be claimed unless the repository owner publishes an explicit written license or notice.

## Conduct

Participation is governed by `CODE_OF_CONDUCT.md`.
