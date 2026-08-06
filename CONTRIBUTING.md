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

## Licensing and contribution rights

Oteryn Platform is proprietary under `LICENSE.md`. Public visibility and repository collaboration do not grant permission to use, modify, redistribute, sublicense, host or commercially exploit the project. Third-party material remains governed by its own terms as described in `THIRD_PARTY_NOTICES.md`.

External contributions are not accepted by default. Before submitting source code, documentation, designs, assets, fixtures, data or other material, obtain a written invitation from the repository owner and written contribution terms covering the exact contribution.

An invited contributor must be able to prove that they created the contribution or otherwise hold sufficient rights to provide it. Do not submit copied code, assets, game data, proprietary documentation, confidential information or material whose provenance or redistribution rights are uncertain.

Opening an Issue, discussion or pull request does not guarantee acceptance and does not change the repository's licensing policy. A contribution may merge only after the agreed written terms establish rights sufficient for the repository owner to use, modify and distribute it under the repository's then-current policy. The project does not infer an assignment, license or waiver from silence or informal review.

No maintainer or agent may publish broader redistribution terms, accept incompatible inbound material or copy a license from another repository without a separate owner-approved decision and provenance review.

## Conduct

Participation is governed by `CODE_OF_CONDUCT.md`.
