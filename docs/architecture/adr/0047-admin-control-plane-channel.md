# ADR 0047 — Admin control plane channel

## Status

Proposed — 2026-11-05

- Decision owner: repository owner
- Programme: `OTV2-ADMIN-CONTROL-PLANE`; task ADM-ADR
- Applies to: the Rust portal in `portal-rs/`
- Does not authorize: deployment, production enablement, certificate issuance, secret changes or execution of Game operations.

## Context

The owner programme calls for one Platform console for Game operations and analytics while Game retains authority over runtime state, nodes, hosts and its database. Platform owns operator identity, authorization, approvals, durable requests and presentation. Platform never initiates a connection to Game and receives no Game database credentials or shell access.

The normative inputs are the following files in `Oteryn/Oteryn-Game`, pinned to commit `7047e5d45af3e16e4cf49ab925c4f1e0bde8ef03` (Game PR #2992, merged):

- [ADR-0023](https://github.com/Oteryn/Oteryn-Game/blob/7047e5d45af3e16e4cf49ab925c4f1e0bde8ef03/docs/architecture/ADR-0023-admin-control-plane-and-analytics-console.md): accepted direction.
- [ADM-01](https://github.com/Oteryn/Oteryn-Game/blob/7047e5d45af3e16e4cf49ab925c4f1e0bde8ef03/docs/architecture/ADM-01_ADMIN_CONTROL_PLANE_CONTRACT.md): normative wire contract `oteryn.admin-control.v1`.
- [ADM-02](https://github.com/Oteryn/Oteryn-Game/blob/7047e5d45af3e16e4cf49ab925c4f1e0bde8ef03/docs/architecture/ADM-02_ADMIN_CONSOLE_DESIGN_AND_DELIVERY_PLAN.md): console and delivery plan.

ADM-02's Laravel/Blade description is historical Platform context. Implementation follows `../OTERYN_PORTAL_RUST_PLAN.md` §§1–4, 7, 8 and 10: axum, askama, PostgreSQL 18.6 in Platform's separate instance, existing RBAC in `portal-rs/crates/admin/src/rbac`, and the existing internal mTLS guard in `portal-rs/crates/internal-api/src/guard.rs`. This proposal introduces no Laravel implementation.

## Decision

### 1. Game-initiated channel and identity boundary

Adopt these Platform endpoints on the existing internal mTLS ingress, all using POST:

| Path | Purpose |
| --- | --- |
| `/internal/v1/admin-control/hello` | Agent session, build, catalog digest, operation versions, scope and ownership epoch |
| `/internal/v1/admin-control/channel` | Agent receives pending requests; long-poll at most 25 seconds, batch at most 32 |
| `/internal/v1/admin-control/results` | Accepted, progress and signed final results |
| `/internal/v1/admin-control/stream` | Large result continuation, at most 1 MiB per chunk and 64 chunks |
| `/internal/v1/admin-control/rollups` | Game-computed analytics cache updates |
| `/internal/v1/admin-control/heartbeat` | Agent/node/database/backup health every 10 seconds |

Only the dedicated agent identity is allowlisted: certificate CN `oteryn-admin-agent/<agent_id>`, with SAN scope for served worlds and hosts. A valid certificate for another internal purpose grants no channel access. Bind the verified peer to the registered agent and its allowed scope; never derive transport identity from request headers or body claims. Reuse the guard's purpose-specific identity allowlist and trusted TLS connection adapter. The current guard represents verified SAN or SPKI identities, so delivery must explicitly validate the required CN and bind it to that verified identity; a body `agent_id` alone cannot bridge this difference.

Each world/node has exactly one owning agent and `owner_epoch`; stale ownership is refused with `ADM_AGENT_SUPERSEDED`. An agent is connected only while its last channel poll or heartbeat is younger than 30 seconds. Missing or ambiguous identity, ownership, capabilities or policy denies. An operation/version absent from the connected agent's hello is **unavailable**, never emulated. Platform never opens a connection to Game; responses to the agent's channel carry requests. Transport upgrades must preserve this boundary and the existing guard's verified transport guarantees.

### 2. Signed envelopes and key custody

Every query and command uses the ADM-01 envelope. The Platform admin signer uses Ed25519 over RFC 8785 JCS canonical bytes of the entire envelope **without the `signature` field**, identified by `platform-admin-signer-<n>`. Private keys exist only in Platform secret storage; Game receives public keys and validity windows in its startup-pinned policy. No secret crosses the channel.

At most two keys are active/trusted at once (current and next). Follow ADM-01 §1 rotation in order: add the next public key to Game policy, restart the agent, switch the Platform signer, then remove the old key from Game policy. Unknown or expired keys fail `ADM_SIGNER_UNTRUSTED`. Preserve the signed bytes and signer id as evidence; this signer is a separate administrative purpose from admission signing.

Game independently enforces ADM-01 §3 verification order: advertised operation/version, trusted signature, issuance/expiry bounds, nonce replay protection, closed parameter schema, Game permission/risk policy, control evidence, operation preconditions and idempotency. Platform cannot lower Game's minimum risk or bypass its per-world enablement. Closed schemas reject unknown fields. Integer encoding, lifetime limits, nonce retention and error codes follow ADM-01 without reinterpretation.

### 3. Durable request store and lifecycle

Plan Platform-owned PostgreSQL stores `admin_control_requests`, `admin_control_results`, `admin_control_agents`, `admin_approvals` and `analytics_cache`; future migrations deliver them separately. Persist the envelope, target/owning agent, timestamps, canonical parameter digest, request and operation ids, approvals, delivery state, progress and signed results. Queue creation and its audit evidence must be transactional. Browser forms write requests through authorized Platform handlers; browsers poll Platform's stored results for progress.

Lifecycle is `queued → delivered → accepted/in_progress → final`, where final is `succeeded`, `failed`, `rejected`, `expired` or `cancelled` as specified by ADM-01. A disconnected agent leaves a request `queued`; at `expires_at`, `queued → expired`. Show expiry explicitly and never silently retry with a new id. Delivery is not proof of execution; ambiguous delivery or outcome remains visible and must be reconciled using the original evidence, never treated as success.

Retain `request_id` (UUIDv7 per attempt chain) and `admin_operation_id` (command idempotency key; equal to request id for queries). Same operation id and canonical parameter digest converge on the recorded result; another digest conflicts. Transport recovery cannot create a new business operation. No execution is valid after expiry. Cancellation is the typed `admin.request.cancel` operation at declared safe points, with the final result recording how far execution got.

### 4. Closed permissions and roles

Extend the existing Rust closed permission list with exactly the ADM-01 §8 constants:

```text
game.ops.read, game.ops.write, game.ops.logs.read,
game.world.read, game.world.ops, game.world.config, game.world.events, game.world.admin,
game.node.read, game.node.ops, game.node.admin, game.node.deploy,
game.db.read, game.db.backup, game.db.query, game.db.admin,
game.player.read, game.player.moderate, game.player.manage, game.player.value.write,
game.economy.read, game.economy.moderate,
game.sanction.read, game.sanction.write, game.sanction.ban, game.staff.admin,
game.analytics.read, game.analytics.player, game.analytics.sql, game.analytics.relink,
game.analytics.export, game.audit.read
```

Adopt these role templates. Inheritance below means explicit finite grants at implementation time, never runtime wildcard matching. `platform_admin` is not a wildcard and receives no implicit new permission.

| Role | Explicit grant template |
| --- | --- |
| `game_viewer` | `game.ops.read`, `game.ops.logs.read`, `game.world.read`, `game.node.read`, `game.db.read`, `game.economy.read`, `game.sanction.read`, `game.analytics.read`, `game.audit.read` |
| `game_support` | viewer plus `game.player.read`, `game.player.moderate`, `game.sanction.read`, `game.sanction.write` |
| `game_master_senior` | support plus `game.player.manage`, `game.player.value.write`, `game.sanction.ban`, `game.world.events`, `game.economy.moderate`, `game.economy.read` |
| `game_operator` | viewer plus `game.world.ops`, `game.world.config`, `game.node.ops`, `game.db.backup`, `game.ops.write`, `game.ops.logs.read` |
| `game_admin` | operator plus `game.world.admin`, `game.node.admin`, `game.db.admin`, `game.staff.admin` |
| `game_dba` | `game.db.read`, `game.db.backup`, `game.db.query`, `game.analytics.sql` |
| `game_analyst` | `game.analytics.read`, `game.analytics.player`, `game.analytics.sql`, `game.analytics.export` |
| `game_security` | analyst plus `game.analytics.relink`, `game.audit.read`, `game.sanction.read` |
| `game_release` | `game.node.deploy`, dormant until the owner's explicit deploy decision |

The viewer expansion follows ADM-01's read permissions except player/relink/SQL. Distinct roles do not imply that a Game administrator also has player value-write authority. Check permissions server-side on every view and mutation.

### 5. Risk controls

All admin sessions require confirmed MFA. Controls are cumulative by risk threshold, with Game's minimum authoritative:

| Risk | Platform controls |
| --- | --- |
| R0 aggregate read | Required permission |
| R1 player-linked read/re-link | Permission, one read-audit row per view, sensitive data masked by default |
| R2 reversible change | Permission and mandatory reason |
| R3 durable value/sanction/configuration | R2 controls plus step-up TOTP within five minutes of issuance |
| R4 destructive/service-wide | R3 controls plus second distinct admin approval, dry-run preview first, and maintenance window where required |

For R4 the approval must be newer than 30 minutes and cannot be by the requester. Execute a dry run first, display its exact effects, and bind execution through `params.preview_digest` to the digest of `dry_run_preview`; mismatch fails `ADM_PREVIEW_STALE`. Every R3/R4 command supports dry run. Missing, stale or ambiguous control evidence denies. Browser mutations retain CSRF, session and rate-limit protections.

### 6. Audit mapping onto the Rust table

Use append-only `audit_log` from `portal-rs/migrations/20261105170000_t10_rbac_audit.sql`, rather than creating the historical `admin_audit_events` table. Preserve its mutation-rejection trigger. Map every ADM-01 §9 Platform field as follows:

| Contract field | Existing storage / planned representation |
| --- | --- |
| `request_id` | `audit_log.request_id`, channel request UUID |
| `operation` | `action`, exact catalog operation |
| `target` | `target`, bounded stable scope identifier; full structured target in `after.target` |
| `created_at` | `created_at`, Platform event time in UTC |
| `admin_operation_id`, `risk_class`, `reason`, `approvals` | Explicit named fields in `after` JSONB |
| `ip`, `user_agent`, `correlation_id` | Explicit named fields in `after` JSONB, restricted audit access |
| `outcome`, `game_ledger_id` | Explicit named fields in `after` JSONB; ledger id absent until supplied by Game |

`actor` holds the Platform account UUID; `before` and `after` retain the applicable state/effect evidence. Version the audit JSON shape and validate required fields in the delivery packet. Append separate request, approval, delivery and result events; never update an earlier row to fill in a result. R1 dossier, inventory, history, re-link, export and SQL views each append a read event. Keep the signed request/result evidence in the request store and correlate it to audit rows and Game ledger ids. An unknown result is recorded as unknown, not inferred successful.

Game retains its own append-only ledger, at least two years with legal hold support, and emits restricted durable `ADMIN_ACTION` plus affected domain events with source `ADMIN` and operation causation under ANL-01. Platform audit does not replace that ledger or authorize raw ledger exposure through analytics.

### 7. Rust console modules and analytics boundary

Adopt all ADM-02 §3 modules as the planned Rust portal surface:

| Module | Scope |
| --- | --- |
| Dashboard | Online curves, Platform metrics, health, backups, alerts, recent actions |
| Worlds | Configuration/history, channels, events, maintenance, lifecycle wizards |
| Nodes and hosts | Agents, builds, assignments, resources, lifecycle and dormant update action |
| Database | Health, slow queries, migrations, backups/restore, maintenance, curated SQL reads |
| Players | Search, dossier, inventory, sessions/history, typed actions, linked masked Platform account |
| Sanctions | Reports, sanctions, appeals using existing Enforcement Command semantics |
| Economy | Supply/sinks, market, holders, provenance, admin changes and escrow |
| Analytics | Named dashboards, bounded filters, cohorts, funnels and exports |
| Security and integrity | Integrity signals, restricted re-link, staff review |
| Audit | Correlated Platform audit and Game ledger, including R1 views |
| Approvals | Distinct-admin R4 approval/rejection with preview and step-up |
| Access | Closed RBAC, signer status, connected agents and Game policy view |

Use axum handlers, askama SSR templates and the existing admin/web composition. Vendored Chart.js is a later dashboard dependency, not introduced by this ADR. Long operations poll Platform's result store; no additional queue service is required by this decision. Pages show source (Game agent or Platform), freshness, and mutation risk/controls. Provide Polish and English through the existing i18n layer, 360 px layouts and WCAG 2.2 AA keyboard, label and contrast support.

Game computes Game analytics and pushes rollups or answers named, bounded queries. Platform caches results with metric revision, watermark, completeness and applied filters; it never reconstructs Game analytics from raw data. Platform-owned metrics remain labelled as such. Read-only SQL, when delivered, is restricted to curated views with timeout, row/cost limits and full read audit. There is no write SQL, arbitrary shell or generic RPC operation.

### 8. Delivery and owner gates

The immediate follow-on scope is ADM-P1 (channel stores/endpoints/signer and ingress identity allowlist) and ADM-P2 (closed RBAC, step-up, audit mapping, read-only Dashboard, Worlds, Nodes, Database, Players, Audit and Access). This ADR itself delivers documentation only. Shared golden vectors and conformance under ADM-X1 qualify the channel with Game; unadvertised operations stay unavailable throughout.

ADM-P3 (moderation/operations, approvals, Sanctions, Economy reads, charts/analytics and alerts), ADM-P4 (value/configuration/deeper analytics) and ADM-P5 (R4 lifecycle/restore wizards) are later packets. Game phases 3–5 and deploy/fleet capabilities remain later work under ADM-02. Tests in delivery packets cover successful requests, refusals and bad input, including ADM-01 §10 signature, replay, expiry, permission/risk and preview cases against PostgreSQL where applicable.

Every Game or money capability is behind a flag off by default. No phase authorizes deployment or production activation; each production phase requires an explicit owner decision and conformance/security evidence. `game_release` and `node.update` remain dormant until separately authorized. World merge/delete retain their separate design gates. Existing restore reconciliation, topology, registry and enforcement contracts remain authoritative for their effects.

### 9. Open owner decisions

All ADM-02 §9 decisions remain open:

1. Backup tooling: retain `pg_dump` or adopt pgBackRest with PITR (recommended before phase 4).
2. Account-ban authority: Platform-decided account bans mirrored to Game versus Game-issued sanctions. The recommendation is Platform for account bans and Game for character bans, both using Enforcement Command semantics; this ADR does not accept that recommendation on the owner's behalf.
3. Production enablement order of phases.

## Consequences

The proposal preserves Game authority and the inbound-only Platform integration boundary while making privileged actions typed, independently verified and auditable. Rust portal components can be delivered before all Game executors exist, with honest unavailable states.

Costs include signer custody/rotation, identity binding, durable request reconciliation, approval controls, read-audit volume, and cross-repository conformance. The initial UI cannot promise operations absent from hello or analytics not yet produced. No runtime component, migration, permission grant or activation is created by this record.

## Alternatives considered

- Platform calling a Game admin API: rejected because it reverses the connection boundary and adds an inbound Game surface.
- Shared Game SQL credentials, write SQL or shell access: rejected because they bypass typed authorization, runtime fencing and durable domain audit.
- Porting the historical Blade console: rejected for this programme; the accepted portal plan selects axum/askama.
- Treating `platform_admin` as universal authority: rejected because the permission set is closed and high-risk controls remain mandatory.

## Reversal

While Proposed, this record grants no activation authority. Before enablement, implementation can remain disabled behind its flags. After any future execution, disabling the channel cannot undo Game effects or erase audit; reconciliation must retain original ids, signed evidence and Game receipts. Changing the accepted boundary would require a successor decision and compatible contract work in both repositories.
