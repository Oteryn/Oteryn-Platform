# NATIVE-LOGIN owner steps (Synology preproduction)

Plain checklist to bring native game login up end to end: Platform and Gateway on the Synology stack, the Game node on the same NAS, and the Windows client. Nothing here is a secret; the values you set are identifiers and addresses. Keys, certificates and passwords stay on their hosts. Examples use addresses already in the repository (`https://oteryn.molehill.cloud` Platform, `https://gateway.molehill.cloud` Gateway, Game port `7172`); use your NAS address.

## Route publication on the Synology store

`game-auth:native-topology:issue` and `game-auth:native-route:publish` reach the deployed Synology store only through the preproduction-store mode (`app/GameAuth/Worlds/PreproductionNativeRouteStore.php`, ADR 0044). Since the PostgreSQL cutover the deployed store is PostgreSQL, and its code fence admits it only as the `pgsql` identity of ADR 0044 Amendment P4 (accepted 2026-10-14): host `postgres`, port `5432`, `sslmode=verify-full`, database `oteryn_platform`, connection `pgsql`, role `oteryn_platform_app`. The fence still also admits the retired `mysql` identity (host `mariadb`, port 3306) until the P7 follow-up removes it; no deployed store uses it. Staging turns the deploy step on in `deploy/synology/native-defaults.env` (`OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED=true`); the step refuses unless the Registry issues exactly the configured `OTERYN_STAGING_NATIVE_WORLD_ID`, and a refusal writes nothing.

## A. GitHub repository variables (Oteryn-Platform, Settings, Secrets and variables, Actions, Variables)

1. Native login switches (set together, or the deploy refuses):

   | Variable | Format and example |
   | --- | --- |
   | `OTERYN_STAGING_NATIVE_ADMISSION_ENABLED` | `true` |
   | `OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES` | JSON, certificate subject to scope list: `{"CN=oteryn-preprod-node-1-runtime-status":["<WORLD_ID>/<CHANNEL_ID>"]}` (subject from Game `preprod-topology.toml` `[node] identity`) |
   | `OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES` | same JSON format for `CN=oteryn-preprod-game-ops` |

   Verify: the deploy log has no line starting `Native admission needs`.

2. Route variables (all required once publication is on; a missing or malformed one stops the deploy and names the variable):

   | Variable | Format and example |
   | --- | --- |
   | `OTERYN_STAGING_NATIVE_WORLD_ID` | lower-case UUIDv7, e.g. `0192b3c4-5d6e-7f80-9a1b-2c3d4e5f6a7b` (existing variable; must equal the WorldId the Registry issues, see step 5) |
   | `OTERYN_STAGING_NATIVE_CHANNEL_KEY` | `ch1` (a-z, 0-9, hyphen; descriptor `[scope] channel_key`) |
   | `OTERYN_STAGING_NATIVE_ROUTE_HOST` | `192.168.1.20` (NAS LAN address; no scheme, port or path; descriptor `[route] host`) |
   | `OTERYN_STAGING_NATIVE_ROUTE_PORT` | `7172` (descriptor `[route] port`) |
   | `OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME` | `node-1.preprod.oteryn.internal` (descriptor `[route] tls_server_name`; DNS name, equals the gameplay certificate SAN) |
   | `OTERYN_STAGING_NATIVE_ROUTE_LOGIN_ENABLED` | `true` (default when unset) or `false` |
   | `OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED` | `true` to publish on deploy (the committed staging default; see the section above) |

## B. Platform deploy

3. If the release changed, run workflow **Build Synology Staging Images** from `main`; note the release SHA and image digests it prints.
4. Run workflow **Deploy Synology Staging** from `main` (approve the environment): `action=deploy`, `release_sha`, the image fields from step 3, `game_world_host` = NAS LAN address, `game_world_id` = `1`, slug, name and region as before.
   Verify in the log: `Native admission signing key trusted (PUBLIC values ...)`, the signing self-check passing, `Oteryn native OAuth client id: <id>` (Rust client), and with publication on, `Native topology (non-secret receipt ...)` then `Native route record published (non-secret receipt ...)`.
5. If the run stops with `The Registry issued native WorldId <uuid>, which differs from OTERYN_STAGING_NATIVE_WORLD_ID`, copy that UUID into the variable and run step 4 again (nothing was published). Write down for the Game side: `world_id`, `channel_id`, `route_version`, `route_revision`, and from the trust line `key_id` and `public_key`.
6. Confirm the run ends healthy. Re-running with the same values keeps `route_revision`; changing host, port or TLS name advances it and invalidates old grants, so update the Game descriptor afterwards (step 8).

## C. Game node (Oteryn-Game `deploy/synology-game`, its README is authoritative)

7. One-time operator setup (README steps 1 to 6): PostgreSQL 17 with TLS, directory layout, config from templates, ops binary, wrapper and sudoers, supervision. Needs the NAS LAN address, the exposed Game port, and certificates under `BASE/node/secrets` (gameplay certificate with `serverAuth` and the SAN of step 2; Platform client, runtime-status, projection and ops authority certificates with the `[certificates]` subjects).
8. Fill `preprod-topology.toml` by Game manifest PR from step 5: `[scope] world_id`, `channel_id`, `channel_key`; `[route] host`, `port`, `tls_server_name`, `route_version`, `route_revision`; `[readiness] route_revision`. Install it as `ROOT_BASE/ops/preprod-topology.toml`, plus `scope.env`, `node.toml`, `report.toml`. Verify: `deploy-ops.sh check-setup` passes.
9. First start, README steps 1 to 10, once by hand: install bundle, `oteryn-game-migrate`, launch authorization, S2 authorization, `character fresh-store` twice, `character interpretation` (and epoch fence F, step 6a), scope grants and `content activate`, record gameplay digest, `supervisor.sh start` and `health`, then within 120 s `ops assignment assign`. Verify: node log shows `readiness ready=true`.
10. Create the tester character: on Platform `game-auth:character-bootstrap-intent:issue` (fresh UUIDv7 operation id, the four interpretation tokens), then on the node `ops character bootstrap`. Record the `character_id`. The bootstrap-intent read route is on from `deploy/synology/native-defaults.env` (node `[platform]` certificate `CN=oteryn-preprod-node-1-native-evidence`, TTL 300 s); a Platform deploy must have picked that up first.
10a. Native login and tester bootstrap need the native-evidence route. The owner authorized one-way preproduction activation on 2026-10-15 for client QA; `native-defaults.env` retains `OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED=true`. Never revert it after activation (`docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`). The Game node keeps evidence floors from then on, so a later Platform store or WorldId reset also needs a Game-side reset. Run step 4 with the activated defaults and verify an authenticated account-security observation before tester bootstrap.

## D. Windows client

11. Create `client.env` on the PC (no secret in it):

    ```text
    OTERYN_PLATFORM_URL=https://oteryn.molehill.cloud
    OTERYN_GATEWAY_URL=https://gateway.molehill.cloud
    OTERYN_OAUTH_CLIENT_ID=<id from the deploy line "Oteryn native OAuth client id">
    OTERYN_WORLD=<WorldId of step 5>
    OTERYN_CHARACTER_ID=<character_id of step 10>
    OTERYN_DEV_ROOT=<Windows path of the gameplay root the client expects>
    ```

    Both URLs must be `https` (plain http is accepted only for loopback). The PC must trust the CA of the gameplay certificate. Open the Game port to the LAN only; do not forward it on the router.
12. Load `client.env`, start the client and log in. Verify: browser OAuth page, sign-in, Gateway admission, connection to the node. Error codes: `NATIVE_LOGIN_ROUTE_UNAVAILABLE` = route record, node readiness or route revision disagree (compare step 5 values); `NATIVE_LOGIN_CHARACTER_CONFLICT` = wrong character id for this World; `NATIVE_LOGIN_AUTHENTICATION_REQUIRED` = ticket not accepted.

## What is tested where

The deploy step is covered by `tests/ci/test_synology_native_route_publication.py`. The Gateway process, the Game node and its verification of the grant, TLS to the client and the client need the Game and Gateway repositories and real hosts, so only steps 9 and 12 prove them.
