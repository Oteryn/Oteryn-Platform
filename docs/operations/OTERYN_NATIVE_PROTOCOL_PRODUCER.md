# Oteryn native protocol producer operations

## Status

`PRODUCER COMPLETE — DISABLED BY DEFAULT — CONSUMERS NOT YET DELIVERED`

This document describes the Platform and Game Gateway producer package for coordination ID `OTS-20260804-native-protocol-selection`.

It does not claim native gameplay availability. Otheryn Game Session v2/native admission and the Rust `protocol-oteryn` consumer remain separate required packages.

## Delivered producer boundary

The existing Gateway API remains version 1. A legacy request containing only `protocol_version` and `game_login_ticket` continues to use the existing Canary-compatible Game Session contract version 1.

An optional bounded `gameplay_offer` allows the Gateway to:

1. validate request syntax before ticket redeem;
2. redeem the one-time Game Login Ticket;
3. obtain the authoritative world, characters and ordered World Registry candidate policy;
4. select the first exact policy candidate offered by the client;
5. verify the selected endpoint readiness identity;
6. issue exactly one opaque Game Session contract version 2 request;
7. return the immutable gameplay selection and sanitized endpoint.

There is no second candidate attempt after readiness or issuer invocation, no gameplay-byte sniffing and no fallback to another family or native protocol version.

## Canonical native identity

```text
family: oteryn
native_protocol_version: 1
transport: tcp.tls13.protobuf.be32.v1
schema_revision: 2
schema_sha256: 9c67f19525400fb9890d2a3541ceb6d02eb955061540ad39ca1c1d891c06eba9
channel_id: 1
```

The SHA-256 value is pinned to the exact UTF-8 bytes of `docs/contracts/oteryn_native_gameplay_v1.proto`. Platform policy projection and Gateway selection both fail closed when the native identity differs.

## Disabled-by-default rule

The migration creates no protocol candidate rows. Every new `game_world_protocol_candidates.enabled` value defaults to `false`.

Do not enable or advertise a native candidate until all of the following are proven for the exact revisions being deployed:

- Otheryn implements the matching v2 issuer/readiness and native admission listener;
- Otheryn readiness echoes the exact world, channel, policy revision, endpoint, audience, native protocol version, transport, schema and capability digest;
- the Rust client implements and offers the exact native candidate;
- cross-repository authorization and gameplay E2E pass;
- the public host, port, TLS server name and certificate identity are verified;
- deployment and rollback ownership is assigned.

## World Registry administration

A candidate is scoped by `game_world_id` and `channel_id`, ordered by `sort_order`, and bound to a monotonic `gameplay_policy_revision` on the world.

Changing candidate order, endpoint identity, required capabilities, schema identity or enablement requires incrementing the world policy revision. Candidate rows must remain unique in policy order and protocol tuple.

Invalid enabled candidate data invalidates the projected policy instead of silently skipping to another candidate. The legacy world route remains available only to legacy requests; extended requests fail closed.

## Readiness and issuance

Gateway calls the v2 readiness boundary before v2 issuance. A missing or contradictory readiness response returns the public `login_unavailable` error and no second candidate is attempted.

The v2 issuer request binds:

- authoritative game account and security generation;
- random login-attempt identifier;
- world and channel;
- policy revision and endpoint identity;
- audience `otheryn-world:<world>:channel:<channel>:endpoint:<endpoint>`;
- exact family/native_protocol_version/transport/schema/list/digest;
- `bind_on_first_admission`;
- single-admission intent.

Credentials, tickets, request bodies, account identifiers and protocol payloads are not logged.

## Rollout

Normal rollout order:

1. deploy the Platform schema/domain and Gateway producer while all candidate rows remain disabled;
2. deploy the exact Otheryn v2/native consumer and verify private readiness;
3. deploy the exact Rust consumer without forcing native selection;
4. run integrated authorization and gameplay E2E;
5. create or update the candidate row while still disabled;
6. increment policy revision;
7. enable advertisement for a bounded test world/cohort;
8. expand only while readiness and compatibility evidence remain current.

## Rollback

Normal rollback:

1. set the native candidate `enabled=false`;
2. increment `gameplay_policy_revision`;
3. verify Gateway no longer selects native for fresh logins;
4. allow already-issued unexpired sessions to drain under the canonical contract;
5. roll back application code only after database compatibility is verified.

Emergency rollback additionally disables the Otheryn native admission listener or raises the admission-revocation mechanism owned by the Otheryn package.

The database migration is reversible: rollback drops `game_world_protocol_candidates` and then removes `game_worlds.gameplay_policy_revision`. Never run that destructive rollback while any deployed code requires the new columns.

## Public failures

```text
400 invalid_request             validation failed before redeem
401 invalid_login               ticket redeem failed
409 unsupported_gameplay_pair   valid redeemed login has no exact policy intersection
503 login_unavailable           policy, readiness or issuer failed after redeem may have occurred
```

A client must obtain a fresh Game Login Ticket after `401`, `409`, `503` or an ambiguous network failure.

## Missing consumers

Producer completion does not complete native gameplay. Remaining mandatory packages are:

- Otheryn Game Session v2 storage/readiness/admission and native gameplay listener;
- Rust `protocol-oteryn`, immutable adapter selection and native state/command handling;
- final cross-repository selection, admission, snapshot/delta, action and rollback E2E.
