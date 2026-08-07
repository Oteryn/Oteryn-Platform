---
task_id: OTERYN-20260807-native-oauth-revocation-postrepair-audit
project_lane: oteryn-platform-auth
task_kind: audit
implementation_authorized: false
status: validating
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
- installed Passport / League OAuth2 authorization, access-token and refresh-token persistence order
- `tests/Feature/GameAuth/OAuth/NativeOAuthRevocationGenerationTest.php`
- `tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php`
- `routes/api.php`
- `routes/internal.php`
- `docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md`

No application/runtime, migration, workflow, dependency, production, credential, session or external-repository mutation is authorized.

## Live selection and collision evidence

- Selected trusted base: `main@84922e4a24be9759c864b41efd34b1e43634d407`.
- Issue #801 is closed completed after repair PR #825.
- PR #825 merged as `f6a2b6cefe8ad5993436ac18be8ca4d08919d69b` from exact implementation head `9183a55c04427ef7a56fa82d097173ef058d8d94`.
- The repair-to-selected-main delta does not modify the audited native OAuth runtime or regression-test paths.
- No open `programme:platform` + `programme:audit-repair` Issue owns this slice.
- Active production-verification/native-protocol tasks remain separate and do not own the product paths inspected by this read-only audit.

## Audit result

**PASS — no new material defect is proven in this bounded Platform native-OAuth revocation slice.**

This verdict does not claim global game-auth production readiness. `AUTH_GAME_LOGIN_CONTRACT.md` still records deployment cutover and retirement/isolation of alternate legacy login paths as separate `UNKNOWN` production facts.

## Generation chain and revocation invariant

The repaired generation chain is coherent:

1. A `game:ticket` authorization code is stamped with the Identity generation at authorization creation.
2. A `game:ticket` access token derives its generation from the exact authorization-code security context or, for refresh, from the source access token carried by the encrypted refresh payload.
3. `AccessTokenCreated` compares that source generation with the current Identity generation. A mismatch revokes the newly persisted access token and aborts issuance.
4. `RevokeIdentityGameAuthorizations` locks the Identity, increments `game_auth_generation`, then revokes the currently visible game-ticket authorization codes, access tokens and associated refresh tokens in the same transaction.
5. Game Login Ticket bootstrap locks the Identity before the access-token row and rejects an access token whose stored generation differs from the locked current Identity generation.

The original OPA-SEC-0003 failure mode — using a pre-reset bearer to mint a fresh current-generation Game Login Ticket — is therefore no longer supported by the inspected path.

## OAuth issuance / refresh versus revocation interleaving

The remaining high-risk interleaving was checked against the exact installed dependency versions:

- `laravel/passport v13.7.5` (`90053dc4ba681c076855779250109bb624f961f6`);
- `league/oauth2-server 9.4.1` (`9d2f6fc0a0b5aa1bb02506971d3a4ecff2c6526c`).

Passport persists a new access token before dispatching `AccessTokenCreated`. League refresh processing persists/validates the source family, issues the new access token, and only afterwards issues the new refresh token.

Consequences for a grant racing revocation:

- If a stale authorization code remains exchangeable in a narrow concurrent interleaving, its stored old generation is propagated to the new access-token binding. The listener compares it with the now-current Identity generation, revokes the newly persisted access token and aborts before a new refresh token is issued.
- If stale refresh material remains visible long enough to pass the dependency's initial refresh-row revocation check, the new access token still derives the old source-token generation. A generation mismatch revokes that new access token and aborts before refresh-token continuation.
- If an old-generation access-token row itself escapes the revocation query because it was concurrently persisted, the ticket-bootstrap boundary still rejects it because bootstrap requires the access token generation to equal the locked current Identity generation.

Therefore the inspected issuance races can at most leave unusable/revoked or generation-stale OAuth persistence; they do not provide post-revocation `game:ticket` authority.

### Residual coverage observation

There is no dedicated deterministic test that pauses OAuth authorization/access/refresh persistence at each issuance-vs-revocation race boundary. Existing MariaDB concurrency evidence specifically races revocation against **ticket bootstrap**, while source-level dependency sequencing is used above for the issuance-race conclusion.

A stale generation-mismatched access-token listener throws a project `LogicException`; Passport's access-token controller normalizes League OAuth exceptions, not arbitrary project `LogicException`s. A generic OAuth error response for that extremely narrow stale-artifact race is therefore not independently proven. This remains a **non-material coverage/error-shape UNKNOWN**, not a security bypass or a confirmed actionable defect. No Issue is created from speculation.

## API authority boundary

The repository exposes one Passport `auth:api` route: `POST /api/v1/game-auth/tickets`. That route enters the generation-checked ticket bootstrap. Internal redeem/context routes use Gateway service authentication instead of Passport bearer authority.

Thus no second Platform API consumer was found where a stale `game:ticket` bearer could retain useful authority without passing the generation bootstrap check.

## Existing regression evidence

`NativeOAuthRevocationGenerationTest` covers:

- pre-revocation access-token rejection after generation change, including a deliberately un-revoked stale access token;
- pre-revocation refresh-token rejection;
- refresh-descendant generation inheritance;
- newly authorized post-revocation token success;
- disabled and terminated Identity rejection.

`GameTicketConcurrencyTest` uses independent processes against MariaDB and covers revocation racing ticket bootstrap; final generation advances, OAuth material is revoked, and any ticket that won the race retains the older generation and is therefore stale afterwards.

## Exact repair-head validation

PR #825 exact implementation head `9183a55c04427ef7a56fa82d097173ef058d8d94`:

- CI `31195817494`: **PASS**.
- Game Auth Ticket Concurrency `31195817269`: **PASS**.
- Agent Governance `31195817204`: **PASS**.
- Acceptance E2E and Visual UX `31195817350`: **PASS**.
- Deep System Validation `31195817276`: **PASS**.
- Platform DB Outage Validation `31195817308`: **PASS**.
- Phase 7 Production-Like Validation `31195817466`: **PASS**.
- Edge Security Emulation `31195817964`: **PASS**.

The same historical head also emitted a Native protocol contract-audits failure unrelated to auth correctness. That CI-routing false positive was separately tracked as Issue #829 and finally repaired through PR #834 with exact-head CI, Native protocol audits and Agent Governance passing. It is not used as negative native-auth evidence.

## Findings and deduplication

No new material finding is proven and no new Issue is created.

- OPA-SEC-0003 / Issue #801 remains the historical identity of the repaired bearer-generation bypass.
- Searches for OAuth refresh/generation/revocation and for a generation-mismatch OAuth error-shape defect found no separate existing actionable Issue.
- The residual issuance-race test gap is retained here as an explicit evidence limitation instead of being promoted to a speculative Issue.

## Acceptance inventory

- [x] Live main, active tasks, open PRs/Issues and repair ownership were refreshed.
- [x] Repair PR, repair-to-main delta and current native OAuth/revocation/bootstrap source were inspected.
- [x] Existing deterministic access-token, refresh-token and bootstrap-vs-revocation tests were inspected.
- [x] Authorization-code/access-token/refresh-token issuance versus revocation interleavings were classified from exact project and installed-dependency source; no usable post-revocation game-ticket authority was found.
- [x] Stale generation-mismatched refresh/access behavior was classified fail-closed for game-ticket authority; the narrow OAuth error-shape remains an explicit non-material coverage UNKNOWN.
- [x] Repair exact-head CI, Game Auth concurrency, Agent Governance and acceptance evidence were verified.
- [x] Findings were deduplicated; no confirmed new material root cause requires an Issue.
- [x] No product/runtime fix is included in this audit.
- [ ] Exact-head CI / Agent Governance for audit PR #844 pass with clean PR hygiene.
- [ ] Lifecycle closeout archives this task and reconciles the programme state.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-07T20:07:00+02:00
invocation_started_at: 2026-08-07T19:51:00+02:00
last_progress_at: 2026-08-07T20:07:00+02:00
head: d7c4a08b0ef634e03db0969953f8d6cbfac9c113
branch: audit/native-oauth-revocation-integrity-20260807
pr: 844
status: validating
phase: final_ci
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
  - Bootstrap rejects missing or generation-stale game-ticket access tokens under an Identity-first lock.
  - Revocation increments Identity generation and revokes currently visible game-ticket OAuth families under the same Identity lock.
  - Existing MariaDB coverage races revocation against ticket bootstrap.
  - Exact installed Passport/League issuance order propagates source generation to a new access token before any new refresh continuation can be issued.
  - No second Passport api-guard consumer was found outside generation-checked ticket issuance.
derived:
  - The original OPA-SEC-0003 direct bearer-to-new-ticket bypass is repaired in the bounded Platform native OAuth path.
  - Authorization/access/refresh issuance racing revocation does not retain usable game-ticket authority because old source generation is rejected on new access binding and again at ticket bootstrap.
unknown:
  - Exact client-visible OAuth error shape if an exceptionally narrow issuance race produces generation-mismatched material that reaches the project access-token listener after dependency validation.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - A stale refresh token can mint a usable post-revocation game-ticket access token; rejected by source-generation propagation plus current-generation comparison before refresh continuation.
  - A concurrently persisted old-generation access token can mint a new ticket after revocation; rejected by Identity-first ticket-bootstrap generation validation.
  - A stale game-ticket OAuth bearer has another Platform API authority path that bypasses ticket bootstrap; rejected by route inventory.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-postrepair-audit.md
validation:
  - command: repair merge f6a2b6c..selected main path delta
    result: PASS
    evidence: no audited native OAuth runtime or regression-test path changed.
  - command: PR #825 exact-head CI 31195817494
    result: PASS
    evidence: repository CI passed on implementation head 9183a55c04427ef7a56fa82d097173ef058d8d94.
  - command: PR #825 Game Auth Ticket Concurrency 31195817269
    result: PASS
    evidence: MariaDB concurrency proof passed on the exact implementation head.
  - command: PR #825 Agent Governance 31195817204
    result: PASS
    evidence: exact-head governance passed.
  - command: PR #825 Acceptance E2E 31195817350 and Deep System Validation 31195817276
    result: PASS
    evidence: acceptance and deep-system gates passed on the exact implementation head.
  - command: audit-document runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: audit PR changes durable audit evidence only; audited runtime evidence is inherited from the exact implementation head and direct code/test inspection.
blockers:
  - none
ci_checks_for_current_head: 0
ci_check_generation: pending
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
context_reconstruction_attempts: 0
stall_warnings: 0
next_action: mark PR #844 ready, require exact-head CI and Agent Governance, inspect PR hygiene, then merge and archive if clean
```
