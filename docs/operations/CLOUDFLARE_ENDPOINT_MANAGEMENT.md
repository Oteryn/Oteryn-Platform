# Cloudflare Oteryn endpoint management

## Purpose

`.github/workflows/cloudflare-oteryn-endpoints.yml` is the repository-controlled interface for auditing and narrowly reconciling the canonical public Oteryn endpoints.

It does not provide a general Cloudflare console or accept arbitrary account, zone, tunnel, hostname or origin values.

## Fixed endpoint contract

| Public hostname | Tunnel origin |
|---|---|
| `oteryn.molehill.cloud` | `http://127.0.0.1:8000` |
| `login.oteryn.molehill.cloud` | `http://127.0.0.1:8080` |

Both DNS records must be proxied `CNAME` records targeting:

```text
<CLOUDFLARE_TUNNEL_ID>.cfargotunnel.com
```

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
- canonical hostnames are not duplicated or path-scoped;
- the fixed hostname-to-origin rules match the repository contract;
- each canonical DNS name has either no record or exactly one non-conflicting record;
- existing DNS records use the required proxied tunnel `CNAME` target.

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

1. update the remote tunnel configuration so the two canonical hostname rules use their fixed loopback origins;
2. create a missing canonical DNS `CNAME` record;
3. update one existing canonical DNS `CNAME` record to the expected target and proxied state.

Apply does not delete DNS records, tunnels or ingress rules. It does not modify WAF, Access, TLS, certificates, Pages, Workers, cache, account membership or billing.

## Preservation and concurrency rules

Before a tunnel update, the script:

- reads the current remote configuration;
- rejects locally managed tunnels;
- rejects duplicate canonical hostname rules;
- rejects path-scoped canonical hostname rules;
- rejects a missing, duplicated or non-final pathless catch-all;
- preserves top-level tunnel configuration;
- preserves existing options on canonical rules while changing only their service target;
- preserves all unrelated ingress rules and their relative order;
- places the exact canonical rules before unrelated wildcard or path rules;
- re-reads and hashes the current configuration immediately before `PUT` and aborts if it changed.

For DNS, the script re-reads each exact hostname immediately before mutation. A non-`CNAME` record, multiple records or an API response that is not an exact match aborts the run.

There are no automatic retries for mutation requests. A transport failure after an ambiguous write must be followed by a fresh `audit`; do not assume either success or failure.

## Verification after apply

The script performs fresh tunnel and DNS reads after all requested changes. The run fails unless:

- both tunnel rules exactly match the fixed contract;
- unrelated ingress and the final catch-all remain structurally valid;
- both DNS records are proxied canonical tunnel `CNAME` records.

After a successful apply, production DNS, TLS and HTTP behavior must still be validated directly. Repository configuration and a successful API mutation do not by themselves establish `PRODUCTION_PROVEN` or satisfy the Production Go-Live Gate.

## Failure recovery

The workflow intentionally does not upload the complete tunnel configuration because unrelated routes may contain operationally sensitive topology.

If apply fails:

1. run `audit` again;
2. inspect the first failed invariant or Cloudflare API error in the job log;
3. verify the two canonical routes in the Cloudflare dashboard;
4. verify the two canonical DNS records;
5. restore any incorrect canonical value in the dashboard if immediate rollback is required;
6. do not change unrelated routes merely to make the workflow pass.

A failed apply can leave a safe partial state, such as tunnel rules updated before a DNS write fails. A subsequent audited apply is designed to converge that bounded state idempotently.

## Token rotation

When the token is rotated or replaced, update only the `CLOUDFLARE_API_TOKEN` secret in `production-cloudflare`. The account, zone and tunnel IDs remain environment variables unless the actual Cloudflare resources change.

After any token or ID change, run `audit` before `apply`.
