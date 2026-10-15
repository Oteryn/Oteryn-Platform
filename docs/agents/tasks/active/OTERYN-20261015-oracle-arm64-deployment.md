---
task_id: OTERYN-20261015-oracle-arm64-deployment
governing_issue: 1639
required_reads: []
search_first: []
optional_reads: []
---

# Oracle ARM64 deployment

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1639

The owner authorized installing and qualifying Game, Platform, gateway and their databases on the existing Oracle A1 host before migrating NAS data and switching traffic. No cutover before the real login/create/list path passes. Preserve all production trust, freshness, fencing and durability boundaries. No paid resources.

Owned paths: deploy/oracle/** and this packet. Game source is read-only, at 87f770c87ce17518ba69f5effdbd04725d8a7d06. Platform source base: 1194d5548760d9d773a1cebb182bc5986a6a42bf. Existing account repair PR #1637 is a separate candidate and is not silently included.

Native image builds are target-host acceptance of the ARM64 deployment using immutable public source archives; no repository checkout is edited or published through Remote Desktop. Runtime wiring is authored in this isolated task worktree. The host is accessed only through exact-call validated host exceptions. Native ARM64 deployment qualification has no existing equivalent CI job.

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-15T20:15:00Z
head: 237aaadcc0a8aa8a8b65285c71462337919e7c55
branch: codex/oracle-arm64-deployment-20261015
pr: 1640
status: implementing
context_routes:
  - deployment
owned_paths:
  - deploy/oracle/**
  - docs/agents/tasks/active/OTERYN-20261015-oracle-arm64-deployment.md
proven:
  - Oracle ARM64 4 OCPU 24 GB RAM 200 GB disk and persistent remote agent
  - Docker 29.1.3 Compose 2.40.3 and buildx 0.30.1 installed
  - Platform and gateway ARM64 builds passed from immutable source 1194d5548760
  - Game ARM64 build, 77 migrations, pinned World and NPC staging passed
  - Platform HTTP 200 and gateway readiness HTTP 200
  - Platform forced WAL archive, base backup and logical dump passed
  - Game PostgreSQL verify-full TLS 1.3; runtime DDL, plaintext and unknown CA rejected
  - Game WAL archive has 3 archived segments and zero failures
  - Candidate CodeQL passed after atomic owner-only credential creation
  - Own task checkpoint validates; remaining liveness failure names baseline task issue 1623
derived: []
unknown:
  - Full native login/create/list and restore qualification results
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - deploy/oracle/game.Dockerfile
  - deploy/oracle/prepare-platform.sh
  - deploy/oracle/README.md
  - deploy/oracle/prepare-game-db.py
  - deploy/oracle/game-db.compose.yml
  - deploy/oracle/prepare-game.sh
  - deploy/oracle/check-game-db.py
validation:
  - command: target docker version and docker compose version
    result: PASS
    evidence: verified on Oracle through its remote agent
blockers:
  - Final coherent source freeze, transfer and real native path remain unqualified
  - none; terminal baseline packet is archived in this candidate
next_action: qualify source snapshot restores, preserve target qualification state, then freeze and transfer existing NAS state without resetting trust or independent custody
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: pending
```

Owner explicitly approved temporarily removing only sudo from the Oracle agent blockedCommands for this deployment. Original list is preserved; restore it when deployment work stops. No other agent control is changed.

Cloudflare: the subsequent explicit owner instruction authorizes direct public DNS and HTTPS on oteryn.com; the rollout below supersedes the original isolated-edge assumption. NAS data has not migrated.

## Direct oteryn.com edge rollout

Owner explicitly instructed moving all web/Game service hostnames of oteryn.com to Oracle and rejected Tunnel. Canonical APP_URL is now https://oteryn.com with secure cookies and explicit edge/internal-proxy trust. Caddy 2 ARM64 is digest-pinned, with persistent automatic certificate storage. OCI default security list received only stateful TCP 80, 443 and 7172 ingress; existing SSH/ICMP/egress preserved.

Cloudflare apex A now targets 158.180.58.90 (proxied); www, gateway, login and game A records target that same address as DNS-only. The original apex tunnel target was 65dcd00f-3b26-424b-a01f-c476e133754c.cfargotunnel.com. All three MX records, DKIM and SPF were preserved. Game DNS is not a native gameplay readiness claim.

## Existing-state migration reconciliation

Source inspection confirms six identities and two authoritative Game Characters;
target qualification has two identities and no Characters. Preserve both target
qualification databases privately before the actual NAS cutover. Transfer existing
trust, custody and canonical identifiers instead of activating unrelated fresh
trust. No new Actions workflow is needed for already-initialized trust. The unused
fresh bootstrap candidate remains historical in ac73de2 only. The source Registry
World and Channel remain the migration identities; the target's qualification scope
is not substituted for them. Game main advanced to 77af046ce14c81df6ed6cab03ae693a68acff32c;
rebuild native runtime before source state is opened on Oracle. The deployed NAS
World digest 1b04cc35a65bcced64dbc933fcb5ac9adad426a369149bb8c170b1aa80d04406
is distinct from the main pin and must be preserved and validated as deployed state.

## Restore feasibility and restart candidate

Source snapshot SHA256 109feed7b35307271d3f4799035d2e1e9f55894367a5407522453e2f26fe98b4
was transferred with a one-file restricted SSH key and strict independently checked
host keys. Private data and keys never enter repository files or logs. Both custom
PostgreSQL dumps restored into isolated scratch databases. Source Platform default
ACLs require the PostgreSQL administrator, while ordinary schema/data restores run
as the schema owner; the six default ACL entries were restored separately. Readback:
6 identities, 2 projected Characters and 2 authoritative Game Character roots.
Read-only trust observation and actual signing self-check passed against the source
snapshot on Oracle, retaining store marker, profile version/revision 1 and trusted
key oteryn-admission-20261011-165131-58b205. No new trust publication occurred.

Game supervisor candidate retains durable phase/cycle identity, reconciles retained
ops files, refuses registration gaps and unresolved assignment/report outcomes, and
owns each Docker incarnation at boot. Six focused custody/uncertainty tests pass.
Native systemd and real restart acceptance remain pending. Public certificates are
renewed by Caddy and refreshed through this controller. No full readiness claim.
