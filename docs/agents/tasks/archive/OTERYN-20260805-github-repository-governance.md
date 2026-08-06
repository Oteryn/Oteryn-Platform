---
task_id: OTERYN-20260805-github-repository-governance
status: completed
pull_request: 564
completed_at: 2026-08-05T19:31:00+02:00
---

# OTERYN-20260805-github-repository-governance

## Goal

Codify repository-side GitHub governance for `blakinio/Oteryn-Platform` without overlapping active workflow ownership.

## Delivered

- repository-wide and sensitive-path CODEOWNERS;
- security, contribution and conduct policies;
- machine-readable merge, branch-protection, security-feature and environment policy;
- standard-library Python verifier/applicator;
- deterministic unit coverage integrated with the existing CI classifier test entry point;
- preserved proprietary licensing declaration;
- no workflow files or production runtime paths changed.

## Validation

The original exact head `860e7647ec84f5c29a866a26ede62e36a3462611` passed CI, Agent Governance, Deep System Validation, Phase 7 Production-Like Validation, Platform DB Outage Validation, Edge Security Emulation and Game Auth Ticket Concurrency.

The branch was then rebuilt on current `main@029a11c3affb264fc126df135bafaac938f4461b` using the same approved file blobs to remove the stale-base merge conflict. Final exact-head checks are required before squash merge.

## Remaining live administration

Repository documents and tooling do not themselves activate GitHub administration controls. After merge, apply the policy using an administration-capable token and verify zero drift:

```bash
GITHUB_TOKEN=... python3 scripts/github/repository_policy.py \
  --policy docs/operations/github-repository-policy.json \
  --apply
```

The connector used for this task does not expose repository-settings, branch-protection, environment-protection or security-feature mutation endpoints, so no live administrative control is claimed as applied by this task.
