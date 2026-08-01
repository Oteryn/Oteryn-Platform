# Oteryn Synology staging deployment

This package targets early local/staging testing on Synology Container Manager. It is intentionally not a production deployment topology and does not satisfy the Production Go-Live Gate.

The NAS is a runtime target only. Platform, Game Gateway and the dedicated deployment-runner images are built on GitHub-hosted Actions runners. Canary must also be supplied as a compatible prebuilt image. The Synology deployment path performs image pulls, database/runtime initialization, migrations, health checks, World Registry configuration and runtime-image rollback; it does not compile C++ or Go source on the NAS.

## Runtime topology

```text
workstation/browser
        |
        | DSM reverse proxy (controlled HTTP/HTTPS ingress)
        v
Synology host loopback
  8000 -> Platform
  8080 -> Game Gateway
  7171 -> Canary legacy login rollback path

workstation/OTClient
        |
        | optional direct private-LAN game TCP only
        v
Synology exact private address
  7172 -> Canary game protocol

Docker private bridge
  Platform <-> MariaDB
  Platform <-> Redis
  Gateway -> HTTPS internal proxy -> Platform private API
  Gateway -> HTTPS internal proxy -> Canary Game Session issuer
```

The Gateway never receives database credentials. Its non-loopback dependencies use generated staging-only TLS certificates and hostname verification. MariaDB and Redis are not published on host ports. Platform, Gateway and Canary legacy login remain host-loopback-only even when game TCP is deliberately enabled for the LAN.

The user-visible Platform origin is a separate concern from the private origin binding. The guarded public staging deployment requires `APP_URL=https://oteryn.molehill.cloud` for requestless console, scheduler, notification, reset, verification and signed URL generation while continuing to bind Platform only to `127.0.0.1:8000`.

## Repository workflows

- `Build Synology Staging Images` runs on GitHub-hosted runners. Pull requests build without publishing. Trusted `main` pushes and explicit manual runs publish GHCR images tagged with `sha-<full-sha>`; `main` also receives the moving `main` tag.
- `Deploy Synology Staging` is manual only, refuses non-`main` workflow dispatches and targets only the custom runner label `oteryn-staging`.
- The deployment runner is registered with `--no-default-labels`, so generic repository jobs targeting ordinary `self-hosted` runners do not match it.
- Any temporary one-shot deployment workflow must be removed after its bounded deployment and sanitized evidence are complete.

## First-time setup order

1. Merge a reviewed version of this package so GitHub-hosted Actions can publish:
   - `ghcr.io/blakinio/oteryn-platform`;
   - `ghcr.io/blakinio/oteryn-game-gateway`;
   - `ghcr.io/blakinio/oteryn-deploy-runner`.
2. Provide a compatible prebuilt Canary image that includes the required Game Session issuer. Do not silently substitute the generic upstream image when native-auth testing is expected.
3. Register the dedicated runner on Synology using the procedure below.
4. Create the GitHub Environment `synology-staging` and configure its staging-only secrets/variables.
5. Run `Deploy Synology Staging` manually from `main`, preferably with an exact `sha-<full-sha>` Platform/Gateway image tag.

## Register the dedicated Synology runner

The one remaining non-repository bootstrap action is obtaining a short-lived repository runner registration token from:

```text
Oteryn-Platform -> Settings -> Actions -> Runners -> New self-hosted runner
```

Do not commit or paste that token into issues, PRs, logs or chat.

The supplied runner project is `deploy/synology/runner/compose.yml`. It mounts the Docker socket because its only responsibility is deploying the Oteryn staging stack. Docker-socket access is effectively host-level container control, so keep this runner repository-scoped, private and dedicated to the `oteryn-staging` label.

Before creating the Container Manager project, create a persistent state directory on Synology, for example:

```text
/volume1/docker/oteryn/state
```

Copy `deploy/synology/runner/.env.example` to a local `.env` and set the one-time `RUNNER_TOKEN`. If GHCR requires authentication to pull the private runner package, configure Container Manager with a GitHub credential that has only the package-read access needed for this pull. Do not store that credential in Git.

Create/start the runner project. When GitHub shows `oteryn-synology-staging` online with label `oteryn-staging`:

1. remove `RUNNER_TOKEN` from the Container Manager project environment;
2. restart the runner container;
3. confirm the runner returns online without the registration token.

The persistent `runner_config` volume retains the registered runner credentials. The short-lived registration token is not needed after first registration.

## GitHub Environment configuration

Create an Environment named `synology-staging`.

Required secrets:

```text
OTERYN_STAGING_APP_KEY
OTERYN_STAGING_MARIADB_ROOT_PASSWORD
OTERYN_STAGING_PLATFORM_DB_PASSWORD
OTERYN_STAGING_CANARY_DB_PASSWORD
OTERYN_STAGING_CANARY_READONLY_DB_PASSWORD
OTERYN_STAGING_CANARY_PROVISIONING_DB_PASSWORD
OTERYN_STAGING_CANARY_CHARACTER_CREATE_DB_PASSWORD
OTERYN_STAGING_REDIS_PASSWORD
OTERYN_STAGING_CANARY_RUNTIME_REDIS_PASSWORD
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN_SHA256
OTERYN_STAGING_GAME_SESSION_SERVICE_TOKEN
OTERYN_STAGING_GAME_SESSION_SERVICE_TOKEN_SHA256
```

Optional rotation secrets:

```text
OTERYN_STAGING_PLATFORM_PREVIOUS_SERVICE_TOKEN_SHA256
OTERYN_STAGING_GAME_SESSION_PREVIOUS_SERVICE_TOKEN_SHA256
```

Recommended variables:

```text
OTERYN_STAGING_STATE_DIR=/var/lib/oteryn-staging-state
OTERYN_STAGING_APP_URL=https://oteryn.molehill.cloud
```

The guarded public staging workflow fails closed when `OTERYN_STAGING_APP_URL` is set to any other value. This protects requestless user-visible links without exposing or changing the private loopback origin.

Generate independent high-entropy staging service tokens and store their exact SHA-256 digests in the matching hash secrets. The deploy script verifies both plaintext/hash pairs before changing the stack.

For database passwords used by the staging grant renderer, use random hexadecimal/alphanumeric values. The deploy script rejects unsupported characters rather than attempting unsafe SQL interpolation.

## Binding policy

The rendered deployment environment has four separate host bind controls:

```text
PLATFORM_BIND_ADDRESS=127.0.0.1
GATEWAY_BIND_ADDRESS=127.0.0.1
CANARY_LOGIN_BIND_ADDRESS=127.0.0.1
CANARY_GAME_BIND_ADDRESS=127.0.0.1
```

The first three must remain exact loopback. `CANARY_GAME_BIND_ADDRESS` may be loopback or one exact RFC1918 private IPv4 address. Wildcard, public, multicast and link-local addresses are rejected.

`CANARY_SERVER_IP` and `GAME_WORLD_HOST` must exactly equal `CANARY_GAME_BIND_ADDRESS`; `GAME_WORLD_PORT` must equal `CANARY_GAME_PORT`. This prevents Gateway from returning an unreachable or different route to OTClient.

## Deployment behavior

A `deploy` run:

1. checks out trusted `main` on the self-hosted runner;
2. logs in to GHCR with the job-scoped token;
3. validates all bind, route, image and secret inputs;
4. writes a permission-restricted ephemeral `.env` from GitHub Environment secrets;
5. validates required values and service-token hashes;
6. pulls prebuilt images;
7. starts MariaDB, Redis and internal TLS bootstrap;
8. starts Canary and waits for the required schema tables;
9. applies the repository-owned least-privilege Canary SQL grant templates without emitting `SHOW GRANTS` output;
10. configures the read-only runtime Redis ACL;
11. starts Platform, runs migrations and ensures the native OAuth client exists;
12. runs the three Canary database privilege verifiers;
13. starts the internal TLS proxy and Game Gateway;
14. verifies Platform/Gateway health, Canary TCP reachability and every exact host-port binding;
15. verifies canonical requestless Platform URLs, exact Gateway `/version` identity, bounded invalid-login response cache controls and no Platform/Gateway cross-routing;
16. when a private game bind is configured, proves the game TCP endpoint is reachable through that NAS address;
17. only after those checks, creates or updates the exact enabled online Platform World Registry route;
18. removes the ephemeral `.env` after the job.

The script snapshots currently running Platform/Gateway/Canary image references before an update. `rollback` restores those runtime images and re-runs health checks. It intentionally does **not** reverse database migrations automatically.

## Early workstation testing through loopback

The safe default binds Platform, Gateway and Canary ports only to Synology loopback. From a workstation with SSH access to the NAS, forward them locally:

```bash
ssh \
  -L 8000:127.0.0.1:8000 \
  -L 8080:127.0.0.1:8080 \
  -L 7171:127.0.0.1:7171 \
  -L 7172:127.0.0.1:7172 \
  <synology-user>@<synology-host>
```

For isolated manual loopback-only development, an operator may use a separate uncommitted environment with `APP_URL=http://127.0.0.1:8000` and `SESSION_SECURE_COOKIE=false`. Those values are intentionally rejected by the guarded public staging workflow and must never be used for the canonical public Cloudflare path.

For this early loopback path, configure OTClient's Oteryn endpoints as literal `http://127.0.0.1` URLs and enable its development-only insecure-loopback option. Do not use this HTTP exception for a LAN hostname, public hostname or production environment.

## Deliberate local-LAN game access

For the current Synology host at `192.168.1.2`, the guarded staging deployment uses:

```text
CANARY_GAME_BIND_ADDRESS=192.168.1.2
CANARY_SERVER_IP=192.168.1.2
GAME_WORLD_ID=1
GAME_WORLD_HOST=192.168.1.2
GAME_WORLD_PORT=7172
```

This exposes only Canary game TCP `192.168.1.2:7172`. Platform `8000`, Gateway `8080` and legacy login `7171` remain on `127.0.0.1`.

DSM Firewall must permit inbound TCP `7172` only from the trusted home LAN subnet or, preferably, the exact workstation address. Do not create router/NAT forwarding for `7172` as part of LAN-only testing.

The browser/OAuth and Gateway endpoints are separate HTTP surfaces. A native OTClient using a LAN hostname requires controlled HTTPS endpoints with a certificate trusted by the workstation; direct game TCP exposure alone does not configure the desktop client.

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
- It does not expose DSM, Docker Engine TCP, MariaDB, Redis or the Canary Game Session issuer publicly.
- It does not build Canary on Synology.
- It does not authorize router port forwarding or Internet exposure of game TCP.
- It does not activate legacy password authentication for Platform-created Canary accounts.
- A fully usable desktop native-login flow additionally requires an exact compatible OTClient build and trusted browser/OAuth/Gateway endpoint configuration.
