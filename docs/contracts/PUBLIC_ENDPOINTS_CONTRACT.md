# Oteryn Public Endpoints Contract

## Purpose

This document is the durable source of truth for the owner-designated public hostnames used by Oteryn Platform. Agents must not infer service roles from hostname wording, previous chat history, unrelated repositories, or ports used by other projects.

## Canonical hostname mapping

| Public hostname | Service role | Synology origin | Notes |
|---|---|---|---|
| `https://oteryn.molehill.cloud` | **Oteryn Platform web server** | `http://127.0.0.1:8000` | Public website, registration, browser login, account pages, CMS and other Platform web routes. |
| `https://login.oteryn.molehill.cloud` | **Oteryn Game Gateway / native client login API** | `http://127.0.0.1:8080` | Client-facing login/session gateway. It is not the main public website and its root path may return a technical response or `404`. |

## Mandatory interpretation

- `oteryn.molehill.cloud` means the **WWW Platform service**.
- `login.oteryn.molehill.cloud` means the **Game Gateway/login service**.
- These hostnames are not interchangeable.
- Port `3031` belongs to a different project and must never be assigned to Oteryn Platform from memory.
- The Cloudflare Tunnel origin uses Synology loopback intentionally because Platform and Gateway are bound to `127.0.0.1` by the deployment contract.

## Separate Canary endpoints

The following are not represented by the two HTTPS hostnames above:

| Endpoint | Role |
|---|---|
| `127.0.0.1:7171` | Canary legacy login rollback path; not the canonical public Game Gateway. |
| `192.168.1.2:7172` | Canary game-protocol TCP endpoint for the current private-LAN deployment. |

Do not route the website hostname to `7171`, `7172` or `8080`. Do not route the Game Gateway hostname to `8000`, `7171` or `7172`.

## Evidence and status boundary

- **PROVEN:** the Synology deployment topology binds Platform to `127.0.0.1:8000` and Game Gateway to `127.0.0.1:8080`.
- **PROVEN, owner-designated:** the two public hostname roles in this document.
- **OWNER-CONFIRMED:** `https://oteryn.molehill.cloud` is reachable from the owner's browser.
- **UNKNOWN unless separately verified:** current external reachability and endpoint behavior of `https://login.oteryn.molehill.cloud`.

This contract records naming and routing responsibility. It does not by itself prove production readiness, Cloudflare configuration correctness, external login completion, OAuth callback correctness, or public game TCP availability.
