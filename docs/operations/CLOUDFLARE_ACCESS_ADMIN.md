# Cloudflare Access option for Oteryn administrator surfaces

## Status

This document describes an optional production defense-in-depth gate for administrator traffic. It does not prove or assume that Cloudflare Access is currently deployed.

The application remains authoritative for administrator authentication and authorization.

## Required application controls

Cloudflare Access must never replace any of these Oteryn Platform controls:

1. Platform `auth` authentication;
2. confirmed Platform MFA through `mfa.confirmed`;
3. explicit server-side RBAC through `admin.permission:<permission>`;
4. administrator audit recording for privileged mutations.

An Access decision alone must never grant an Oteryn role or permission.

## Recommended boundary

When the production topology supports it, place an Access policy in front of the administrator surface, for example the `/admin` path family or a dedicated administrator hostname.

The selected boundary must still allow administrators to complete the Platform login and MFA flow required before the application grants privileged access. The exact hostname/path policy depends on the deployed routing topology and is therefore an environment-specific deployment decision.

## Application trust boundary

Oteryn Platform does not treat browser-supplied identity headers as RBAC evidence.

If a future deployment consumes identity assertions from an Access integration, that must be introduced as a separately reviewed authentication integration with explicit trusted-proxy/header validation. Until then, Access is only an additional network/application-edge gate.

## Failure behavior

- If Access is unavailable or denies the request, administrator traffic may be blocked before reaching Oteryn Platform.
- If Access allows the request, Oteryn Platform still independently requires valid Platform authentication, confirmed MFA and the exact route permission.
- If Platform authorization state is missing or ambiguous, the application fails closed regardless of the Access result.

## Deployment checklist

- choose the exact administrator hostname/path boundary for the deployed topology;
- require the organization's approved strong authentication policy at the Access layer;
- keep `/admin` application routes protected by `auth`, `mfa.confirmed` and explicit permissions;
- verify that login and MFA completion remain reachable through the chosen routing design;
- ensure origin ingress restrictions do not create an alternate direct administrator bypass;
- test both Access denial and application-level authorization denial independently;
- document the operational recovery path for an Access outage without disabling Oteryn RBAC or MFA.

## Out of scope

This option does not introduce:

- automatic administrator provisioning;
- role synchronization from Cloudflare;
- trust in arbitrary request headers;
- bypass tokens for Oteryn RBAC/MFA;
- arbitrary plugin/code upload;
- Legacy engine or login-server authentication changes.
