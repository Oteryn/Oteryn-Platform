# Oteryn Synology staging deployment

This package targets early local/staging testing on Synology Container Manager. It is intentionally not a production deployment topology and does not satisfy the Production Go-Live Gate.

The NAS is a runtime target only. Platform, Game Gateway and the dedicated deployment-runner images are built on GitHub-hosted Actions runners. The Synology deployment path performs image pulls, database/runtime initialization, migrations, health checks, World Registry configuration and runtime-image rollback; it does not compile C++ or Go source on the NAS.

## Runtime topology

```text
workstation/browser
        |
        | DSM reverse proxy (controlled HTTP/HTTPS ingress)
        v
Synology host loopback
  8000 -> Platform
  8080 -> Game Gateway

Synology LAN address (opt-in, see Platform internal mTLS listener)
  8543 -> internal-mtls (TLS 1.3 + client certificate) -> FastCGI unix socket -> Platform

Docker private bridge
  Platform <-> PostgreSQL (verify-full TLS)
  Scheduler (always on) <-> PostgreSQL (verify-full TLS)
  Gateway -> HTTPS internal proxy -> Platform private API
```

The Gateway never receives database credentials. Its non-loopback dependencies use generated staging-only TLS certificates and hostname verification. PostgreSQL is not published on host ports. Platform and Gateway remain host-loopback-only.

The user-visible Platform origin is a separate concern from the private origin binding. The guarded public staging deployment requires `APP_URL=https://oteryn.molehill.cloud` for requestless console, scheduler, notification, reset, verification and signed URL generation while continuing to bind Platform only to `127.0.0.1:8000`.

## Repository workflows

- `Build Synology Staging Images` runs on GitHub-hosted runners. Pull requests build without publishing. Trusted `main` pushes and explicit manual runs publish GHCR images tagged with `sha-<full-sha>`; `main` also receives the moving `main` tag.
- `Deploy Synology Staging` is `workflow_dispatch`-only at its own boundary, refuses non-`main` dispatches and runs through organization runner group `platform-runners` with custom label `oteryn-platform`; guarded trusted-main build automation may dispatch it with exact release inputs.
- `Stage Synology Deploy Runner Image` (`workflow_dispatch`, `main` only) takes a `build_run_id` and an exact `oteryn-deploy-runner@sha256:` digest, authenticates the digest against the deploy-runner provenance artifact of that successful Build Synology Staging Images run on protected `main`, then pulls it into the NAS Docker host with the job's own token; the NAS host keeps no GHCR credentials. Repinning the organization runners then only changes the image line in their compose file and recreates the runner.
- The Platform runner is `oteryn-synology-platform`, registered at organization scope with `--runnergroup platform-runners` and `--no-default-labels`. Generic `self-hosted` jobs do not match it.
- Any temporary one-shot deployment workflow must be removed after its bounded deployment and sanitized evidence are complete.

## First-time setup order

1. Merge a reviewed version of this package so GitHub-hosted Actions can publish:
   - `ghcr.io/blakinio/oteryn-platform`;
   - `ghcr.io/blakinio/oteryn-game-gateway`;
   - `ghcr.io/blakinio/oteryn-deploy-runner`.
2. Register/recover the Platform organization runner on Synology using `docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md`.
3. Create the GitHub Environment `synology-staging` and configure its staging-only secrets/variables.
4. Run `Deploy Synology Staging` manually from `main`, preferably with an exact `sha-<full-sha>` Platform/Gateway image tag.

## Register or recover the Platform organization runner

The current Platform execution route is not the retired repository runner. Follow `docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md` and use:

- Compose reference: `deploy/synology/runner/compose.organization.example.yml`;
- Compose project/service: `oteryn-organization-runners/platform`;
- organization URL: `https://github.com/Oteryn`;
- runner group: `platform-runners`;
- runner name: `oteryn-synology-platform`;
- custom label: `oteryn-platform`.

The Platform runner retains Docker-socket access and the Platform staging-state mount because current trusted staging operations require them. Docker-socket access is host-equivalent privilege; selected-repository runner-group policy and trusted-main/protected-environment workflow gates remain mandatory.

First registration uses a one-time organization registration token from a local file as documented in the organization-runner runbook. After registration, truncate the token file and prove restart from the persistent `/runner` state without re-registration.

The old `oteryn-synology-staging` repository runner/container was retired after the replacement route passed. Its preserved recovery volumes/state are evidence, not an active container that autostart repair should recreate.

## GitHub Environment configuration

Create an Environment named `synology-staging`.

Required secrets:

```text
OTERYN_STAGING_APP_KEY
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN_SHA256
```

plus the five `OTERYN_STAGING_PLATFORM_POSTGRES_*_PASSWORD` secrets (*PostgreSQL service*). The retired `OTERYN_STAGING_MARIADB_ROOT_PASSWORD` and `OTERYN_STAGING_PLATFORM_DB_PASSWORD` are no longer read by any deploy script (plan P7); remove them only after the deploy workflow stops rendering them.

Optional rotation secrets:

```text
OTERYN_STAGING_PLATFORM_PREVIOUS_SERVICE_TOKEN_SHA256
```

Optional PostgreSQL secrets (all five or none; see *PostgreSQL service*):

```text
OTERYN_STAGING_PLATFORM_POSTGRES_SUPERUSER_PASSWORD
OTERYN_STAGING_PLATFORM_POSTGRES_OWNER_PASSWORD
OTERYN_STAGING_PLATFORM_POSTGRES_APP_PASSWORD
OTERYN_STAGING_PLATFORM_POSTGRES_BACKUP_PASSWORD
OTERYN_STAGING_PLATFORM_POSTGRES_DUMP_PASSWORD
```

Recommended variables:

```text
OTERYN_STAGING_STATE_DIR=/var/lib/oteryn-staging-state
OTERYN_STAGING_APP_URL=https://oteryn.molehill.cloud
```

The guarded public staging workflow fails closed when `OTERYN_STAGING_APP_URL` is set to any other value. This protects requestless user-visible links without exposing or changing the private loopback origin.

Generate a high-entropy staging service token and store its exact SHA-256 digest in the matching hash secret. The deploy script verifies the plaintext/hash pair before changing the stack.

For database passwords, use random hexadecimal/alphanumeric values. The deploy script rejects unsupported characters rather than attempting unsafe SQL interpolation.

## Binding policy

The rendered deployment environment has two host bind controls, both of which must remain exact loopback:

```text
PLATFORM_BIND_ADDRESS=127.0.0.1
GATEWAY_BIND_ADDRESS=127.0.0.1
```

The Platform internal mTLS listener has its own bind control, `PLATFORM_MTLS_BIND_ADDRESS` (default `127.0.0.1`, staging value `192.168.1.2` from `native-defaults.env`). It must be loopback or one exact RFC1918 private IPv4 address present on the host, and only that listener (TLS 1.3, client certificate required) is published there.

`GAME_WORLD_HOST` is the Platform-owned route returned to OTClient; it must be loopback or one exact RFC1918 private IPv4 address. Wildcard, public, multicast and link-local addresses are rejected.

## Deployment behavior

A `deploy` run:

1. checks out trusted `main` on the self-hosted runner;
2. logs in to GHCR with the job-scoped token;
3. validates all bind, route, image and secret inputs;
4. writes a permission-restricted ephemeral `.env` from GitHub Environment secrets;
5. validates required values and service-token hashes;
6. pulls prebuilt images;
7. starts internal TLS bootstrap and reconciles PostgreSQL (roles, TLS and privilege contract) and its backups;
8. starts Platform, runs migrations and ensures the native OAuth client exists;
9. starts the internal TLS proxy and Game Gateway;
10. verifies Platform/Gateway health and every exact host-port binding;
11. verifies canonical requestless Platform URLs, exact Gateway `/version` identity, bounded invalid-login response cache controls, the well-formed native login probe (see *Native login*) and no Platform/Gateway cross-routing;
12. only after those checks, creates or updates the exact enabled online Platform World Registry route;
13. removes the ephemeral `.env` after the job.

The script snapshots currently running Platform/Gateway image references before an update. `rollback` restores those runtime images and re-runs health checks. It intentionally does **not** reverse database migrations automatically.

## Native login

Native (protocol_version 2) login is the only Gateway login protocol. It needs both sides switched together, so `compose.yml` holds one value, `x-native-admission-enabled`, that feeds both:

```text
GAME_AUTH_NATIVE_ADMISSION_ENABLED   (Platform issuer)
GATEWAY_NATIVE_LOGIN_ENABLED         (Gateway native branch)
```

On this package native admission is opt-in (default off, see below): Synology runs `APP_ENV=preproduction`, the only deployed environment in which the Platform issuer signs grants (`testing` is for CI; `production` still refuses native admission and the legacy login switch stays as configured). The deployment proves the issuer can sign, or it refuses:

1. Before any stack change, `deploy.sh` runs `generate-native-admission-key.sh ensure`. On first use this creates the Ed25519 signing key inside the Docker named volume `oteryn-native-admission-keys`, which Docker stores under its root-only data root, outside the repository, the Compose context, the rendered `.env` and the runner state directory. The volume directory is mode 0700 and every file 0600, owned by the Platform process user (the keyring refuses anything else). The private key is generated and read only inside a throwaway container with no network and is never printed, logged or written to CI output. The command is idempotent and prints only the key id and the public key (`current_key_id=`, `current_public_key=`), which is what Game's trust config needs.
2. The Platform entrypoint exports `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_FILE` / `GAME_AUTH_NATIVE_ADMISSION_SIGNING_KEY_ID` (and, during rotation, `GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_FILE` / `GAME_AUTH_NATIVE_ADMISSION_RETIRING_SIGNING_KEY_ID`) from the read-only volume mount. No key value is ever a Compose value, `.env` entry or repository file.
3. After migrations and before the Gateway is recreated, `deploy.sh` runs `php artisan game-auth:native-admission:self-check` in the Platform container. It signs a throwaway grant with the configured key and verifies it against the Platform trust registry. A non-zero exit refuses the deployment. `health-check.sh` re-runs the same self-check whenever the flag is `true`, and requires the Gateway to reject an unknown native ticket through the Platform issuer (401 `NATIVE_LOGIN_AUTHENTICATION_REQUIRED`).

The signing key must also be trusted by the Platform registry and by Game (`current_public_key`); until it is, the self-check fails and the deployment is refused rather than leaving a Gateway that accepts logins no issuer can serve. The health check also fails when the Platform flag is missing or the two flags differ.

Key rotation (native login contract 9.3), run on the Synology host with `PLATFORM_IMAGE` set to the deployed Platform image:

```bash
PLATFORM_IMAGE=<image> deploy/synology/scripts/generate-native-admission-key.sh rotate [--maintenance]  # current -> retiring, new current
# publish the printed current_public_key to the Platform trust registry and Game, wait for propagation
PLATFORM_IMAGE=<image> deploy/synology/scripts/generate-native-admission-key.sh drop-retiring  # after the retiring key's grants expired
```

While native admission is live, `rotate` refuses (exit 3) unless run as `rotate --maintenance`, because the new key is untrusted until its public key is published; the safe order is to disable native admission and redeploy, rotate, publish and check, then re-enable. `rotate` and `drop-retiring` restart the running Platform container, because the entrypoint exports the key ids once at start. After `rotate` signing needs the new public key in the Platform trust registry; `check` runs the signing self-check in the running container. `show` prints the current ids and public keys without changing anything. Key ids are never reused.

Native-login settings need no hand-set GitHub variable: `deploy/synology/native-defaults.env` holds committed non-secret defaults (native World, runtime-status and scope-assignment identities, admission on). A repository variable of the same name overrides the file, and a change to the file ships through a normal PR. Native admission is opt-in. Set the repository variable (or the file value) `OTERYN_STAGING_NATIVE_ADMISSION_ENABLED=true` only together with `OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES` and `OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES` (the Game identities that report runtime status and own scope assignment; without them no scope is routable); deploy refuses otherwise. Rollback enables it for an image only if that image has both the self-check command and the entrypoint key export.

Account-character projection is on: `compose.yml` pins `GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED=true` with source authority `oteryn:character-authority:primary` and the projection identity `CN=oteryn-preprod-character-projection`, and turns the D171 fallback off (`GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_OWNERSHIP=false`). Native admission needs no World UUID: the client directory lists the World from the native route record. Route publication does: it is on by default (`OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED=true` in `native-defaults.env`), and `oteryn_native_route_publish_validate` refuses the deploy without `OTERYN_STAGING_NATIVE_WORLD_ID`, so that committed default must stay (a repository variable overrides it).

Rollback of a Platform image that predates the self-check sets `OTERYN_NATIVE_ADMISSION_ENABLED=false` for that image only (both flags stay paired), so the older release restores with native login off.

## Platform internal mTLS listener

The Game node calls Platform's Game-node internal routes over mutual TLS on the LAN. `compose.yml` service `internal-mtls` (Compose profile `internal-mtls`, nginx config `nginx/internal-mtls.conf`) terminates TLS on `${PLATFORM_MTLS_BIND_ADDRESS}:${PLATFORM_MTLS_PORT}` and passes requests to Platform's PHP-FPM pool over a unix socket:

- TLS 1.3 only, `ssl_verify_client on`, depth 1 (client certificates are issued directly by the preprod CA), no session tickets.
- `SSL_CLIENT_VERIFY`, `SSL_PROTOCOL` and `SSL_CLIENT_S_DN` reach PHP only as FastCGI parameters from the TLS session; request headers cannot set them, and forwarding headers are blanked. Platform's peer middleware still requires `SUCCESS`, `TLSv1.3` and an allowlisted subject DN on every request.
- Only `POST /internal/v1/game-auth/{native-evidence, native-runtime-status, native-scope-assignments, native-account-characters, native-account-characters/watermark, character-bootstrap-intents/read, character-commands/pending, character-commands/read, character-commands/receipt}` exist. `native-admissions` (Gateway only), the premium-snapshot family and every other path return 404. Separately, `POST /internal/v1/game-public/{highscores, character-profile, character-deaths, guild, house, world-online, watermark}` (Game Public Projections v1) is routed with its own 512 KiB body bound; Platform accepts it only while `GAME_PROJECTIONS_PUBLIC_ENABLED=true` in `testing` or `preproduction` and only from a subject listed in `GAME_PROJECTIONS_PUBLIC_IDENTITIES`, for example `["CN=oteryn-preprod-public-projection"]` (a dedicated subject, not shared with any other route), with `GAME_PROJECTIONS_PUBLIC_SOURCE_AUTHORITY` set to the producer's source authority (`oteryn:game:primary`). `compose.yml` forwards these settings fail-closed (off, no identity) together with `PUBLIC_GAME_DATA_WORLD_ID`, which names the World the public online, highscores and character pages serve (empty serves the only World that has published).
- The FastCGI socket lives on the `platform_fastcgi` tmpfs volume, shared only by `platform` and `internal-mtls`; there is no FastCGI TCP port. The terminator joins only the `mtls_edge` network, so it cannot reach PostgreSQL, Gateway or the internal proxy. With Platform stopped it answers 502.
- Platform starts its pool only when `OTERYN_PLATFORM_FASTCGI_ENABLED=true` (fed from the same `OTERYN_PLATFORM_MTLS_ENABLED` value); `php artisan serve` on :8000 is unchanged.

Settings (environment file or exported variable, else `native-defaults.env` `OTERYN_STAGING_NATIVE_PLATFORM_MTLS_*`, else the built-in default):

| Variable | Staging value | Built-in default |
| --- | --- | --- |
| `OTERYN_PLATFORM_MTLS_ENABLED` | `true` | `false` |
| `PLATFORM_MTLS_BIND_ADDRESS` | `192.168.1.2` | `127.0.0.1` |
| `PLATFORM_MTLS_PORT` | `8543` | `8543` |
| `PLATFORM_MTLS_SERVER_CERT_FILE` | `/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.fullchain.pem` | same |
| `PLATFORM_MTLS_SERVER_KEY_FILE` | `/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.key` | same |
| `PLATFORM_MTLS_CLIENT_CA_FILE` | `/volume1/oteryn/preprod-platform-tls/oteryn-preprod-ca.crt` | same |

The three files are host files mounted read-only with `create_host_path: false`; a missing file fails the deployment instead of creating an empty directory. They are never copied into the repository, the Compose context, the rendered `.env` or CI output. `deploy.sh` checks the settings, that the Platform image carries the FastCGI pool, that the bind address exists on the host, that the server certificate is valid and that nginx accepts the config with the mounted files (`nginx -t`) before changing the stack. The certificate check runs a TLS 1.3 handshake in memory in a container with no network: the chain must verify against the mounted CA file, be inside its validity period, carry the DNS SAN `platform-internal.preprod.oteryn.internal` and allow server authentication, and the key file must match the certificate. Only the verdict leaves the container. After the preflight, it, starts or removes `internal-mtls` after Platform and the Gateway. `rollback.sh` runs the same settings check and preflight before stopping anything, so a rollback cannot take down a working listener. While the listener is on, it refuses a last-good Platform image built before the listener; set `OTERYN_PLATFORM_MTLS_ENABLED=false` to roll back to such an image. It then reconciles the listener the same way. An unresolved migration candidate is finalized with the listener off (logged) when its Platform image was built before the listener or its running Platform has FastCGI off; the deploy then turns the listener on with its own preflighted image. While the listener is on, the shared ingress stop (`lib.sh` `oteryn_native_admission_ingress_stop`) stops `internal-mtls` together with the internal proxy and the Gateway. Every route that replaces Platform uses it: the fast, full and candidate deploy paths, rollback, the Platform reconcile, key promotion, Character Bazaar enable and prepare-rollback, the Marketplace runtime recreation and the failure restore. The listener comes back only after the gates pass: it starts again with the ingress, at the final self-check, or it is recreated by the listener reconcile. While the setting is off, the same stop removes any existing `internal-mtls` (a no-op when there is none), so a listener left from before the setting was turned off cannot keep answering for a replaced Platform or come back through `restart: always`. `health-check.sh` fails when the listener is on and the running Platform image has no FastCGI pool, and verifies the binding, the FastCGI socket, the server chain for `platform-internal.preprod.oteryn.internal` against the client CA file over TLS 1.3, and that a request without a client certificate is refused (400). It also checks that the nginx user can open the FastCGI socket. Both services mount `platform_fastcgi` with `nocopy`, so the tmpfs keeps its own owner and mode. The fast deploy path takes the full path when the listener's bind address, port, host paths, certificate, key, client CA or nginx config changed. It compares a digest of these inputs with the `oteryn.platform-mtls.fingerprint` label of the running `internal-mtls`. While the pool runs, the Platform entrypoint stays PID 1, and when either the pool or the main process exits it stops the container, so `restart: always` brings both back.

Independence from tls-init: the server chain and the client trust file come from the long-lived preprod CA (`/volume1/oteryn/preprod-ca-public/oteryn-preprod-ca.crt`, nameConstraints `DNS:preprod.oteryn.internal`), not from the `internal_tls` volume. A tls-init rotation therefore changes neither the server certificate the Game node verifies nor the CA that verifies Game node certificates.

Host steps to (re)issue the server certificate (run on the Synology host as root, with the preprod CA key wherever the operator keeps it; nothing here goes into Git):

```bash
umask 077
d=/volume1/oteryn/preprod-platform-tls
openssl genpkey -algorithm EC -pkeyopt ec_paramgen_curve:P-256 -out "$d/platform-internal-preprod.key"
openssl req -new -key "$d/platform-internal-preprod.key" -subj "/CN=platform-internal.preprod.oteryn.internal" \
  -out "$d/platform-internal-preprod.csr"
printf '%s\n' 'basicConstraints=critical,CA:FALSE' 'keyUsage=critical,digitalSignature' \
  'extendedKeyUsage=serverAuth' 'subjectAltName=DNS:platform-internal.preprod.oteryn.internal' > "$d/server.ext"
openssl x509 -req -in "$d/platform-internal-preprod.csr" -CA /volume1/oteryn/preprod-ca-public/oteryn-preprod-ca.crt \
  -CAkey <preprod-ca-key> -CAcreateserial -days 730 -sha256 -extfile "$d/server.ext" -out "$d/platform-internal-preprod.crt"
cat "$d/platform-internal-preprod.crt" /volume1/oteryn/preprod-ca-public/oteryn-preprod-ca.crt > "$d/platform-internal-preprod.fullchain.pem"
cp /volume1/oteryn/preprod-ca-public/oteryn-preprod-ca.crt "$d/oteryn-preprod-ca.crt"
rm -f "$d/platform-internal-preprod.csr" "$d/server.ext"
chmod 0700 "$d"; chmod 0600 "$d/platform-internal-preprod.key"; chmod 0644 "$d"/*.crt "$d"/*.pem
openssl verify -CAfile "$d/oteryn-preprod-ca.crt" "$d/platform-internal-preprod.crt"
```

The nginx master reads the key as root before dropping privileges, so 0600 root-owned is sufficient. After replacing files, redeploy (the reconcile recreates `internal-mtls`).

Client identities: the subject DN Platform compares is nginx's RFC 2253 form, for example `CN=oteryn-preprod-character-projection`. Each route's allowlist is its own Platform setting (`GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES`, `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES`, `GAME_AUTH_NATIVE_EVIDENCE_MTLS_CLIENT_IDENTITY`, `GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_IDENTITIES`, ...); a certificate the CA accepts but no allowlist names is refused by Platform.

Route settings per Game-node route (Platform answers 503 while a route's settings are incomplete):

| Route | Platform settings | Staging state |
| --- | --- | --- |
| `native-runtime-status` | `GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED`, `GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES` | rendered by the deploy workflow from `native-defaults.env` |
| `native-scope-assignments` | `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED`, `GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES` | rendered by the deploy workflow from `native-defaults.env` |
| `native-account-characters`, `native-account-characters/watermark` | `GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_*` | pinned on in `compose.yml` (Character Authority projection, see below) |
| `native-evidence` | `GAME_AUTH_NATIVE_EVIDENCE_ACTIVATED`, `_MTLS_CLIENT_IDENTITY`, `_FRESH_ACCOUNT_PURPOSE`, `_FRESH_ACCOUNT_SCOPE`, `_CLOCK_UNCERTAINTY_SECONDS` (0..5), `_SOURCE_AUTHORITY` (default `platform`), `_REQUESTS_PER_MINUTE` (default `120`) | bindings committed in `native-defaults.env` for the node 1 `[platform]` certificate (`CN=oteryn-preprod-node-1-native-evidence`); activation off, so still 503 |
| `character-bootstrap-intents/read` | `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_MTLS_CLIENT_IDENTITY`, `GAME_AUTH_CHARACTER_BOOTSTRAP_INTENT_TTL_SECONDS` (1..300) | on from `native-defaults.env`: the same node 1 `[platform]` certificate, TTL 300 |
| `character-commands/pending`, `character-commands/read`, `character-commands/receipt` | `CHARACTER_AUTHORITY_COMMANDS_ENABLED`, `CHARACTER_AUTHORITY_COMMAND_IDENTITIES` (JSON list), `CHARACTER_AUTHORITY_COMMAND_SOURCE_AUTHORITY`, `CHARACTER_AUTHORITY_COMMAND_MAX_PENDING_INTENTS` (1..40); web creation: `CHARACTER_AUTHORITY_WEB_CREATE_ENABLED` | on in `native-defaults.env` (identity `CN=oteryn-preprod-node-1-character-commands`, source authority `oteryn:character-authority:primary`, bound 40); web creation on, so the Create character form is available |

The deploy workflow does not render the native-evidence and bootstrap-intent settings, so `lib.sh` `oteryn_resolve_game_node_route_settings` resolves each from the environment file or exported variable, else `native-defaults.env` (`OTERYN_STAGING_NATIVE_EVIDENCE_*`, `OTERYN_STAGING_NATIVE_BOOTSTRAP_INTENT_*`), else the fail-closed built-in (not activated, empty identity, binding and TTL; source authority and rate limit mirror the Platform config defaults, because an empty value would replace them). A value other than `true` or `false` for the activation switch refuses the run before any stack change. Turning a route on is a reviewed change to `native-defaults.env` that names the issued client certificate's subject DN. The Game node presents one `[platform]` client certificate for both native evidence and bootstrap-intent reads, so those two routes share it; it stays distinct from the runtime-status, scope-assignment and account-character identities. Native evidence activation is one-way (`OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md`): flipping `OTERYN_STAGING_NATIVE_EVIDENCE_ACTIVATED` to `true` is a separate owner decision.

Character Authority commands and web Character creation follow `docs/operations/CHARACTER_AUTHORITY_COMMANDS_ENABLEMENT.md` (Game contract `oteryn-game-character-authority-commands-v1` §14 step 4). The same `lib.sh` function resolves them from `OTERYN_STAGING_NATIVE_CHARACTER_*` in `native-defaults.env`; intent TTL and request rate are not mapped and keep the Platform defaults (300 s, 600 per minute). Platform honours them only while `APP_ENV` is `testing` or `preproduction`; this stack is `preproduction`. The run is refused before any stack change when a switch is not `true` or `false`, when the pending-intent bound is not an integer from 1 to 40, when the commands are enabled without an identity list and source authority, or when web creation is enabled without the commands. The bound is at most 40, the Platform code default, and Platform makes the command surface unavailable for a larger value (Oteryn/Oteryn-Platform#1632): at 6 s per unseen key (read plus receipt) the contract §4.1 arithmetic keeps 41 keys inside the 300 s TTL, and 40 keeps one key of margin, so a larger backlog could expire unread. Enabling is a reviewed change to `native-defaults.env`: first `OTERYN_STAGING_NATIVE_CHARACTER_COMMANDS_ENABLED=true` with `OTERYN_STAGING_NATIVE_CHARACTER_COMMAND_IDENTITIES=["CN=<subject>"]`, the subject of the dedicated Game consumer certificate (used by no other internal purpose); after the Game node's `[platform.character_commands]` table is in place and its first pending-list exchange answers `200`, `OTERYN_STAGING_NATIVE_CHARACTER_WEB_CREATE_ENABLED=true`. Roll back in reverse: web creation off first, commands on until the pending list is empty.

Fail closed: every gate failure that removes Platform, the internal proxy and the Gateway (trust gate, self-check gate, Marketplace runtime recreation, failure restore) first removes `internal-mtls` as well (`lib.sh` `oteryn_platform_mtls_remove_fail_closed`), so a listener the ingress stop only stopped cannot come back through `restart: always` in front of an unproven Platform. The next deploy recreates it behind the gates. The failure restore (fast deploy and Platform reconcile EXIT traps) reconciles the listener after the restored Platform passes the trust gate: recreated while the setting is on, even when the failed forward gate already removed it, and removed while it is off.

Native account-character push from the Game node is switched on by this package. `compose.yml` sets, on Platform:

```text
GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED=true
GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_SOURCE_AUTHORITY=oteryn:character-authority:primary
GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_IDENTITIES=["CN=oteryn-preprod-character-projection"]
```

`health-check.sh` leaves native admission enabled only after the running Platform reports a live account-character feed (a fresh watermark from the Character Authority; `lib.sh` `oteryn_wait_native_account_characters_feed`, waits up to `OTERYN_NATIVE_ACCOUNT_CHARACTERS_FEED_WAIT_SECONDS`, default 90). Without it every login would be refused while the unknown-ticket probe passes, so the check fails closed and the deployment is not reported healthy.

The D171 fallback is off, and native admission no longer needs a World UUID. `OTERYN_STAGING_NATIVE_WORLD_ID` stays committed in `native-defaults.env` because route publication, on by default, requires it; removing it makes every deploy fail before route publication.

## PostgreSQL service

ADR 0046 makes PostgreSQL 17 the Platform database. `docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md` added it beside MariaDB (P4), cut staging over (P6, `scripts/cutover-postgres.sh`) and removed MariaDB (P7): `compose.yml` has no `mariadb` service or `mariadb_data` volume, and no deploy script reads `MARIADB_ROOT_PASSWORD`, `PLATFORM_DB_USER` or `PLATFORM_DB_PASSWORD`.

Engine: `release-contract.env` `PLATFORM_DB_ENGINE=postgres`, the only value `lib.sh` accepts. `lib.sh` exports the runtime connection (`OTERYN_PLATFORM_DB_*`) that `compose.yml` passes to Platform and the scheduler as `DB_*`. Every image carries the engine in its release contract, and a candidate or rollback image built for MariaDB (no engine, or `mariadb`) is refused. `deploy.sh` and `deploy-impact.sh` still run `lib.sh` `oteryn_require_cutover_deploy_window` before any change: a deploy needs the clean-slate bootstrap's complete `bootstrap-complete.env` (a valid UTC `BOOTSTRAPPED_AT`, `YYYY-MM-DDTHH:MM:SSZ` naming a real date as `bootstrap-postgres.sh` writes it, and nonempty OAuth client and native world IDs, as the cutover adoption path requires), and is refused once a cutover rollback started, in the rollback window (bootstrap complete, native re-enable not started) for any release other than the current one once the frozen MariaDB release left `current-release.env` (its deploy would move a PostgreSQL release into `last-good-release.env`, which the cutover rollback restores through `rollback.sh` and refuses on a PostgreSQL image) or while `current-release.env` is missing, or when a compose file defines the retired `mariadb` service once `retire-mariadb` started (also after an interrupted run) or finished. A cutover or bootstrap state directory that is not owner-only (owned by the deploying user, not group- or world-writable, as `bootstrap-postgres.sh` requires) is refused before any record in it is read. A cutover `run.lock` (a running step, or one interrupted before it recorded its marker) refuses every deploy. A readable `cutover-state.env` that `cutover-postgres.sh` cannot have written is refused, never taken as no step recorded: `TASK_ID` first, then step markers with nondecreasing UTC times, each followed by exactly the keys its step records and each allowed by the step model in `lib.sh` (`_oteryn_cutover_step_allowed`, derived from the step guards: for example no re-enable after a started rollback and no rollback after re-enable). In the rollback window the current and pending candidate release records count only after `release-state.sh validate` accepts them. A cutover state or completion record that exists but cannot be read, a state whose absence cannot be proven (an unsearchable or non-directory parent), a compose file that cannot be read after the retirement started, and a linked cutover or bootstrap state directory are refused, never taken as absent.

- `deploy.sh` requires all five secrets and `PLATFORM_DB_NAME=oteryn_platform`, reconciles the roles before Platform starts, and runs migrations as `oteryn_platform_owner` in a one-off `platform` container (`docker compose run --rm` with `--database=pgsql_migrations`). The owner password reaches that container only as a named environment variable and never reaches the long-running services, which connect as `oteryn_platform_app`. Route publication (`OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=true`) uses the PostgreSQL store identity of ADR 0044 Amendment P4 (accepted 2026-10-14). The fast and reconcile paths preserve the `postgres` container.
- The pre-migration backup and the fresh-schema baseline write `platform.dump`: `pg_dump -Fc` as `oteryn_platform_dump` over `verify-full` TLS, password on standard input, kept only when `pg_restore --list` reads it. The evidence starts with `BACKUP_DB_ENGINE=postgres`.
- Schema recovery refuses evidence without `BACKUP_DB_ENGINE=postgres` (a MariaDB-era backup). The local superuser (socket, peer, no password) drops the database `WITH (FORCE)`, recreates it with the role script in `--database-only` mode, runs `pg_restore --single-transaction` as the owner and re-applies the grants.
- Marketplace staging control checks pending auctions in PostgreSQL and migrates as the owner.
- The health check requires the `postgres` service and the Platform container to connect as the app role over `verify-full`. The production-target preflight additionally checks TLS 1.3, SCRAM, `pg_hba`, the role attributes and ownership, that no TCP client is connected without TLS, and a Platform PDO probe proving the app role cannot create tables. Its restore drill streams a plain dump into a scratch database and drops it afterwards.

Service contract (`compose.yml` service `postgres`, profile `postgres`):

- image pinned by digest; private bridge only, no `ports` or `expose`; `restart: always`;
- data in the named volume `postgres_data`, never removed by any workflow or deploy script (`tests/ci/test_synology_postgres_contract.py`);
- TLS 1.3 only with a server certificate for `postgres` issued by a dedicated PostgreSQL CA (`tls-init`); the CA private key lives only in `postgres_ca_signer`, which only `tls-init` mounts (the stack CA key in `internal_tls` is readable by the root-started ingress, so it must not sign this certificate), the server pair lives in `postgres_tls`, and Platform mounts only the PostgreSQL CA certificate (`postgres_client_ca`) and connects with `sslmode=verify-full`;
- `pg_hba.conf` admits only `hostssl` SCRAM logins of the Platform roles from the bridge, plus local peer login of the `postgres` OS user; every other host rule is `reject`, including the superuser over TCP;
- health check: `pg_isready` plus a `verify-full` login of the runtime role.

Roles (`postgres/init/10-platform-roles.sh`, re-applied on every full deploy):

| Role | Purpose | Rights |
| --- | --- | --- |
| `oteryn_platform_owner` | migrations | owns database `oteryn_platform`, schema `public` and every migrated object |
| `oteryn_platform_app` | Platform runtime | `CONNECT`, schema `USAGE`, table DML and sequence use; no DDL, no `TRUNCATE` |
| `oteryn_platform_dump` | logical dump | `CONNECT`, schema `USAGE`, `SELECT` only |
| `oteryn_platform_backup` | physical backup | `REPLICATION` login only; no database access |

None has `SUPERUSER`, `CREATEDB`, `CREATEROLE` or `BYPASSRLS`. Default privileges extend the grants to every object the owner creates later.

Secrets (owner action): the five `OTERYN_STAGING_PLATFORM_POSTGRES_*_PASSWORD` secrets of the `synology-staging` environment, all required. Each must be a distinct generated value of at least 24 characters from `A-Z a-z 0-9 . _ -` (for example `openssl rand -hex 32`). The deploy workflow and `lib.sh` refuse a missing, partial or unsafe set before any stack change and never print a value. Every full deploy:

1. stages `postgresql.conf`, `pg_hba.conf` and the role script into `bootstrap_files`;
2. starts `postgres` (the first start runs `initdb` with the superuser password and data checksums);
3. pipes the four role passwords on standard input to the role script, run as the `postgres` OS user over the local socket with statement logging off;
4. proves the contract: no published port, a `verify-full` TLS login of the runtime role, refusal of a non-TLS login, refusal of DDL by the runtime role, no privileged role attributes and the expected ownership; then waits for the container to be healthy.

The health check then requires `postgres` to be running, healthy and unpublished.

Rotation: change the GitHub secret and run a full deploy; the role script re-applies the owner, app, backup and dump passwords, and Compose recreates `postgres` so the probe uses the new runtime password. The superuser password is used only by `initdb` on an empty volume and cannot log in over TCP, so changing that secret later has no effect on an existing cluster.

Rollback: remove all five secrets. The next deploy skips PostgreSQL and the health check ignores it. To stop the idle service, an operator runs `docker compose --profile postgres stop postgres`; the volume and its data stay. Never remove `postgres_data` without the restore drill and owner sign-off required by the plan.

## Attempt evidence and first-publication fence (P6a)

Design: `docs/operations/PLATFORM_POSTGRES_TRUST_SAFEGUARDS_DESIGN.md`. PostgreSQL is the only engine since P7, so every deploy sets the fence to `required` (`lib.sh` `oteryn_platform_db_engine_settings`).

- **Evidence share (owner action before P6 step 4).** A dedicated Synology shared folder `/volume1/oteryn-attempt-evidence` (override: `OTERYN_ATTEMPT_EVIDENCE_HOST_PATH`), owned by root, group 82, mode `2750`. It must be excluded from every snapshot, replication, Hyper Backup and restore task, and must be the real share, not a symlink or bind mount, on the same volume as Docker's root directory (`check-location` verifies the volume through the runner's named volumes). Set `chattr +a` on the directory if the file system allows it (hardening only). It is mounted read-write into the deploy runner (`runner/compose.yml`, and the `platform` runner of `runner/compose.organization.example.yml`, which runs the cutover) and read-only into the `platform` service (`compose.yml`) at `/var/lib/oteryn-attempt-evidence`. If the path does not exist, Docker creates a plain directory there; `attempt_evidence.py check-location` then refuses it, so step 4 fails closed.
- **Writer.** `scripts/attempt_evidence.py` is the only writer of the evidence directory (`check-location`, `open`, `append`, `verify`, `classify`, `require-open-attempt`). It makes no network call; the P6 cutover posts every anchor as a comment by `github-actions[bot]` (user id 41898282) on a locked anchor issue.
- **Guard.** On PostgreSQL with native admission on, `oteryn_publish_native_admission_trust` observes trust state (`game-auth:native-evidence:observe-trust`). With a profile present it publishes as before. With none, it requires an open, anchored attempt intent (`require-open-attempt`) and passes `--first-publication-fence` to `publish-trust`; otherwise it refuses.
- **Fence.** The producer repeats every check from the read-only evidence mount (`GAME_AUTH_NATIVE_EVIDENCE_FIRST_PUBLICATION_FENCE=required`), so a direct `publish-trust` or `publish-key` cannot bypass the guard.
- **Provenance init.** `game-auth:native-evidence:init-provenance` creates only the witness-store id and its binding row; it advances no floor.
- **Kid history.** `generate-native-admission-key.sh check-kid-history [--kid <kid>...]` is read-only and exits non-zero unless each kid has exactly one `used.kids` record.

## Early workstation testing through loopback

The safe default binds Platform and Gateway ports only to Synology loopback. From a workstation with SSH access to the NAS, forward them locally:

```bash
ssh \
  -L 8000:127.0.0.1:8000 \
  -L 8080:127.0.0.1:8080 \
  <synology-user>@<synology-host>
```

For isolated manual loopback-only development, an operator may use a separate uncommitted environment with `APP_URL=http://127.0.0.1:8000` and `SESSION_SECURE_COOKIE=false`. Those values are intentionally rejected by the guarded public staging workflow and must never be used for the canonical public Cloudflare path.

For this early loopback path, configure OTClient's Oteryn endpoints as literal `http://127.0.0.1` URLs and enable its development-only insecure-loopback option. Do not use this HTTP exception for a LAN hostname, public hostname or production environment.

## Manual local execution

The same deployment package can be executed from a trusted Linux deployment host with Docker Compose v2:

```bash
cp deploy/synology/.env.example deploy/synology/.env
# Fill staging-only values outside Git.
bash deploy/synology/scripts/deploy.sh
```

Health check:

```bash
bash deploy/synology/scripts/health-check.sh
```

Runtime-image rollback:

```bash
bash deploy/synology/scripts/rollback.sh
```

## Non-goals and blockers

- This package does not make staging evidence `PRODUCTION_PROVEN`.
- It does not configure Cloudflare certificates, WAF/Bot/Access rules, redirects or HSTS.
- It does not expose DSM, Docker Engine TCP or PostgreSQL publicly.
- It does not authorize router port forwarding or Internet exposure of game TCP.
- A fully usable desktop native-login flow additionally requires an exact compatible OTClient build and trusted browser/OAuth/Gateway endpoint configuration.


## Container supply-chain gate

The Synology build path enforces `docs/security/CONTAINER_SUPPLY_CHAIN_POLICY.json` before an affected image build can dispatch staging work.

For every allocated Platform, Game Gateway, or deploy-runner image build, CI must:

1. validate all external `FROM` and external `COPY --from` references are tag-plus-`sha256` pinned;
2. preserve the reviewed non-root final runtime user;
3. generate an SPDX JSON SBOM;
4. scan HIGH/CRITICAL OS and library vulnerabilities with Trivy;
5. fail on patchable HIGH/CRITICAL findings unless the exact component + vulnerability + package has a current bounded exception in `docs/security/CONTAINER_VULNERABILITY_EXCEPTIONS.json`;
6. generate `oteryn-container-provenance-v1` tying the exact source SHA, Dockerfile and dockerignore SHA-256, immutable base references, SBOM digest, scan digest, policy digest and final image digest together;
7. retain SBOM, vulnerability, policy and provenance evidence for 14 days as a workflow artifact.

Exceptions require review evidence, expire, and may not exceed the policy's 30-day lifetime. Expired or malformed exceptions fail closed. Blanket CVE ignores are not allowed.

Docker base-image pins are kept updateable through Dependabot Docker entries for `/deploy/synology/docker` and `/deploy/synology/runner`. Updating a digest must pass the same image build, SBOM, vulnerability and provenance checks.

This gate introduces no signing key, new registry credential or production action. Staging dispatch remains downstream of successful image-build jobs, so a failed supply-chain check blocks that release path.
