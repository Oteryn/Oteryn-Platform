# PublicEdge Architecture

## Status and authority

**CURRENT FOCUSED ARCHITECTURE — architecture/evidence contract only, not live edge or production proof.**

This document is the focused Oteryn Platform architecture for the Internet-to-origin `PublicEdge` boundary declared by `docs/architecture/SYSTEM_ARCHITECTURE.md`. It consolidates already accepted hostname, security and production-readiness invariants from:

- ADR 0020 and `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md`;
- `docs/architecture/SECURITY_ARCHITECTURE.md`;
- `docs/operations/PRODUCTION_READINESS_CHECKLIST.md`;
- `docs/operations/PRODUCTION_TOPOLOGY_EVIDENCE.md`.

Provider-specific Cloudflare workflows, audits and operational guides are subordinate implementation/evidence sources. Their existence does not make Cloudflare a permanent architectural dependency and does not prove their effective live state.

This document does not authorize protected-environment access, DNS/TLS/WAF/Access mutation, production deployment, origin/network mutation, credential use, server/game-repository access or production activation.

No new ADR is required for this focused document because it does not alter the accepted public hostname decision, application-security policy or production go-live policy. A future change that materially changes public hostname ownership, weakens application defense in depth, establishes a durable provider lock-in or changes the production evidence/activation policy requires the relevant ADR process.

## Purpose

`PublicEdge` owns the expected control and evidence boundary between the public Internet and Oteryn-operated HTTP origins.

Its job is to make these questions explicit and independently verifiable:

- which public host represents which service;
- where public DNS/proxy and TLS termination occur;
- which redirect/canonicalization and HSTS behavior is intended and actually observed;
- which edge abuse controls or administrative access gates are intended and active;
- how public traffic reaches the declared origin and whether unintended direct-origin bypass exists;
- which environment/release the evidence applies to;
- which facts remain unknown when the protected provider/environment cannot be inspected.

`PublicEdge` is not an application business module and does not own application authentication, authorization, MFA, CSRF, session policy, domain rate limits or domain truth.

## Canonical current public endpoints

The service mapping is owned by `docs/contracts/PUBLIC_ENDPOINTS_CONTRACT.md` and is reproduced here only as routing context:

| Public hostname | Service role | Current declared Synology origin |
|---|---|---|
| `https://oteryn.molehill.cloud` | Oteryn Platform web application | `http://127.0.0.1:8000` |
| `https://gateway.molehill.cloud` | Oteryn Game Gateway / native-client login API | `http://127.0.0.1:8080` |

`login.oteryn.molehill.cloud` is retired and must not be restored as a canonical endpoint without an explicit contract/decision change.

The origin mapping above is a repository/deployment contract. It is not proof that current public DNS, proxying, Tunnel ingress, TLS, firewall or runtime state matches that contract.

## Ownership boundary

### PublicEdge owns

- expected public DNS hostname and proxy-mode semantics;
- public TLS/certificate identity, hostname coverage and termination evidence;
- HTTP-to-HTTPS and canonical-host redirect evidence where adopted;
- HSTS applicability, effective policy and rollout evidence after hostname/TLS scope is proven;
- edge WAF, bot, rate-limit/challenge and equivalent abuse-control evidence where used;
- edge administrative access gates such as Cloudflare Access where used;
- public tunnel/reverse-proxy/origin routing expectations;
- direct-origin/private-ingress exposure evidence;
- least-privilege provider control-plane credential requirements for edge automation/auditing;
- public acceptance evidence tied to the exact environment and observation time;
- fail-closed classification of unavailable, permission-denied, ambiguous or stale provider evidence.

### PublicEdge does not own

- Laravel/browser authentication, MFA, RBAC, CSRF, session or application authorization policy;
- application/domain rate limiting or validation;
- Identity or Game Session semantics;
- game-protocol TCP exposure or protection;
- Oteryn-v2/Canary/server runtime implementation;
- business data, content publication or gameplay truth;
- generic application observability, backup/restore or deployment evidence owned by `OperationsObservability`;
- production activation authority.

Cloudflare or another edge provider is defense in depth only. A request reaching the application outside the intended edge path must not gain application privilege merely because an edge control was bypassed.

## Provider-neutral control model

The durable architecture describes control classes rather than one vendor API.

### DNS and proxy identity

For every canonical public host, evidence must identify:

- the hostname;
- its expected service role;
- whether the public record is proxied or direct according to the selected provider model;
- the declared origin/tunnel target;
- whether retired hostnames remain externally reachable or routed.

A DNS record that resolves successfully proves only DNS behavior. It does not prove TLS, application reachability, correct origin routing, WAF policy or production readiness.

### TLS and certificate identity

Public TLS acceptance must independently prove the expected hostname identity and usable certificate chain for each canonical HTTPS host.

Repository knowledge of wildcard scope, certificate product configuration or a provider API record is supporting evidence only. It does not prove which certificate is actually presented to a public client.

TLS failure before HTTP must be classified as a TLS/edge failure. It must not be misreported as an application `4xx/5xx` or application availability result.

### Redirect and canonicalization

HTTP redirect behavior must be observed separately from HTTPS success.

Where canonicalization is adopted, evidence must distinguish at least:

- cleartext HTTP to HTTPS behavior;
- alternate/retired hostname behavior;
- final canonical hostname and scheme;
- redirect loops or provider challenge pages.

A `403`, challenge page or provider-generated interstitial is not a successful redirect and must not be classified as application success.

### HSTS

HSTS is fail-closed behind proven TLS and hostname/subdomain scope. The application intentionally does not hard-code HSTS before the effective public topology is proven.

Before enabling or strengthening HSTS, the selected policy must be compatible with every hostname/subdomain covered by its effective scope. Evidence must capture the actually served header and the environment/host where it was observed.

A repository default, desired value or provider rule that has not been observed at the public endpoint is not deployed HSTS evidence.

### WAF, bot, rate limiting and challenge controls

Edge abuse controls may supplement application controls. Evidence must identify the effective policy/resource sufficiently to distinguish:

- absent/unconfigured;
- intentionally disabled;
- enabled and applicable to the canonical host/path;
- permission-denied/unknown;
- an observed challenge/block result.

A successful public response does not prove that a WAF/bot/rate-limit policy exists. Conversely, a challenge or block must not be treated as proof that the upstream application is unavailable.

Public WWW and Gateway services must not be accidentally hidden behind an administrative-only Access gate unless that is an explicit accepted policy for the affected surface.

### Administrative edge access

Provider-side administrative access gates are additional protection only. They do not replace Platform authentication, confirmed MFA or RBAC.

Evidence must identify which administrative host/path is protected and which public host/path is intentionally not protected. An unknown Access policy must remain `UNKNOWN` rather than inferred from a single browser outcome.

### Tunnel, reverse proxy and origin ingress

The current deployment contract uses loopback origins for the Platform WWW and Game Gateway HTTP services. Evidence for a selected environment must distinguish:

1. the public host/proxy state;
2. the provider tunnel/reverse-proxy route;
3. the intended origin service/port;
4. the effective origin/firewall/private-ingress restrictions;
5. any direct-origin path that bypasses the edge.

Tunnel/DNS convergence proves neither TLS correctness nor origin-bypass resistance. Likewise, successful application health from the origin does not prove the public edge path.

Direct-origin exposure is `UNKNOWN` until the applicable network/provider evidence proves either that bypass is blocked or that a documented accepted risk applies where policy permits it.

## Evidence model

PublicEdge uses the same environment evidence semantics as `OPERATIONS_OBSERVABILITY_ARCHITECTURE.md` and the Production Go-Live Gate:

- `REPOSITORY_PROVEN` — exact source/configuration/tests prove a repository capability or invariant;
- `STAGING_PROVEN` — a dated controlled non-production exercise proves only the exact exercised environment/run;
- `ENVIRONMENT_EVIDENCE_REQUIRED` — direct deployed-environment evidence is required and not yet present;
- `PRODUCTION_PROVEN` — authorized direct evidence proves the bounded fact for the exact production environment/release;
- `UNKNOWN` — available evidence is absent, stale, permission-denied, contradictory or insufficient.

No state promotes itself automatically to another state.

In particular:

- repository configuration does not become `STAGING_PROVEN` merely because it can be deployed;
- staging/Tunnel/DNS success does not become `PRODUCTION_PROVEN`;
- a public point-in-time probe proves only the behavior it observed, not hidden provider configuration;
- provider API success proves only the resources that token permissions allow it to inspect;
- `401`/`403 permission_denied` from the control plane is evidence of an inspection boundary, not evidence that the requested control is absent;
- missing evidence remains `UNKNOWN` or `ENVIRONMENT_EVIDENCE_REQUIRED` rather than being filled from intended architecture.

## Evidence envelope

A production/PublicEdge proof record should be sanitized and should identify, where applicable:

- observation timestamp;
- environment identity;
- exact active application/Gateway release identity when the proof depends on them;
- canonical hostname/service role;
- public DNS/proxy state;
- public TLS certificate/hostname result;
- HTTP/HTTPS/redirect result;
- effective HSTS result;
- applicable WAF/bot/rate-limit/Access resource identifiers or explicitly recorded absence/unknown state;
- intended tunnel/reverse-proxy/origin mapping;
- direct-origin/private-ingress disposition;
- test/probe result and bounded sanitized errors.

Do not store provider tokens, authorization headers, private keys, copied `.env` files, raw sensitive rule expressions, private infrastructure inventories or unrelated personal data in the evidence record.

## Failure and ambiguity semantics

The following states must stay distinct:

| Observation | Correct classification |
|---|---|
| DNS record resolves | DNS behavior only |
| Tunnel/DNS apply converged | configured routing evidence only |
| TLS handshake/certificate validation fails | PublicEdge TLS failure before HTTP |
| HTTPS returns provider-generated `403`/challenge | edge-policy/challenge observation; upstream app state not proven |
| HTTP does not redirect | redirect acceptance failure independent of HTTPS content |
| HSTS missing or `max-age=0` | HSTS inactive/disabled at the observed response, not a generic TLS failure |
| Provider API returns permission denied | provider-control state remains unknown; inspection scope is insufficient |
| Origin responds locally | origin/runtime evidence only; public route and bypass posture not proven |
| Public endpoint succeeds | observed public path works; hidden controls and direct-origin posture still require their own evidence |

An implementation or operator must not collapse these observations into one generic `edge healthy` boolean.

## Current Cloudflare implementation evidence

The current repository contains Cloudflare-specific endpoint management and a trusted-main GET-only remaining-edge audit. These are valid current implementation/operations mechanisms beneath this architecture.

Current durable evidence from the active `OTERYN-20260801-public-domain-repair` record establishes, among other bounded facts, that:

- repository/staging public-domain changes and the canonical Tunnel/DNS mapping were previously reconciled;
- later public acceptance still observed material TLS/HTTP/policy failures;
- the protected Cloudflare token could not inspect all certificate, Ruleset, Bot, Access and selected zone-setting resources;
- token self-management capability was not proven;
- further autonomous live inspection/repair is blocked until an external Cloudflare administrator replaces the protected token with the minimum required read scopes.

Those facts are execution/evidence state, not architecture policy. This document does not bypass that blocker, expand token scope or claim the remaining live controls are configured correctly.

## Least-privilege control-plane credentials

PublicEdge automation must use the minimum provider scopes required for the exact bounded operation.

Preferred progression for a repair is:

1. read-only inspection using minimum applicable scopes;
2. exact resource identification and sanitized evidence;
3. determination of the smallest write permission needed, if any;
4. separately authorized bounded mutation;
5. independent public revalidation;
6. removal/reduction of unnecessary write capability where operationally practical.

A token must not be assumed able to inspect or modify a resource merely because authentication succeeds for another provider API family. A token also must not be expected to self-expand unless that capability is explicitly proven and authorized.

## Rollout and rollback

A PublicEdge change must have a bounded rollback target before mutation when rollback is technically meaningful.

Examples include the prior exact DNS record, tunnel ingress target, redirect rule revision, Access/WAF rule state or HSTS configuration. The rollback description must not require publishing secrets.

HSTS deserves additional caution: browser-cached policy can outlive a server-side rollback. Stronger HSTS or subdomain inclusion therefore requires proof that the affected hostname scope is ready before activation; disabling a rule later is not an instantaneous client rollback.

## Validation expectations

### Repository/documentation validation

Architecture and tooling changes should prove:

- canonical host/service mappings remain consistent across the public endpoint contract and deployment docs;
- provider tooling cannot silently reinterpret retired hostnames or swap Platform/Gateway origins;
- trusted workflows receiving protected credentials do not execute untrusted PR code;
- audit outputs are sanitized and mutation-free when declared read-only;
- provider-specific tooling remains subordinate to this provider-neutral architecture.

### Public acceptance

After an authorized edge change, the smallest sufficient public acceptance should independently verify the affected behavior rather than relying solely on provider API success.

Applicable acceptance may include:

- public TLS hostname/certificate verification;
- HTTPS response classification for both canonical hosts;
- cleartext HTTP redirect behavior;
- canonical/retired-host disposition;
- HSTS header observation when in scope;
- challenge/Access behavior for intended public versus administrative surfaces;
- direct-origin/private-ingress disposition where safely and authoritatively observable.

A result must identify what was actually tested and what remains unobserved.

### Runtime/browser E2E

For an architecture-only package, runtime/browser E2E is `NOT_APPLICABLE` because no executable path changes.

For a later live edge repair, public network acceptance is an operational E2E/evidence gate and must be classified separately from Laravel browser/application E2E.

## Relationship to production readiness

PublicEdge is one required boundary of the Production Go-Live Gate. A focused architecture document can become complete while production remains unproven.

`PRODUCTION_PROVEN` requires authorized direct evidence for the exact production candidate/environment. Neither this document, successful repository CI, a staging run nor the existence of provider tooling satisfies that requirement.

Production activation remains a separate explicit authority even after all PublicEdge evidence passes.

## Implementation and evidence handoff

The existing `OTERYN-20260801-public-domain-repair` task remains the durable execution/evidence record for the currently blocked Cloudflare/public-domain repair. This architecture task does not take ownership of its live token, protected environment, trigger workflows, reports or external mutation scope.

The next live PublicEdge action remains outside this architecture package: an authorized external Cloudflare administrator must replace the protected token with the minimum required remaining-edge read scopes; only then may the existing trusted-main audit determine exact resource state and justify any smallest write scope/repair.

Issue #490 remains the shared audit owner until PublicEdge protected-environment proof and the other still-applicable direct production evidence requirements have terminal dispositions.