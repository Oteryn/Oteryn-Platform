---
task_id: OTERYN-20260724-download-center
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/PROJECT_STATE.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/contracts/PUBLIC_PORTAL_EXTENSION_CONTRACT.md
  - docs/operations/PRODUCTION_READINESS_CHECKLIST.md
search_first:
  - current CMS/Admin/RBAC/Audit conventions
  - current release and deployment artifact documentation
  - active tasks and open PRs for Downloads path overlap
optional_reads: []
---

# OTERYN-20260724-download-center

## Goal

Implement an isolated production-capable Download Center that publishes approved immutable client artifact references without exposing executable uploads or arbitrary URL proxying.

## Acceptance criteria

- [x] Platform-owned `client_releases` and `client_release_artifacts` persistence supports stable/beta channels, OS/architecture variants, publication and per-channel current state.
- [x] Public `/download` identifies current approved builds and shows version, platform, filename, size and SHA-256.
- [x] Optional platform filtering, empty state and dependency-unavailable state are explicit.
- [x] Draft and non-current unpublished releases are not public.
- [x] Administrator list/create/update/publish workflow requires `auth`, `mfa.confirmed` and exact `downloads.manage`.
- [x] No executable upload input or Platform URL proxy exists.
- [x] Artifact references reject unapproved schemes/hosts including `javascript:` and `data:`.
- [x] Administrator mutations are CSRF-protected, validated and recorded with bounded non-secret audit metadata.
- [x] No shared route, layout, homepage, navigation, central permission registry, Events, Wiki, Support or PublicGameData paths are modified.
- [x] Formatting, PHPStan, focused tests, full tests and required CI pass on the exact implementation head.

## Ownership

```yaml
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - config/downloads.php
  - database/migrations/*client_release*
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Feature/Downloads/**
  - tests/Unit/Downloads/**
  - docs/agents/tasks/active/OTERYN-20260724-download-center.md
modules:
  - Downloads
dependencies:
  - merged public-web parallel foundation / PR #146
  - reserved exact permission downloads.manage
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-07-25T08:15:51Z
head: e0041f609b0ec156598ce1ecd9ef7713aa534d74
branch: feat/OTERYN-20260724-download-center
pr: 161
status: complete
context_routes:
  - agent-governance
  - architecture
  - web-cms
  - admin-rbac
  - database
  - security
  - testing
owned_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - config/downloads.php
  - database/migrations/*client_release*
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Feature/Downloads/**
  - tests/Unit/Downloads/**
  - docs/agents/tasks/active/OTERYN-20260724-download-center.md
proven:
  - PR #146 provides module-local route loading and reserves the exact downloads.manage permission without an automatic shared-role grant.
  - client release and artifact data is Platform-owned and does not require Canary or login-server writes.
  - the public endpoint exposes direct approved HTTPS references only; no executable upload or artifact proxy endpoint exists.
  - artifact hosts are exact-match allowlisted and javascript, data, HTTP, user-info, fragment, non-standard port and host-root references fail closed.
  - publication locks the release channel, revalidates enabled artifacts, updates current state atomically and records bounded audit metadata.
  - checksums are displayed as supplied release metadata and are not represented as independently verified by Platform.
  - the final implementation changed only Downloads-owned paths and did not change routes/web.php, shared layouts, homepage, navigation or the central permission registry.
derived:
  - operators must configure DOWNLOADS_ALLOWED_ARTIFACT_HOSTS before an artifact can be approved for publication.
unknown: []
conflicts: []
first_failure:
  marker: resolved
  evidence: PHPStan redundant publication guards were replaced with explicit nested state guards; an out-of-scope navigation contribution was removed after existing integration tests identified it as the only route-activation regression.
rejected_hypotheses:
  - editing routes/web.php or shared public/admin layouts is unnecessary and prohibited.
  - adding a public navigation contribution is outside the declared ownership and is not required for the /download endpoint.
  - granting downloads.manage to an existing shared role bundle is outside this isolated task.
changed_paths:
  - app/Downloads/**
  - app/Http/Controllers/Downloads/**
  - app/Http/Requests/Downloads/**
  - config/downloads.php
  - database/migrations/2026_07_24_211000_create_client_release_tables.php
  - resources/views/downloads/**
  - resources/views/admin/downloads/**
  - routes/modules/downloads.php
  - tests/Feature/Downloads/**
  - tests/Unit/Downloads/**
  - docs/agents/tasks/active/OTERYN-20260724-download-center.md
validation:
  - command: vendor/bin/pint --test
    result: PASS
    evidence: CI run 1960, head e0041f609b0ec156598ce1ecd9ef7713aa534d74
  - command: composer analyse
    result: PASS
    evidence: CI run 1960, head e0041f609b0ec156598ce1ecd9ef7713aa534d74
  - command: composer test
    result: PASS
    evidence: CI run 1960, head e0041f609b0ec156598ce1ecd9ef7713aa534d74
  - command: required GitHub workflows
    result: PASS
    evidence: CI, Agent Governance, Platform DB Outage Validation, Phase 7 Production-Like Validation, Game Auth Ticket Concurrency, Build Synology Staging Images and Acceptance E2E and Visual UX succeeded on head e0041f609b0ec156598ce1ecd9ef7713aa534d74
blockers:
  - none
next_action: Review and merge PR #161.
```

## Notes

The module stores operator-supplied artifact metadata and approved immutable HTTPS references only. It does not fetch artifacts, proxy URLs, upload executables or claim checksum verification.
