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
