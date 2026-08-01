# Production Go-Live Gate evidence — 2026-08-01

## Verdict

```text
BLOCKED — PENDING PRODUCTION VERIFICATION
PRODUCTION_PROVEN=false
```

Issue #91 must remain open. No repository, staging, local-target or public-edge observation below establishes full production readiness.

## Evaluated repository state

- task branch base: `de949075d14ebecc57423237b9330d865da28645`;
- task branch: `agent/production-go-live-gate`;
- draft PR: #405;
- Cloudflare fixed-scope audit: run `30699270139`;
- Cloudflare fixed-scope apply: run `30700054602`;
- Cloudflare-managed Tunnel/DNS state after apply: `current`;
- no Cloudflare, DNS, Synology runtime, database, Redis, secret, deployment, rollback, restore or application-data mutation was performed by this task.

The repository/task SHA is not the deployed application identity.

## PROVEN

### Public edge

Independent post-Cloudflare observation:

- workflow run: `30701140509`;
- job: `91372237869`;
- observed at: `2026-08-01T13:09:16.214513+00:00`;
- artifact: `8818850803`;
- artifact digest: `sha256:787ea72c616812ade431eb1cc396e921a6c8b04e459c89557221cbf6caebe656`;
- runner region: West US.

Direct observations:

- both canonical hostnames resolved to the same Cloudflare IPv4/IPv6 anycast set;
- `oteryn.molehill.cloud` verified only with TLS 1.3 and presented the expected wildcard-domain certificate;
- all representative HTTPS WWW routes returned Cloudflare `403` challenge content instead of Oteryn Platform;
- `login.oteryn.molehill.cloud` failed TLS 1.2 and TLS 1.3 before HTTP and exposed no certificate;
- plain HTTP on both canonical names returned Cloudflare `403` rather than redirecting to HTTPS;
- WWW returned `Strict-Transport-Security: max-age=0; includeSubDomains; preload`.

This directly blocks public launch and prevents application-level end-to-end smoke through the canonical endpoints.

### Synology read-only preflight

- observer run: `30701433548`;
- dispatched trusted-main preflight: `30701440189`;
- live self-hosted runner job: `91373030006`;
- database restore input: `false`;
- first failure: `mariadb does not use restart policy unless-stopped`;
- the run stopped before upload of a sanitized runtime inventory;
- no restore or runtime configuration mutation ran.

The actual MariaDB restart-policy value was not printed and remains `UNKNOWN`. Open PR #335 proposes `restart: always`, but an open PR is not direct evidence of the running container value.

### Repository validation

On PR #405 head `0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7`:

- Agent Governance run `30701773251`: PASS;
- CI run `30701773237`: PASS;
- Synology Production Target Preflight static run `30701773212`: PASS;
- Edge Security Emulation run `30701773227`: PASS;
- Game Auth Ticket Concurrency run `30701773203`: PASS;
- Platform DB Outage Validation run `30701773233`: PASS;
- Phase 7 Production-Like Validation run `30701773198`: PASS;
- Build Synology Staging Images run `30701773188`: PASS.

These results are repository or staging-support evidence only.

## DERIVED

- Cloudflare Tunnel/DNS convergence did not resolve the separately controlled certificate, WAF/Bot/Access, redirect or HSTS failures.
- A `403` Cloudflare challenge is not evidence that Platform routes are healthy or correctly routed.
- The public Game Gateway cannot be smoke-tested while TLS negotiation fails.
- Mutation smoke is unsafe and prohibited while exact deployed identity, rollback, dated production restore, controlled identities/data, and application reachability remain unproven.
- The final result cannot be `PRODUCTION_PROVEN`.

## UNKNOWN

- exact currently deployed Platform source SHA, tag, image ID and repository digest;
- exact currently deployed Game Gateway source SHA, tag, image ID and repository digest;
- exact currently deployed Canary image/revision for the selected launch scope;
- actual MariaDB restart policy and the full effective restart/health/restart-count inventory;
- current Compose/stack source and deployment timestamp;
- whether cloudflared is a host process or container and its effective network mode;
- application-origin reachability through the canonical public hostnames;
- effective production application configuration verifier result;
- production DB topology, effective grants, backup policy and dated restore evidence;
- production Redis/session/cache/queue topology and monitoring;
- production mail delivery and bounce monitoring;
- centralized logs, metrics, alerts and on-call ownership;
- actual deployment, migration and emergency rollback mechanism;
- launch-scope decisions and controlled smoke identities/data;
- all mutation-authorized critical production smoke results.

A trusted branch-defined sanitized inventory was dispatched through observer run `30701773214`. At this checkpoint it remained queued on the Synology runner and therefore supplies no evidence yet.

## CONFLICT

- archived Synology preflight policy requires `unless-stopped`;
- the current live preflight directly proved that MariaDB does not satisfy that expectation;
- open PR #335 proposes `restart: always`, but the effective running value and approved target policy are not yet directly reconciled.

## Blockers and ownership

| Blocker | Responsible operator |
|---|---|
| WWW returns Cloudflare challenge instead of Platform | Cloudflare zone/security-policy operator |
| Game Gateway hostname has no usable TLS | Cloudflare zone/certificate operator |
| HTTP does not redirect to HTTPS and HSTS is disabled | Cloudflare zone/TLS operator |
| Exact running release and container topology not captured | Synology runtime operator / queued read-only runner |
| Backup/restore, rollback, mail, observability and smoke prerequisites absent | Production owner/operator |
| Launch-scope and controlled mutation-smoke inputs unresolved | Repository owner |

## Single next action

Complete and inspect the already-dispatched sanitized Synology inventory associated with observer run `30701773214`; do not mutate runtime state. After its exact release/topology evidence is persisted, the separately authorized edge operator must address the proven TLS/WAF/redirect/HSTS blockers before any application mutation smoke.
