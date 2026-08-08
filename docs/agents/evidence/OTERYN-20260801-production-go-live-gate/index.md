# Production Go-Live Gate evidence — 2026-08-01

## Evidence lifecycle

`HISTORICAL DIRECT OBSERVATION — SUPERSEDED FOR CURRENT EDGE STATE`

This index preserves the sanitized direct evidence collected on **2026-08-01**. It is not a current production-state snapshot and must not be resumed as a privileged operations checklist.

The historical verdict remains:

```text
BLOCKED — PENDING PRODUCTION VERIFICATION
PRODUCTION_PROVEN=false
```

Issue #91 remains the durable current Production Go-Live Gate. A future production verdict requires fresh direct evidence tied to one exact deployed production release.

## Evaluated historical repository/runtime state

- task branch base: `de949075d14ebecc57423237b9330d865da28645`;
- historical branch: `agent/production-go-live-gate`;
- historical PR: #405;
- no Cloudflare, DNS, Synology runtime, database, Redis, secret, deployment, rollback, restore or application-data mutation was performed by this evidence collection.

The task branch SHA was not the deployed application identity.

## PROVEN on 2026-08-01

### Synology staging runtime identity and topology

Sanitized read-only inventory:

- dispatched inventory run: `30701775782`;
- live inventory job: `91373911925`;
- observed at: `2026-08-01T13:38:45.781133+00:00`;
- observer source SHA: `0c435dd02d2afcc7f0e8d963a79b5441b29a6cb7`;
- artifact: `8819161257`;
- artifact digest: `sha256:67b1d16eb67f90e1534a9071644aeaf42da97adc527643964a14df120c37db9c`;
- production mutation: `NONE`;
- restore drill: `NOT_RUN`.

Observed images:

| Service | Historical source/tag | Repository digest |
|---|---|---|
| Platform | `3eb109b505f7d1c8718cffb823de6d9d5166717c` | `sha256:ac0e88a1627a8ab78b4bca87dbce16035c29da8f3ab01c152fe2bf0946651b7a` |
| Game Gateway | `3eb109b505f7d1c8718cffb823de6d9d5166717c` | `sha256:9a731ca6528faec0ae70106898b757a22a6e561d99efc639d1fe71e5ece687ed` |
| Canary | immutable digest reference | `sha256:784e5dbdcc64e311c48c51cd94aa206e2efa1e5eefb2f4ef40170d5aac55031f` |

The observed Compose project was `oteryn-staging`; all six expected service containers were running. MariaDB and Redis were healthy and bounded Platform/Gateway container/host-loopback probes passed.

Historical effective profile:

- `APP_ENV=staging`;
- debug disabled;
- file sessions;
- file cache;
- synchronous queue;
- array/non-delivery mail;
- `production:verify-configuration` exit code `1`;
- runtime classification `STAGING_TARGET`;
- production environment proven `false`.

No cloudflared container was visible through Docker. Host-process/network-path state was not proven by that observation.

### Managed Tunnel/DNS scope

- audit run `30699270139`: PASS;
- apply run `30700054602`: PASS;
- managed Tunnel/DNS state after apply: `current`.

This proved only that guarded scope at that time.

### Public edge observation at that time

Independent run `30701140509`, job `91372237869`, artifact `8818850803`, digest `sha256:787ea72c616812ade431eb1cc396e921a6c8b04e459c89557221cbf6caebe656` observed:

- WWW returning Cloudflare 403 challenge content instead of Platform;
- Game Gateway TLS failing before HTTP;
- plain HTTP returning 403 rather than redirecting to HTTPS;
- WWW HSTS `max-age=0`.

These are **historical observations**, not current blockers.

## Later evidence superseding the edge-generation facts

Protected-main PR #516 later closed the separately authorized Cloudflare edge programme and records:

- guarded HSTS apply run `30855934824` reaching `state=staged`, `max_age=2592000`;
- complete public E2E PASS and `positive_hsts_www=true`;
- independent trusted-main audit run `30857136575` reproducing the staged target with `desired_state=true` and `mutation=none`;
- stable WAF/Bot repair with the canonical skip rule first and Bot Fight Mode false;
- terminal archival of the Cloudflare edge task.

Therefore this evidence index must not claim that the August 1 WWW-403, Gateway-TLS, redirect or HSTS state is still current, and it must not retain the old “request Cloudflare zone-edge audit/apply” action as a current next step.

Open PR #541 contains later public-domain reconciliation that also treats the edge/HSTS repair as complete, but because it remains unmerged it is work-in-progress corroboration rather than protected-main authority.

Issue #877 owns residual Cloudflare verification-evidence reconciliation. This historical PR does not duplicate that work.

## Historical DERIVED conclusions

The following were valid deductions from the August 1 observation generation:

- the exact observed runtime was staging and could not be `PRODUCTION_PROVEN`;
- local loopback health did not substitute for public application delivery;
- array mail blocked real production password-recovery delivery in that observed staging profile;
- production mutation smoke was unsafe without a proven production runtime, rollback, restore, controlled identity/data and public reachability;
- the August 1 evidence could not produce `PRODUCTION_PROVEN`.

They do not prove the current value of those runtime/configuration facts.

## Current UNKNOWN boundary

This historical evidence does not prove today's:

- exact production Platform/Gateway/game release;
- Synology production topology or current container state;
- cloudflared host/network path;
- production DB/Redis grants/topology/backup/dated restore;
- session/cache/queue choice;
- production mail provider and sender-domain readiness;
- logs/metrics/alerts/on-call ownership;
- deployment/migration/emergency rollback mechanism;
- launch scope and controlled production smoke;
- any public-edge fact not covered by a later current evidence package.

Do not copy the historical UNKNOWN set blindly into a future verification; rediscover the current scope from then-current canonical documents and environment evidence.

## Current authority and next action

Issue #91 remains authoritative and `PRODUCTION_PROVEN=false` until fresh exact-release production evidence satisfies it.

There is no current Cloudflare mutation next action in this index.

If a new production attempt is authorized, create a fresh current-main bounded verification task that starts with read-only discovery and requests any privileged mutation only after a current exact failure proves it necessary and the owner approves the exact change/rollback.

PR #405 should close unmerged as a superseded historical evidence branch. This index remains recoverable from the closed PR/branch and Git history without reintroducing stale operational instructions into current `main`.

## Safety

Issue #885 performs documentation/lifecycle reconciliation only. It authorizes no Cloudflare/Synology/environment/secret/deployment/database/Redis/restore/smoke/production or external-repository mutation.
