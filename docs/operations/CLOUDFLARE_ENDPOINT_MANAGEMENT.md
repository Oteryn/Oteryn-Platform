# Cloudflare Oteryn endpoint management

## Purpose

`.github/workflows/cloudflare-oteryn-endpoints.yml` is the repository-controlled interface for auditing and narrowly reconciling the canonical public Oteryn endpoints.

It does not provide a general Cloudflare console or accept arbitrary account, zone, tunnel, hostname or origin values.

## Fixed endpoint contract

| Public hostname | Tunnel origin |
|---|---|
| `oteryn.molehill.cloud` | `http://127.0.0.1:8000` |
| `gateway.molehill.cloud` | `http://127.0.0.1:8080` |

Both DNS records must be proxied `CNAME` records targeting:

```text
<CLOUDFLARE_TUNNEL_ID>.cfargotunnel.com
```

`login.oteryn.molehill.cloud` is the retired legacy Gateway hostname. ADR 0020 records the owner-approved move to the single-level hostname so existing Universal `*.molehill.cloud` certificate coverage can apply without Advanced Certificate Manager.

The durable service-role contract remains `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md` and `deploy/synology/PUBLIC_ENDPOINTS.md`.

## GitHub environment

The workflow uses the protected environment:

```text
production-cloudflare
```

Required environment secret:

```text
CLOUDFLARE_API_TOKEN
```

Required environment variables:

```text
CLOUDFLARE_ACCOUNT_ID
CLOUDFLARE_ZONE_ID
CLOUDFLARE_TUNNEL_ID
```

The token must be limited to the intended Cloudflare account and the `molehill.cloud` zone with only:

- Cloudflare Tunnel read/write capability;
- DNS read/write capability.

Do not use a Global API Key. Do not add the token to repository variables, files, comments, logs or pull-request text.

The environment must allow deployments from `main` only. Enabling a required reviewer is recommended when every production edge mutation should require a separate approval in GitHub.

## Workflow modes

### Audit

Run:

1. GitHub repository `Actions`;
2. `Cloudflare Oteryn Endpoints`;
3. `Run workflow`;
4. branch `main`;
5. mode `audit`;
6. leave confirmation empty.

Audit performs no write request. It verifies:

- the API token is active;
- the supplied account and tunnel IDs resolve to the same remotely managed tunnel;
- the tunnel configuration has exactly one pathless final catch-all rule;
- canonical and retired hostnames are not duplicated or path-scoped;
- the fixed hostname-to-origin rules match the repository contract;
- each canonical DNS name has either no record or exactly one non-conflicting record;
- existing canonical DNS records use the required proxied tunnel `CNAME` target;
- a legacy `login.oteryn.molehill.cloud` record is either absent or exactly one `CNAME` to the same managed tunnel target.

Drift is reported in the workflow summary. Audit returns success when the API state is safely readable even if bounded drift exists. Ambiguous or unsafe state fails closed.

### Apply

Apply is available only after the workflow is merged to `main`.

Run the same workflow with:

```text
mode: apply
confirmation: APPLY-OTERYN-CLOUDFLARE
```

The confirmation phrase is checked by both the workflow and the script.

Apply can perform only these changes:

1. update the remote Tunnel configuration so the two canonical hostname rules use their fixed loopback origins;
2. create a missing canonical DNS `CNAME` record;
3. update one existing canonical DNS `CNAME` record to the expected target and proxied state;
4. remove the retired `login.oteryn.molehill.cloud` ingress rule while preserving all unrelated ingress;
5. delete the retired DNS record only when it is exactly one `CNAME` pointing to the same Oteryn tunnel target.

Apply does not delete arbitrary DNS records, tunnels or unrelated ingress rules. It does not modify WAF, Access, TLS, certificates, Pages, Workers, cache, account membership or billing.

## Edge maintenance gate

The gate closes the public edge during the PostgreSQL cutover (design: `PLATFORM_POSTGRES_MAINTENANCE_GATE_DESIGN.md`). Closing switches exactly the two canonical ingress rules (`oteryn.molehill.cloud`, `gateway.molehill.cloud`) to `http_status:503`; every other rule and field is preserved verbatim, in order, including the final catch-all. The tunnel is shared with unrelated services, so their rules are never deleted, altered or reordered by the gate, and their hostnames, services and ports are never printed. Both modes run only from `main` through `workflow_dispatch`, in the `production-cloudflare` environment, and are never available from marker or comment triggers.

State is classified from the live tunnel configuration as `open`, `closed`, `split` (one rule closed) or `unknown` (anything else). Audit reports `gate_state`, `edge_version`, `other_origin_routes` (an informational count of other rules that route to an origin), `oteryn_bypass_rules` (a count, below), `preserved_rules` (the count of rules other than the two canonical ones) with `preserved_rules_hash` (SHA-256 of the canonical JSON of the configuration with the two canonical rules removed, so it is identical open and closed) and `closure_match` (`not_requested`, `yes` or `no`), and uploads the same fields as `audit.json` (schema `oteryn-edge-audit-result/v2`) in the `oteryn-edge-audit-result` artifact (30 days).

### maintenance-close

Inputs: `confirmation=CLOSE-OTERYN-EDGE`, `task_id` (`OTERYN-YYYYMMDD-slug`), optional `closure_run_id`.

- Other rules do not block the close: they are preserved verbatim and `other_origin_routes` is informational. Refused only unless `oteryn_bypass_rules` is 0, that is no other rule may reach or shadow the Oteryn origins while the canonical rules return 503: a service on port 8000 or 8080 of any host (or an unparseable service), a hostname that is a canonical hostname with a port or other spelling, or a rule placed before a canonical rule whose hostname is empty, `*` or a wildcard that matches a canonical hostname (ingress is first-match). Hosts that must stay on those ports are listed in `ALLOWED_ORIGIN_HOSTS` in the script (empty by default). A refusal prints the count only.
- The planned configuration must have the same preserved-rules hash as the read it is built from, and the read-back after the PUT must show the same preserved count and hash; otherwise the close fails.
- From `open`, the script re-reads the edge immediately before the PUT, aborts if its hash, version or classification changed, writes once, reads back, and requires the version to have increased by exactly one.
- From `closed`, the run is a renewal and requires `closure_run_id` of a verifiable earlier closure that still matches the live edge version and hash. The recorded origin fingerprint is carried forward.
- The result is the `oteryn-edge-gate-closure` artifact (30 days) holding `closure.json`: schema, `task_id`, `close_run_id`, `run_attempt`, `edge_version`, `config_hash`, `preserved_rules_count`, `preserved_rules_hash`, `fingerprint_status`, `origin_fingerprint` (schema `oteryn-edge-gate-closure/v2`; a v1 closure is not accepted). Any later edit of the tunnel, even identical rules, raises the version and invalidates the closure.
- A secret-free `edge-fingerprint` job records which cookie and header names the application sends. The secret-free `edge-probe` job then requires both canonical hosts to answer 503 from the edge and a random synthetic hostname not to return the recorded application fingerprint. Without a usable fingerprint the wildcard probe is informational only.

### maintenance-open

Inputs: `confirmation=OPEN-OTERYN-EDGE`, `task_id`, `closure_run_id`, `evidence_run_id`.

- Allowed only from `closed`; `open` is an idempotent no-op that needs no ids. `split` and `unknown` are refused.
- Provenance is read through `gh api`: the closure run must be a successful `cloudflare-oteryn-endpoints.yml` dispatch on `main` at attempt 1 with exactly one unexpired closure artifact that matches the live version, hash and preserved-rules hash. After the PUT the live preserved-rules hash must still equal the closure's, so the open restores the canonical rules and nothing else. The evidence run must be a `deploy-synology-staging.yml` dispatch on `main` at attempt 1 that is in progress or successful, with `platform-cutover-gate-evidence` whose `task_id`, `close_run_id` and `outcome` (`cutover_complete` or `rollback_verified`) match.
- A failed closing run is accepted for renewal only when `manage` succeeded, `edge-probe` failed and every other job succeeded or was skipped.

### Serialization and apply

`apply`, `maintenance-close` and `maintenance-open` writes share the job-level concurrency group `cloudflare-oteryn-edge-write` (no cancellation). Audit, fingerprint and probe jobs use per-run groups. Before any `apply`, a read-only `guard` job fails while a gate run (display title `Cloudflare Oteryn Endpoints [maintenance-close|open]`) is queued, waiting, pending or in progress in either endpoint workflow.

`apply` itself now refuses unless `gate_state=open`, at script level and before any DNS or tunnel write, so it cannot reopen a closed gate or run against an unrecognized edge. Repair of an `unknown` edge is done in the Cloudflare dashboard, then audit and apply are run again.

Break-glass: if the gate cannot be opened through the workflow, the owner restores the two rules to `http://127.0.0.1:8000` and `http://127.0.0.1:8080` in the Cloudflare dashboard. This raises the version and invalidates any closure, so record the action on the task and rerun audit.

## Migration order and preservation rules

Before a Tunnel update, the script:

- reads the current remote configuration;
- rejects locally managed tunnels;
- rejects duplicate managed hostname rules;
- rejects path-scoped managed hostname rules;
- rejects a missing, duplicated or non-final pathless catch-all;
- preserves top-level Tunnel configuration;
- preserves existing options on canonical rules;
- carries safe options from the old Gateway ingress into the renamed Gateway rule;
- preserves all unrelated ingress rules and their relative order;
- places the exact canonical rules before unrelated wildcard or path rules;
- re-reads and hashes the current configuration immediately before `PUT` and aborts if it changed.

For DNS, the script re-reads each exact hostname immediately before mutation. A non-`CNAME` record, multiple records or an API response that is not an exact match aborts the run.

The new canonical Tunnel and DNS state is verified before legacy DNS deletion. A legacy record is deleted only when its exact name, type and tunnel target match the bounded migration contract. A legacy record pointing elsewhere is never overwritten or deleted automatically.

There are no automatic retries for mutation requests. A transport failure after an ambiguous write must be followed by a fresh `audit`; do not assume either success or failure.

## Verification after apply

The script performs fresh Tunnel and DNS reads after all requested changes. The run fails unless:

- both canonical Tunnel rules exactly match the fixed contract;
- the retired legacy ingress is absent;
- unrelated ingress and the final catch-all remain structurally valid;
- both canonical DNS records are proxied tunnel `CNAME` records;
- the retired legacy DNS record is absent.

After a successful apply, production DNS, TLS and HTTP behavior must still be validated directly. Repository configuration and a successful API mutation do not by themselves establish `PRODUCTION_PROVEN` or satisfy the Production Go-Live Gate.

## Failure recovery

The workflow intentionally does not upload the complete Tunnel configuration because unrelated routes may contain operationally sensitive topology.

If apply fails:

1. run `audit` again;
2. inspect the first failed invariant or Cloudflare API error in the job log;
3. verify the two canonical routes in the Cloudflare dashboard;
4. verify the two canonical DNS records;
5. verify whether the retired legacy record still exists;
6. restore an incorrect canonical value in the dashboard only when immediate rollback is required;
7. do not change unrelated routes merely to make the workflow pass.

A failed apply can leave a safe partial state, such as the new Gateway route created while the old DNS record still exists. A subsequent audited apply is designed to converge that bounded state idempotently.

Rollback may recreate the old ingress and proxied CNAME to the same tunnel target, but it does not provide a valid certificate for the retired multi-level hostname. It restores routing state only.

## Token rotation

When the token is rotated or replaced, update only the `CLOUDFLARE_API_TOKEN` secret in `production-cloudflare`. The account, zone and tunnel IDs remain environment variables unless the actual Cloudflare resources change.

After any token or ID change, run `audit` before `apply`.
