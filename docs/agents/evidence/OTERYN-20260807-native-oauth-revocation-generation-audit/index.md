# Evidence — OTERYN-20260807 native OAuth revocation-generation audit

## Target

- Repository: `blakinio/Oteryn-Platform`
- Audited main: `acc7a5dc58c501cdcd235e5da16464060363ef43`
- Finding: `OPA-SEC-0003` / Issue #801
- Risk / priority: HIGH / P1

## Credential-revocation evidence

- `app/Identity/Credentials/IdentityCredentialUpdater.php`: password change and reset both call `RevokeIdentityGameAuthorizations`.
- `app/Identity/Recovery/IdentityRecoveryKeyService.php`: successful recovery-key recovery also calls `RevokeIdentityGameAuthorizations`.
- `app/Identity/Actions/RevokeIdentityGameAuthorizations.php`: the revocation action increments only `identities.game_auth_generation` and records an event.

## Native OAuth bootstrap evidence

- `routes/api.php`: `POST /v1/game-auth/tickets` is authenticated by Passport `auth:api`.
- `GameLoginTicketIssueController`: resolves the current Passport access-token ID and forwards it to `IssueGameLoginTicketFromOAuth`.
- `IssueGameLoginTicketFromOAuth`: checks token existence, revoked flag, expiry, user identity, `game:ticket` scope and expected client; no issuance-generation/current-generation comparison is performed.
- After those checks it calls `IssueGameLoginTicket` and revokes the OAuth access/refresh material only after successful ticket creation.
- `IssueGameLoginTicket`: reads the current Identity and stores the current `game_auth_generation` into the new Game Login Ticket.

## Falsified revocation invariant

A Passport access token issued at generation N can remain unrevoked after password reset/change/recovery increments the Identity to generation N+1. The bearer is then accepted by the OAuth bootstrap and creates a new Game Login Ticket stamped N+1. The existing stale-ticket generation check therefore cannot reject that newly-created ticket.

## Test evidence

- `tests/Feature/GameAuth/OAuth/NativeOAuthPkceTest.php` proves public-client PKCE/S256 and short token lifetime.
- No inspected test increments `game_auth_generation` after OAuth token issuance and before `/api/v1/game-auth/tickets` bootstrap.
- Existing Game Auth revocation coverage concerns ticket/session generation but does not establish revocation of the upstream Passport bearer capability.

## Contract evidence

`docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md` requires credential-change/reset session/token revocation coupling before authoritative game-auth rollout and identifies Identity/OAuth -> Game Login Ticket as the delivered Oteryn producer chain.

## Duplicate search

No open or closed Issue was found for this OAuth-bearer generation-binding root cause.

## Safety

No token, credential, staging/production environment or external repository was mutated. No real authentication secret is included in this evidence.
