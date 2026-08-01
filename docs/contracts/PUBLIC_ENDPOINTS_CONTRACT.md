# Oteryn Public Endpoints Contract

## Purpose

This document is the durable source of truth for the owner-designated public hostnames used by Oteryn Platform. Agents must not infer service roles from hostname wording, previous chat history, unrelated repositories, or ports used by other projects.

ADR `0020-use-single-level-gateway-public-hostname.md` records the decision to use a one-label Gateway hostname covered by the zone's existing Universal SSL certificate product.

## Canonical hostname mapping

| Public hostname | Service role | Synology origin | Notes |
|---|---|---|---|
| `https://oteryn.molehill.cloud` | **Oteryn Platform web server** | `http://127.0.0.1:8000` | Public website, registration, browser login, account pages, CMS and other Platform web routes. |
| `https://gateway.molehill.cloud` | **Oteryn Game Gateway / native client login API** | `http://127.0.0.1:8080` | Client-facing login/session Gateway. It is not the main public website and its root path may return a technical response or `404`. |

## Mandatory interpretation

- `oteryn.molehill.cloud` means the **WWW Platform service**.
- `gateway.molehill.cloud` means the **Game Gateway/native-client login service**.
- These hostnames are not interchangeable.
- `login.oteryn.molehill.cloud` is a retired legacy hostname and must not be introduced into new client, deployment or Cloudflare configuration.
- Port `3031` belongs to a different project and must never be assigned to Oteryn Platform from memory.
- The Cloudflare Tunnel origin uses Synology loopback intentionally because Platform and Gateway are bound to `127.0.0.1` by the deployment contract.

## Separate Canary endpoints

The following are not represented by the two HTTPS hostnames above:

| Endpoint | Role |
|---|---|
| `127.0.0.1:7171` | Canary legacy login rollback path; not the canonical public Game Gateway. |
| `192.168.1.2:7172` | Canary game-protocol TCP endpoint for the current private-LAN deployment. |

Do not route the website hostname to `7171`, `7172` or `8080`. Do not route the Game Gateway hostname to `8000`, `7171` or `7172`.

## Certificate boundary

`gateway.molehill.cloud` has one label below the zone apex and is eligible for the existing Universal `*.molehill.cloud` certificate coverage. Repository configuration does not itself prove that Cloudflare has issued and is presenting the expected certificate; public TLS acceptance remains mandatory after every edge migration.

The retired two-label hostname `login.oteryn.molehill.cloud` is not covered by a normal one-label wildcard and must not be restored as canonical merely from historical configuration.

## Evidence and status boundary

- **PROVEN:** the Synology deployment topology binds Platform to `127.0.0.1:8000` and Game Gateway to `127.0.0.1:8080`.
- **PROVEN, owner-designated:** the two public hostname roles in this document.
- **PROVEN, owner-approved:** `gateway.molehill.cloud` supersedes `login.oteryn.molehill.cloud` as the canonical Game Gateway hostname.
- **OWNER-CONFIRMED:** `https://oteryn.molehill.cloud` has been reachable from the owner's browser.
- **UNKNOWN unless separately verified:** current external DNS, TLS and endpoint behavior of `https://gateway.molehill.cloud`.

This contract records naming and routing responsibility. It does not by itself prove production readiness, Cloudflare configuration correctness, external login completion, OAuth callback correctness, native-client configuration, or public game TCP availability.
