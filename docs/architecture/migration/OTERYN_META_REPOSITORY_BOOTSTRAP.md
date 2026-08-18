# Oteryn META Repository Bootstrap Plan

Date: 2026-08-18
Programme: `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION`
Task: `OTERYN-20260818-meta-repository-bootstrap`
Target: `Oteryn/Oteryn`

## Decision boundary

This document prepares exactly one physical repository-creation transaction. It does not create, rename, transfer or delete a repository and does not access server/game repositories.

### FACT

- ADR 0041 defines `Oteryn` as the thin ecosystem coordination/META plane and records real workload: ecosystem topology, cross-repository ADRs, repository/release manifests, compatibility matrices and cross-repository orchestration.
- The `Oteryn` GitHub organization is reachable through GitHub App installation `154585379`.
- `Oteryn/Oteryn-Atlas` is already an organization repository reachable through the integration with administrative/write access.
- `Oteryn/Oteryn` returned `404 Not Found` at the organization-access recovery observation. This is an expiring evidence lease and must be refreshed immediately before creation.
- The current ChatGPT GitHub connector exposes no repository-create operation.
- GitHub's current web flow requires choosing the owner, repository name and visibility; for a new non-import repository it can optionally initialize a README and select applicable GitHub Apps.

### UNKNOWN

- The owner's intended visibility for `Oteryn/Oteryn` is not yet explicitly recorded.
- Whether installation `154585379` is `all repositories` or `selected repositories` for an owner-created repository is not exposed by the current connector result. Post-create access must therefore be proven rather than assumed.

### RECOMMENDATION

Use **public** visibility unless the owner has a reason to keep ecosystem governance private. The current Platform and Atlas target repositories are public, while the META bootstrap is intentionally limited to public architecture/governance/compatibility metadata and must contain no secrets. This is a recommendation only; it is not authority to select visibility.

## Minimal non-ceremonial bootstrap

After the repository exists and integration access is proven, bootstrap only the following initial authority package:

1. `README.md`
   - identify the repository as the Oteryn ecosystem coordination/META plane;
   - state that it contains no Game/Platform/Atlas runtime source;
   - link the product repositories and transition coordinates truthfully;
   - state that provider-owned schemas remain in provider repositories.

2. `AGENTS.md`
   - establish the repository-local write/safety boundary;
   - require dedicated task branches/PRs and exact-head validation;
   - forbid secrets, production/live-state mutation and silent cross-repository writes;
   - make META authority coordination-only rather than permission to mutate product repositories.

3. `docs/architecture/adr/0001-ecosystem-topology-authority.md`
   - adopt the four-repository topology;
   - explicitly state `Supersedes: blakinio/Oteryn-Platform ADR 0041` for ecosystem-topology authority;
   - preserve Game, Platform and Atlas provider ownership boundaries;
   - prohibit provider-schema duplication in META;
   - record transition sequencing rather than pretending every target coordinate is already migrated.

4. `ecosystem/repositories.json`
   - machine-readable ecosystem repository manifest;
   - distinguish `target_coordinate`, `current_coordinate`, `migration_state` and authority owner;
   - represent `Oteryn-Game` and organization transfer states truthfully as pending until separately proven;
   - never copy provider schemas or generated product artifacts.

`SECURITY.md`, `CONTRIBUTING.md`, release matrices and cross-repository workflows are later workload, not prerequisites for the first canonical authority package unless live target governance or organization policy requires them.

## Creation transaction sequence

1. Refresh `Oteryn/Oteryn` and prove it is still absent.
2. Refresh organization installation/access evidence and overlapping migration ownership.
3. Resolve target visibility explicitly.
4. Freeze the canonical transaction record and merge the Platform preparation PR so the create instructions/rollback are durable before mutation.
5. Owner performs one GitHub web creation flow:
   - Owner: `Oteryn`
   - Repository name: `Oteryn`
   - Description: `Oteryn ecosystem coordination, compatibility and release authority.`
   - Visibility: exact owner-approved value
   - Initialize with README: **yes**
   - `.gitignore`: none
   - License: none during bootstrap
   - Select the ChatGPT/OpenAI GitHub App if the creation UI offers it for this organization
   - Create repository
6. Do not repeat the create operation if the UI/session result is ambiguous. Immediately inspect `Oteryn/Oteryn` first.
7. Verify exact owner/name, visibility, archived state, default branch, repository identity and connector permissions.
8. Verify installation access. If the app cannot access the new repository, stop before bootstrap and add `Oteryn/Oteryn` to the installed app's selected repositories (or switch the installation to all repositories), then re-read access.
9. Create a dedicated bootstrap branch in `Oteryn/Oteryn`; install the minimal bootstrap package above; open a Draft PR; re-read the new `AGENTS.md`; run proportionate exact-head validation/self-review; merge only when target-local gates pass.
10. Only after META ADR 0001 is canonical may a separate Platform reconciliation mark ADR 0041 superseded for ecosystem scope.

## Fail-closed replay guard

Repository creation is non-idempotent at the namespace level.

```yaml
replay_guard:
  mutation_fingerprint: create_repository:Oteryn/Oteryn
  reissue_forbidden_until_state_proven_not_applied: true
  resume_detection:
    - GET exact repository Oteryn/Oteryn
    - confirm repository owner/name/id and creation state before any retry
```

If the user reports an error, timeout, blank page or uncertain result after clicking Create, the next action is **read the target coordinate**, not click Create again.

## Point of no return

```yaml
point_of_no_return:
  reached_when: GitHub exposes a repository object at Oteryn/Oteryn
  consequences:
    - the target namespace is occupied
    - permissions/integration state may exist
    - rollback requires a separate destructive repository-deletion operation
```

## Rollback

Creation rollback is permitted only before the new META becomes a dependency/authority for product repositories and only after proving that the target contains no unique work that must be preserved.

```yaml
rollback:
  feasibility: PROVEN_FOR_FRESH_BOOTSTRAP_ONLY
  operation: owner deletes exact repository Oteryn/Oteryn through GitHub repository settings
  trigger:
    - wrong owner/name/visibility that cannot safely be corrected inside the same transaction
    - creation result violates organization policy or cannot be brought under required integration/governance before authority handover
  decision_owner: Oteryn organization owner
  execution_window: before META ADR authority handover, external dependents, releases or unique history
  verification:
    - exact target coordinate returns 404
    - installation repository list no longer exposes the target
    - no dependent manifest/ADR has been marked canonical against the deleted repository
```

GitHub documents that organization owners or repository admins can delete an organization repository subject to organization/enterprise policy; deletion permanently removes team permissions, and some deleted repositories can be restored within 90 days. That recovery window is supporting safety evidence, not a substitute for keeping rollback bounded before any unique authority or dependents exist.

## Post-create validation

The create transaction is not complete when the UI reports success. Verify:

- exact coordinate is `Oteryn/Oteryn` and repository identity is new/expected;
- visibility equals the approved value;
- repository is not archived/disabled;
- default branch exists after README initialization;
- connector has at least pull/push access required for bootstrap;
- no unexpected template/source history was introduced;
- no secret, generated artifact or product-runtime code exists;
- README initialization is the only pre-bootstrap content;
- dedicated bootstrap branch/PR is created before non-bootstrap content;
- META ADR authority is not considered canonical until its bootstrap PR merges;
- Platform ADR 0041 is not marked superseded before that canonical META authority exists.

## Explicit exclusions

- no `Oteryn-v2` / `Oteryn-Game` inspection or cutover;
- no Platform transfer;
- no Atlas extraction or deployment;
- no Canary/otclient access;
- no package/GHCR/release mutation;
- no DNS, Synology, Cloudflare, production, auth/session, secret or live-game operation.
