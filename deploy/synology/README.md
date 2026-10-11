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

Docker private bridge
  Platform <-> MariaDB
  Platform <-> Redis
  Gateway -> HTTPS internal proxy -> Platform private API
```

The Gateway never receives database credentials. Its non-loopback dependencies use generated staging-only TLS certificates and hostname verification. MariaDB and Redis are not published on host ports. Platform and Gateway remain host-loopback-only.

The user-visible Platform origin is a separate concern from the private origin binding. The guarded public staging deployment requires `APP_URL=https://oteryn.molehill.cloud` for requestless console, scheduler, notification, reset, verification and signed URL generation while continuing to bind Platform only to `127.0.0.1:8000`.

## Repository workflows

- `Build Synology Staging Images` runs on GitHub-hosted runners. Pull requests build without publishing. Trusted `main` pushes and explicit manual runs publish GHCR images tagged with `sha-<full-sha>`; `main` also receives the moving `main` tag.
- `Deploy Synology Staging` is `workflow_dispatch`-only at its own boundary, refuses non-`main` dispatches and runs through organization runner group `platform-runners` with custom label `oteryn-platform`; guarded trusted-main build automation may dispatch it with exact release inputs.
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
OTERYN_STAGING_MARIADB_ROOT_PASSWORD
OTERYN_STAGING_PLATFORM_DB_PASSWORD
OTERYN_STAGING_REDIS_PASSWORD
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN
OTERYN_STAGING_PLATFORM_SERVICE_TOKEN_SHA256
```

Optional rotation secrets:

```text
OTERYN_STAGING_PLATFORM_PREVIOUS_SERVICE_TOKEN_SHA256
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

`GAME_WORLD_HOST` is the Platform-owned route returned to OTClient; it must be loopback or one exact RFC1918 private IPv4 address. Wildcard, public, multicast and link-local addresses are rejected.

## Deployment behavior

A `deploy` run:

1. checks out trusted `main` on the self-hosted runner;
2. logs in to GHCR with the job-scoped token;
3. validates all bind, route, image and secret inputs;
4. writes a permission-restricted ephemeral `.env` from GitHub Environment secrets;
5. validates required values and service-token hashes;
6. pulls prebuilt images;
7. starts MariaDB, Redis and internal TLS bootstrap;
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

Native admission is opt-in. Set the repository variable `OTERYN_STAGING_NATIVE_ADMISSION_ENABLED=true` only together with `OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES` and `OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES` (the Game identities that report runtime status and own scope assignment; without them no scope is routable); deploy refuses otherwise. Rollback enables it for an image only if that image has both the self-check command and the entrypoint key export.

Account-character projection has no producer on Synology, so `GAME_AUTH_NATIVE_ACCOUNT_CHARACTERS_ENABLED` is `false` and the D171 fallback is selected for the one native World: set the repository variable `OTERYN_STAGING_NATIVE_WORLD_ID` to that World's UUIDv7; deploy refuses without it.

Rollback of a Platform image that predates the self-check sets `OTERYN_NATIVE_ADMISSION_ENABLED=false` for that image only (both flags stay paired), so the older release restores with native login off.

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
- It does not expose DSM, Docker Engine TCP, MariaDB, Redis publicly.
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
