# GitHub Repository Governance

## Purpose

This document defines the desired GitHub-side controls for `Oteryn/Oteryn-Platform`. The machine-readable source is `docs/operations/github-repository-policy.json`.

Repository documents describe policy; GitHub settings enforce it. A control is not considered active until live API state verifies it.

## Current verified gaps at task start

Baseline: `main@968c8adc912beef0119da21a345b0afadc45a494` on 2026-08-05.

- `main` was not protected and no repository ruleset existed.
- squash, merge-commit and rebase merge were all enabled.
- automatic branch deletion, update-branch suggestions and auto-merge were disabled.
- Dependabot alerts were disabled.
- `production` had no protection rules or branch restriction.
- `production-cloudflare` and `synology-staging` had branch policies but no required reviewer.
- no CODEOWNERS, `SECURITY.md`, `CONTRIBUTING.md`, GitHub-detected license file or code of conduct existed.
- `composer.json` already declared the project package `proprietary`.

## Required repository settings

The repository uses squash-only history:

- squash merge enabled;
- merge commits disabled;
- rebase merge disabled;
- PR title used as the squash title;
- merged branches deleted automatically;
- update-branch suggestions enabled;
- auto-merge enabled, but never treated as authority to bypass required checks.

Pull-request titles and final squash commits use Conventional Commit form.

## `main` protection

`main` must require a pull request, exact-head status checks and resolved review conversations. Force-push and deletion are forbidden; linear history and administrator enforcement are enabled.

Initial required contexts are:

- `classify-changes`;
- `test`.

These are always emitted by the current CI workflow, including a skipped terminal result when the change classifier determines the application suite is not applicable. Conditional workflows must not be globally required because a check that is never emitted can deadlock pull requests.

After active workflow PR #542 and Agent Governance remediation Issue #558 are terminal, create one always-emitted aggregate required check and replace the individual contexts with that stable gate.

## Review policy for the current ownership model

The repository currently has one trusted maintainer. Requiring one approval or a code-owner approval would prevent the author from merging their own pull request because GitHub does not permit self-approval.

Therefore the initial enforced policy requires a pull request, checks and resolved conversations but uses zero mandatory approvals and does not require CODEOWNER approval. CODEOWNERS still routes responsibility and becomes enforceable after a second trusted maintainer exists.

When a second trusted maintainer is available, change the policy to:

- one required approval;
- required CODEOWNER review;
- dismiss stale approvals;
- require approval of the most recent reviewable push by someone other than its author.

## Environments

### `production` and `production-cloudflare`

- deployment only from protected branches;
- five-minute wait timer;
- explicit approval by the repository owner under the current single-maintainer model;
- administrator bypass expected to be disabled;
- production secrets remain environment-scoped and never appear in Git, logs or artifacts.

### `synology-staging`

- deployment only from protected branches;
- no required reviewer while it remains a staging environment;
- administrator bypass expected to be disabled;
- staging secrets remain separate from production secrets.

The REST environment update endpoint can apply wait timers, reviewers and branch policy. The repository policy verifier reports administrator-bypass drift separately because support for mutating that flag varies by GitHub API surface and plan; it must never claim success without live verification.

## Security controls

Enable and verify:

- dependency graph;
- Dependabot alerts;
- Dependabot security updates;
- private vulnerability reporting;
- secret scanning;
- secret-scanning push protection.

The reporting process is defined in `SECURITY.md`.

## Workflow hardening backlog

Workflow changes are serialized while PR #542 owns workflow paths and Issue #558 owns Agent Governance remediation. After both become terminal, perform a separate workflow-hardening task to:

1. add the always-emitted aggregate required check;
2. pin third-party actions to immutable full commit SHAs with version comments;
3. add deterministic workflow linting and unsafe-pattern checks;
4. verify least-privilege `GITHUB_TOKEN` permissions;
5. require the aggregate check in `main` protection.

Do not combine that work with this non-overlapping governance package.

## License status

`composer.json` declares the package `proprietary`; this task preserves that status and does not introduce an open-source license. The repository still has no GitHub-detected `LICENSE` file. Public visibility does not grant permission to use, modify, redistribute or sublicense the code.

The owner may later add an explicit proprietary `LICENSE` or `NOTICE` file, or deliberately replace the proprietary declaration with a reviewed open-source license after checking upstream and dependency obligations. That is a separate legal/product decision.

## Verification and application

The standard-library Python policy tool is intentionally safe by default:

```bash
python3 scripts/github/repository_policy.py \
  --policy docs/operations/github-repository-policy.json
```

It reads public state without mutation. For authenticated verification, set `GITHUB_TOKEN`.

Applying supported controls requires an administration-capable fine-grained token and an explicit flag:

```bash
GITHUB_TOKEN=... python3 scripts/github/repository_policy.py \
  --policy docs/operations/github-repository-policy.json \
  --apply
```

The tool exits non-zero when drift or an unsupported/unverified control remains. Never store the token in shell history, repository files, task records, Issues, pull requests or workflow logs.

Pure policy behavior is covered by `tests/ci/test_repository_policy.py`, which is imported by the existing change-classifier test entry point and therefore runs in the normal CI classification gate.

## Completion rule

This repository-side package is complete when its files and tests pass. The live GitHub governance rollout remains incomplete until an administration-capable actor applies the policy and a second verification run reports zero drift. Repository merge authority does not authorize production deployment or protected-environment approval.
