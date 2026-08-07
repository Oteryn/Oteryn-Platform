# Audit report — native OAuth revocation generation

## Scope

Repository-only audit of the delivered Platform Identity credential-revocation boundary and native OAuth `game:ticket` bootstrap on `main@acc7a5dc58c501cdcd235e5da16464060363ef43`.

No runtime, workflow, production, staging or external-repository mutation was performed.

## Verdict

`FINDING — HIGH / P1 / PROVEN`

Finding: `OPA-SEC-0003` / Issue #801.

## Security path reviewed

```text
password change / password reset / recovery-key recovery
  -> RevokeIdentityGameAuthorizations
  -> identities.game_auth_generation++

native OAuth bearer issued before increment
  -> POST /api/v1/game-auth/tickets
  -> Passport auth:api
  -> IssueGameLoginTicketFromOAuth
  -> validates token revoked/expiry/user/scope/client
  -> IssueGameLoginTicket(current Identity)
  -> new Game Login Ticket.security_generation = current generation
  -> old OAuth bearer revoked only after successful bootstrap
```

## Primary evidence

1. `IdentityCredentialUpdater::change()` and `reset()` revoke web sessions and call `RevokeIdentityGameAuthorizations`.
2. `IdentityRecoveryKeyService::recover()` also calls the same game-authorization revoker.
3. `RevokeIdentityGameAuthorizations` only increments `game_auth_generation`; Passport authorization/access/refresh records are untouched.
4. `IssueGameLoginTicketFromOAuth` accepts an unexpired, unrevoked access token belonging to the Identity with `game:ticket` scope and expected client. No authorization-generation binding exists.
5. `IssueGameLoginTicket` locks the current Identity and stamps the newly-created ticket with the current generation.
6. Consequently a bearer issued before credential revocation can mint one new ticket after revocation, and that new ticket is not stale because it carries the new generation.
7. Access/refresh material is revoked in `IssueGameLoginTicketFromOAuth` only after the bootstrap has succeeded.
8. Existing OAuth PKCE coverage proves short lifetime/PKCE but does not test a generation increment between token issuance and ticket bootstrap.

## Contract comparison

`AUTH_GAME_LOGIN_CONTRACT.md` treats credential change/reset session/token revocation coupling as required before authoritative game-auth rollout and represents the delivered Oteryn chain as Identity/OAuth -> single-use Game Login Ticket -> Gateway.

The current implementation protects old Game Login Tickets at redemption but leaves the upstream OAuth capability able to recreate a current-generation ticket.

## Impact

A stolen native OAuth bearer can survive a password reset/change or recovery event for its remaining token lifetime and regain game-login authority once after the user has attempted to revoke credentials. Native-auth production cutover is not currently proven, so this is not evidence of a live production compromise, but it is a material revocation flaw in the delivered producer path.

## Duplicate analysis

Open and closed Issue searches for OAuth/access-token + password reset/change + game-ticket revocation produced no actionable duplicate.

## Remediation handoff

Issue #801 owns remediation. The repair must bind OAuth authorization/token material to revocation generation (or equivalently revoke it atomically) and prove access-token, refresh-token and concurrency negative cases without broadening into production activation or legacy-path retirement.

## Audit delivery E2E

`NOT_APPLICABLE`: documentation/evidence-only audit package.
