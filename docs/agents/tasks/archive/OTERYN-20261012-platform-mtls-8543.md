---
task_id: OTERYN-20261012-platform-mtls-8543
governing_issue: 1474
required_reads:
  - deploy/synology/README.md
search_first:
  - app/Http/Middleware/GameAuth/GuardNativeRuntimeStatusPeer.php
  - routes/internal.php
  - deploy/synology/compose.yml
optional_reads: []
---

# OTERYN-20261012 Platform internal mTLS listener on :8543

## Goal

Governing GitHub Issue: #1474 (native-only Platform migration). Packet P-MTLS-1 from the coordinator: the Game node calls Platform over mTLS on the Synology LAN (`192.168.1.2:8543`).

Add a Synology terminator with TLS 1.3 only and `ssl_verify_client on`. It passes `SSL_CLIENT_S_DN`, `SSL_CLIENT_VERIFY` and `SSL_PROTOCOL` to PHP over FastCGI without weakening the peer middleware, and exposes only the Game-node routes. The server chain, key and client CA are configurable read-only host mounts from the long-lived preprod CA, independent of tls-init.

## Acceptance criteria

- [x] TLS 1.3 only, client certificate mandatory, and the three SSL values reach PHP only as FastCGI parameters from the TLS session.
- [x] Only the six Game-node `POST` routes are reachable. `native-admissions` and every other path return 404.
- [x] FastCGI runs over a unix socket on a tmpfs volume shared only by Platform and the terminator; there is no TCP FastCGI port.
- [x] The server chain, key and client CA come from `PLATFORM_MTLS_*` variables. The documented staging values point into `/volume1/oteryn/preprod-platform-tls/`. The files are mounted read-only with `create_host_path: false`, and no secrets are in the repository.
- [x] The deploy, rollback and health-check scripts validate, preflight, reconcile and probe the listener.
- [x] The Game-side Platform settings for account-character push are documented only.
- [ ] Exact-head CI passes (pending on PR #1570).

## Ownership

```yaml
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/agents/tasks/archive/OTERYN-20261012-platform-mtls-8543.md
modules:
  - synology-deploy
  - game-auth
dependencies:
  - host files in /volume1/oteryn/preprod-platform-tls/ (prepared by the operator)
blockers:
  - none
cross_repository_tasks:
  - Oteryn-Game node client configuration for platform-internal.preprod.oteryn.internal:8543 (Game repository, not this task)
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-12T16:00:00Z
head: fbd636f3
branch: claude/platform-internal-mtls-8543-20261012
pr: 1570
status: completed
context_routes:
  - security
  - auth-identity
owned_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/agents/tasks/archive/OTERYN-20261012-platform-mtls-8543.md
proven:
  - The peer middleware requires SSL_CLIENT_VERIFY=SUCCESS, SSL_PROTOCOL=TLSv1.3 and an allowlisted RFC 2253 subject DN from $_SERVER; only FastCGI parameters can set these.
  - A local Docker smoke with the real nginx config and FPM pool returned 200 for a valid client, with the forged headers ignored.
  - In the same smoke, native-admissions, encoded paths, premium and /health returned 404. GET returned 403, no certificate or a rogue CA returned 400, TLS 1.2 was refused, and Platform down returned 502.
  - Codex P1 on #1570 (head a133e5ef): a Platform image without the FastCGI pool now fails health-check.sh while the listener is on. deploy.sh preflight and rollback.sh refuse such an image before any stack change. The image helper returned no for the FPM base without the pool file and yes for an image with it.
  - Codex P1 on #1570 (fast path): the fast deploy path now takes the full path when the listener's bind address, port, host paths, certificate, key, client CA or nginx config change. A fingerprint label on internal-mtls records them. Local check: same inputs gave the same digest, a key or port change gave a new one, and a missing file failed without creating it.
  - Codex P2 on #1570 (supervision): the entrypoint stays PID 1 while the pool runs and stops the container when either process exits. Local smoke: FPM killed gave exit 8, main killed gave exit 9, docker stop finished at once with 143, and no zombies remained.
  - Codex P2 on #1570 (encodings): a smoke request showed HTTP_TRANSFER_ENCODING and HTTP_CONTENT_ENCODING reaching PHP, so the bounds middleware still sees them; fastcgi_pass_request_headers on is now explicit.
  - Found in smoke: with short-syntax mounts, a fresh platform_fastcgi volume first mounted by Platform copied the image directory's owner (82:82), so nginx got EACCES and answered 502. nocopy on both mounts kept 82:101 0750, and health-check.sh now checks that the nginx user can write the socket.
  - Codex P1s on #1570 (head ca5e735b). (1) deploy.sh: an unresolved candidate whose Platform image was built before the listener is now finalized with the listener off. The decision is made, and pinned into the candidate health env, before any stack change, so later deploys no longer repeat the failure. An image that cannot be inspected fails closed. Local check: the probe returned 3 for an image without the pool, 0 for one with it, and 125 for a missing image. (2) rollback.sh runs the settings check and the full preflight before oteryn_native_admission_ingress_stop. The preflight covers the image, the bind address and nginx -t with the mounted files.
  - Further P1s in this area after this round go to the named follow-up D282, unless they are a real regression (coordinator).
  - The Compose profile keeps the bind-mounted service out of the default render, so the CI "no bind mounts" render check passes.
derived:
  - Coordinator answers 2026-10-12. (2) ssl_verify_depth 1 is correct because Game client certificates are issued directly by the preprod root (pathlen 0, no intermediates). (3) The listener stays on by default for staging. (4) The commit trailer names Opus 5.5 where the packet said Sonnet 5.5; the deviation is accepted and recorded.
  - A tls-init rotation cannot affect this listener because none of its files come from the internal_tls volume.
unknown:
  - Behaviour on the real Synology host with the operator's files; the first deploy after merge proves it through health-check.sh.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - FastCGI over TCP on the private network. Other containers could forge the SSL parameters.
  - Reusing tls-init's CA for client trust. A rotation would break Game trust.
  - Naming the service platform-mtls. The ingress trust-gate test pattern reads "up -d ... platform" as a Platform start.
changed_paths:
  - deploy/synology/compose.yml
  - deploy/synology/native-defaults.env
  - deploy/synology/.env.example
  - deploy/synology/README.md
  - deploy/synology/docker/platform.Dockerfile
  - deploy/synology/docker/platform-entrypoint.sh
  - deploy/synology/nginx/internal-mtls.conf
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/rollback.sh
  - deploy/synology/scripts/health-check.sh
  - deploy/synology/tests/test_platform_mtls_contract.py
  - docs/agents/tasks/archive/OTERYN-20261012-platform-mtls-8543.md
validation:
  - command: python3 deploy/synology/tests/test_platform_mtls_contract.py
    result: PASS
    evidence: 12 tests
  - command: CI-listed Synology python tests and container_supply_chain validate-repository
    result: PASS
    evidence: all pass, rc 0
  - command: python3 -m pytest -q tests/ci
    result: PASS
    evidence: 317 passed
  - command: python3 tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
blockers:
  - none
next_action: "named follow-up synology-mtls-contract-test-ci-wiring (control plane, coordinator answer 1 on 2026-10-12): run deploy/synology/tests/test_platform_mtls_contract.py in build-synology-staging-images.yml and synology-rollback-contract.yml"
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository feature branch with no retention purpose after merge
source_branch_evidence: same-repository PR #1570 from this branch; the repository setting delete_branch_on_merge is true (REST repos/Oteryn/Oteryn-Platform), so a merge removes the ref; confirm it is absent after merge
```
