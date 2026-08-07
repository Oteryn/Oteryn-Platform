---
task_id: OTERYN-20260807-native-oauth-revocation-postrepair-audit
project_lane: oteryn-platform-auth
task_kind: audit
implementation_authorized: false
status: investigating
risk: high
validation_intensity: HEIGHTENED
execution_mode: github_only
branch: audit/native-oauth-revocation-integrity-20260807
base_branch: main
base_sha: 84922e4a24be9759c864b41efd34b1e43634d407
pr: 844
production_activation_authorized: false
cross_repository_mutation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
search_first:
  - Issue #801 and repair PR #825
  - open auth/game-auth audit or repair PRs and active tasks
optional_reads: []
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-postrepair-audit.md
modules:
  - native-oauth-revocation-integrity
coordination_key: audit:native-oauth-revocation-integrity
blockers: []
cross_repository_tasks: []
---

# OTERYN-20260807 native OAuth revocation post-repair audit

## Goal

Independently re-audit OPA-SEC-0003 / Issue #801 after repair PR #825. Prove or falsify that pre-revocation native OAuth authorization, access and refresh material cannot retain game-login authority across `game_auth_generation` changes, including concurrent issuance/refresh, revocation and ticket bootstrap.

## Scope

Read-only product evidence:

- `app/GameAuth/OAuth/NativeOAuthGenerationBinding.php`
- `app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php`
- `app/Identity/Actions/RevokeIdentityGameAuthorizations.php`
- `app/Providers/GameAuthOAuthServiceProvider.php`
- Passport authorization/access/refresh persistence used by the installed dependency
- `tests/Feature/GameAuth/OAuth/NativeOAuthRevocationGenerationTest.php`
- `tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php`
- `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md`

No application/runtime, migration, workflow, dependency, production, credential, session or external-repository mutation is authorized.

## Live selection evidence

- Selected trusted base: `main@84922e4a24be9759c864b41efd34b1e43634d407`.
- Issue #801 is closed completed after repair PR #825.
- PR #825 merged as `f6a2b6cefe8ad5993436ac18be8ca4d08919d69b` from exact implementation head `9183a55c04427ef7a56fa82d097173ef058d8d94`.
- The repair-to-current-main delta does not modify the audited native OAuth runtime or regression-test paths.
- No open `programme:platform` + `programme:audit-repair` Issue currently owns this slice.
- Active production-verification/native-protocol tasks remain separate and are not product-code ownership for this read-only audit.

## Proven so far

- Authorization codes carrying `game:ticket` are stamped with the current identity generation.
- New access tokens carrying `game:ticket` derive generation from their authorization code or refresh-source token and fail closed when that generation differs from the current Identity generation.
- Game Login Ticket bootstrap locks the Identity first, locks the access token, and requires token generation to equal current Identity generation before ticket issuance.
- `RevokeIdentityGameAuthorizations` locks the Identity, increments generation, and revokes current game-ticket authorization codes, access tokens and refresh tokens in the same transaction.
- Dedicated MariaDB coverage races revocation against OAuth ticket bootstrap.
- The only API route authenticated by the Passport `api` guard is the bounded game-auth ticket issuance route in `routes/api.php`.

## Current audit hypothesis

`UNKNOWN`: token issuance or refresh can overlap revocation because Passport persists an access token before its `AccessTokenCreated` listener and persists a refresh token after access-token issuance. The audit must prove whether every legal interleaving still fails closed for game-ticket authority and whether any stale refresh artifact can cause a material bypass or unsafe failure mode.

This is not yet a finding.

## Acceptance inventory

- [x] Live main, active tasks, open PRs/Issues and repair ownership were refreshed.
- [x] Repair PR, repair-to-main delta and current native OAuth/revocation/bootstrap source were inspected.
- [x] Existing deterministic access-token, refresh-token and bootstrap-vs-revocation tests were inspected.
- [ ] Authorization-code/access-token/refresh-token issuance versus revocation interleavings are proven safe or a material root cause is recorded.
- [ ] Negative behavior for stale but unrevoked generation-mismatched refresh material is classified from exact implementation evidence.
- [ ] Repair exact-head CI/Game Auth concurrency/required security evidence is verified.
- [ ] Findings are deduplicated; confirmed material findings receive one existing or new durable Issue.
- [ ] Audit record receives exact-head CI / Agent Governance and clean PR hygiene before merge.
- [ ] Lifecycle closeout archives the task and reconciles the programme state.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T20:00:00+02:00
invocation_started_at: 2026-08-07T19:51:00+02:00
last_progress_at: 2026-08-07T20:00:00+02:00
head: ee35f64c5858a98443cb0833f0d3adfbd035f70e
branch: audit/native-oauth-revocation-integrity-20260807
pr: 844
status: investigating
phase: investigate
execution_mode: github_only
context_routes:
  - continuous-audit
  - auth-identity
  - security
  - database
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-postrepair-audit.md
proven:
  - Issue #801 is closed completed through repair PR #825.
  - Audited native OAuth runtime/test paths are unchanged between repair merge f6a2b6cefe8ad5993436ac18be8ca4d08919d69b and selected main.
  - Current bootstrap rejects missing or generation-stale game-ticket access tokens under an Identity-first lock.
  - Current revocation increments Identity generation and revokes currently visible game-ticket OAuth families under the same Identity lock.
  - Existing MariaDB coverage races revocation against ticket bootstrap.
derived:
  - The original OPA-SEC-0003 direct bearer-to-new-ticket bypass appears repaired for the inspected bootstrap path.
unknown:
  - Whether refresh/access issuance that overlaps revocation can leave usable post-revocation game-ticket authority or a material error path.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-postrepair-audit.md
validation:
  - command: repair merge f6a2b6c..current main path delta
    result: PASS
    evidence: no audited native OAuth runtime or regression-test path changed.
blockers:
  - none
ci_checks_for_current_head: 0
ci_check_generation: draft
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: prove or refute the OAuth issuance/refresh versus revocation interleaving and classify stale generation-mismatched refresh behavior before packaging the audit verdict
```
