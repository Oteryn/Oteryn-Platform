---
task_id: OTERYN-20260807-native-oauth-revocation-generation-audit
programme_id: OTERYN_PLATFORM_CONTINUOUS_AUDIT
project_lane: oteryn-platform-core
task_kind: audit
implementation_authorized: false
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/prompts/OTERYN_PLATFORM_CONTINUOUS_AUDIT_PROGRAM.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
---

# OTERYN-20260807 native OAuth revocation-generation audit

## Goal

Audit whether credential/game-authorization revocation on the delivered Platform Identity -> native OAuth -> Game Login Ticket path invalidates every pre-revocation credential capable of minting a new game credential.

## Acceptance criteria

- [x] Current main, active tasks, open PRs and live remediation ownership were refreshed.
- [x] Password change/reset/recovery revocation paths were traced to `game_auth_generation`.
- [x] Native OAuth authorization/access/refresh and Game Login Ticket bootstrap paths were inspected.
- [x] Existing revocation and PKCE tests were checked for the pre-revocation-token case.
- [x] Open and closed Issues were searched for the same root cause.
- [x] One material finding was routed to Issue #801 as OPA-SEC-0003.
- [ ] Exact-head documentation/governance CI passes, the audit package merges, and this task is archived with ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/reports/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/evidence/OTERYN-20260807-native-oauth-revocation-generation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
modules:
  - native OAuth/game-auth security audit records only
dependencies:
  - Issue #801 is the remediation handoff; this audit does not implement it.
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T10:27:00Z
head: 1460efe7bdc1d5c78dcc26664e0fa1d484271a2a
branch: audit/OTERYN-20260807-native-oauth-revocation-generation
pr: none
status: implementing
context_routes:
  - security
  - authentication-identity
  - api-contracts
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/tasks/archive/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/reports/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/evidence/OTERYN-20260807-native-oauth-revocation-generation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
proven:
  - Credential change/reset calls RevokeIdentityGameAuthorizations, and recovery-key recovery does the same.
  - RevokeIdentityGameAuthorizations increments only Identity.game_auth_generation; it does not revoke Passport authorization codes, access tokens or refresh tokens.
  - Existing Game Login Tickets carry security_generation and are invalidated when the Identity generation changes.
  - IssueGameLoginTicketFromOAuth validates Passport token identity, revoked/expiry state, scope and client, but has no comparison to the generation under which the OAuth authorization/token was issued.
  - The OAuth bootstrap then calls IssueGameLoginTicket, which reads the current Identity and mints a new Game Login Ticket using the current generation.
  - A pre-reset unexpired OAuth bearer can therefore cross the revocation boundary and mint a fresh current-generation ticket before that OAuth token is revoked by the successful bootstrap.
  - Existing PKCE tests do not cover password/reset/recovery generation changes after OAuth token issuance.
  - OPA-SEC-0003 is recorded as Issue #801 with risk high, priority P1 and implementation authorization.
derived:
  - `game_auth_generation` currently protects already-issued ticket redemption but not the earlier OAuth bearer-to-ticket bootstrap authority.
  - Password reset/change/recovery does not fully revoke delivered native game-login authority while a pre-revocation OAuth bearer remains valid.
unknown: []
conflicts:
  - AUTH_GAME_LOGIN_CONTRACT requires credential-change/reset session/token revocation coupling, while the delivered native OAuth bootstrap is not generation-bound before minting a new ticket.
first_failure:
  marker: OPA-SEC-0003
  evidence: a valid pre-revocation Passport bearer is accepted after game_auth_generation increments and can mint a new ticket stamped with the new generation.
rejected_hypotheses:
  - The generation increment directly revokes Passport tokens; rejected because RevokeIdentityGameAuthorizations only increments the Identity column.
  - Ticket security_generation closes the gap; rejected because the stale bearer mints a new ticket using the current generation after the reset.
  - OAuth single-use bootstrap closes the gap; rejected because the stale bearer retains one unauthorized post-reset bootstrap, which is the capability revocation is intended to remove.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/reports/OTERYN-20260807-native-oauth-revocation-generation-audit.md
  - docs/agents/evidence/OTERYN-20260807-native-oauth-revocation-generation-audit/index.md
  - docs/agents/programs/OTERYN_PLATFORM_CONTINUOUS_AUDIT.md
validation:
  - command: primary-source native OAuth revocation falsification on main@acc7a5dc58c501cdcd235e5da16464060363ef43
    result: PASS
    evidence: credential updater, generation revoker, OAuth bootstrap, ticket issuer, API route/controller and tests establish the gap.
  - command: duplicate root-cause search across open and closed Issues
    result: PASS
    evidence: no actionable duplicate found.
  - command: runtime/product E2E
    result: NOT_APPLICABLE
    evidence: this audit package changes documentation/evidence only and performs no native-auth activation.
blockers: []
next_action: Create the audit PR, bind this task to it, complete exact-head checks, merge, then archive the task and release ownership.
```
