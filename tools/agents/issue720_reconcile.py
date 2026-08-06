from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")


gateway = "docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md"
replace_once(
    gateway,
    "`TARGET CONTRACT — NOT YET IMPLEMENTED`\n\nThis contract defines",
    """`IMPLEMENTED REPOSITORY CONTRACT — DEPLOYMENT AND PRODUCTION ACTIVATION NOT PROVEN`

`PRODUCTION_PROVEN=false`.

## Delivered repository boundary

The bounded contract producer is delivered on `main`:

- Oteryn Platform PR #121 delivered Game Login Ticket issuance and the private atomic redeem endpoint;
- Oteryn Platform PR #122 delivered the separately buildable Game Gateway, authoritative login-context orchestration, World Registry use, account-scoped character projection and the Game Session issuer boundary, merged as `8006534108d835474dadd208b0ec934e4a12528b`;
- `GAME_SESSION_CANARY_CONTRACT.md` is the current operation-specific authority for the legacy-compatible Game Session contract version 1 and records its bounded cross-repository E2E evidence.

Repository delivery proves the application and service contract, not a production deployment. Exact public/private ingress, TLS termination, service identity, credential rotation, deployed revisions, global legacy-path retirement and effective network isolation remain `UNKNOWN` until verified against one exact environment.

The selected native Game Session contract version 2 remains disabled by default and separately governed. Active PR #542 is producer-only work and is not evidence that a native consumer or production cutover is complete.

This contract defines""",
)

auth = "docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md"
replace_once(
    auth,
    "`PARTIALLY PROVEN — CURRENT FLOW MAPPED / CREDENTIAL MIGRATION BLOCKED`",
    "`PARTIALLY PROVEN — LEGACY PATHS MAPPED / OTERYN GATEWAY PATH DELIVERED / PRODUCTION CUTOVER UNKNOWN`",
)
overlay = """## Current Oteryn path overlay

This section is the current repository-state overlay. It does not erase the legacy-path discovery below.

### Delivered on `main` — PROVEN

```text
OTClient
  -> Oteryn Identity / OAuth + PKCE
  -> short-lived single-use Game Login Ticket
  -> separately deployable Oteryn Game Gateway
  -> private Platform ticket redeem
  -> private Platform login context
       -> Platform World Registry
       -> account-scoped active character projection
  -> Game Session issuer contract version 1
  -> opaque short-lived Game Session credential
  -> Canary world-entry admission
```

The Platform/Gateway producer boundary was delivered through PR #121 and PR #122; PR #122 merged as `8006534108d835474dadd208b0ec934e4a12528b`. The operation-specific world-entry authority is `GAME_SESSION_CANARY_CONTRACT.md`, which records the bounded Canary and OTClient consumer evidence for legacy-compatible Game Session contract version 1.

The delivered path does not send the user's Oteryn password across the Gateway-to-Canary boundary. Gateway authorization is derived from authoritative ticket redeem and the exact ready Platform-to-Canary account binding; client-supplied account or character ownership data cannot establish authority.

### Still retained or unresolved

The legacy native Canary, external login-server, reusable `account_sessions`, old-protocol direct-password and source-level alternate-path findings below remain relevant until exact deployment evidence proves that each unsupported path is disabled, isolated or intentionally retained.

```yaml
repository_gateway_path: PROVEN
legacy_path_source_capability: PROVEN
native_v2_platform_producer: DISABLED_BY_DEFAULT
native_v2_cross_repository_consumer: NOT_PROVEN_BY_THIS_REPOSITORY
production_deployment_identity: UNKNOWN
production_private_ingress: UNKNOWN
production_service_auth_rotation: UNKNOWN
global_legacy_path_retirement: UNKNOWN
PRODUCTION_PROVEN: false
```

No repository document may infer global MFA, email-verification, password-migration or session-revocation enforcement merely from the delivered Gateway path while an alternate deployed password path may remain reachable.

"""
replace_once(auth, "## Evidence baseline\n", overlay + "## Evidence baseline\n")

system = "docs/architecture/SYSTEM_ARCHITECTURE.md"
topology = """## Current game-authentication topology

The current repository topology includes a separately deployable Game Gateway in addition to the Laravel modular monolith:

```text
OTClient
   |
   v
Oteryn Identity / OAuth + PKCE
   |
   +--> one-time Game Login Ticket
            |
            v
     Oteryn Game Gateway (Go service)
            |
            +--> private Platform ticket redeem
            +--> private Platform login context
            |       +--> Platform World Registry
            |       +--> read-only account-scoped Canary character projection
            |
            +--> Game Session issuer contract v1
                    |
                    v
              Canary world-entry admission
```

This is a repository-delivered bounded path, not proof of one exact deployed production topology. Oteryn Platform PR #122 merged the Gateway producer as `8006534108d835474dadd208b0ec934e4a12528b`; the detailed identity and world-entry semantics remain owned by `GAME_GATEWAY_IDENTITY_CONTRACT.md` and `GAME_SESSION_CANARY_CONTRACT.md`.

The earlier `login-server / auth path` in the broad system-context diagram represents retained legacy/external compatibility paths, not the only current authentication topology. Exact exposure or isolation of native Canary login, external login-server and old-protocol password paths remains environment-specific and `UNKNOWN` without deployment/network evidence.

Native Game Session contract version 2 is a separate disabled-by-default rollout. Active producer PR #542 does not prove an Otheryn/OTClient consumer, cross-repository cutover or production activation. `PRODUCTION_PROVEN=false` until exact deployed revisions, edge/private ingress, service authentication/rotation and legacy-path disposition are verified together.

"""
replace_once(system, "## Evidence dimensions\n", topology + "## Evidence dimensions\n")

module = "docs/architecture/MODULE_CATALOG.md"
replace_once(
    module,
    "| Integration | AVAILABLE | Implemented Canary read/write adapters, schema translation and contract enforcement; future login bridge remains separate | Product policy that belongs in domain modules |",
    "| Integration | AVAILABLE | Implemented Canary read/write adapters, schema translation and contract enforcement; the bounded Identity -> Game Gateway -> private Platform login-context -> Game Session v1 bridge is merged, while native v2 and production cutover remain separate gated work | Product policy that belongs in domain modules, direct credential policy duplication or production-activation claims without exact evidence |",
)

task = "docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-reconciliation.md"
replace_once(task, "pr: pending\nstatus: implementing\nphase: implement", "pr: 731\nstatus: validating\nphase: validate")
replace_once(
    task,
    "changed_paths:\n  - docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-reconciliation.md",
    """changed_paths:
  - docs/contracts/GAME_GATEWAY_IDENTITY_CONTRACT.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/architecture/SYSTEM_ARCHITECTURE.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/agents/tasks/active/OTERYN-20260806-game-auth-topology-reconciliation.md""",
)
replace_once(
    task,
    "next_action: Apply the four canonical documentation corrections, remove the temporary helper, and open the draft PR.",
    "next_action: Verify the exact five-path diff, run exact-head governance/documentation CI, then publish one fresh independent documentation audit target.",
)
