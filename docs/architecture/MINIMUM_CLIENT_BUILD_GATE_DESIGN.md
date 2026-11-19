# Minimum client build gate: design

Status: **PROPOSED**, together with
`docs/contracts/OTERYN_V2_MINIMUM_CLIENT_BUILD_GATE_CONTRACT_PROPOSAL.md`
(the contract wins where they differ). Implementation is a separate PR after
acceptance.

## 1. Pieces and owners

| Piece | Owner | Exists today |
|---|---|---|
| Ordered build id (`release_sequence` per update channel), signed | release signing + TUF channel policy (ADR 0035, client update origin v1) | yes, in `policy-v1.json` |
| Client reports its release in `offer.release` | Game client | no |
| Gateway validates and forwards `offer.release` | `crates/gateway` (`wire.rs` `Offer`, `check_offer`) | no |
| Gate records, effective minimum, refusal | `crates/gameauth` admission | no |
| Active verified policy lookup | `crates/downloads` (published generation) | yes |
| Operator command and audit | `oteryn-portal admin-cli` | no |
| Which roots a release trusts, chain served by a node | Game ADR-0024 | no |

The gate adds no network call. The admission transaction reads the gate
records and the active Downloads generation from Platform's own database.

## 2. Data model (implementation PR, one additive migration)

`game_client_build_gates`:

| column | type | note |
|---|---|---|
| `scope_kind` | text | `global`, `world` or `channel` (CHECK) |
| `world_id` | uuid null | null only for `global` |
| `channel_id` | uuid null | set only for `channel` |
| `min_stable` | bigint null | > 0 |
| `min_beta` | bigint null | > 0 |
| `unknown_release` | text | `admit` or `refuse` |
| `state` | text | `open`, `enforcing` or `maintenance` |
| `revision` | bigint | +1 per change |
| `chain_evidence` | text null | free reference, at most 256 bytes |
| `updated_by` | uuid | staff account |
| `updated_at` | timestamptz | |

- A unique index on `(scope_kind, coalesce(world_id), coalesce(channel_id))`.
- Every change also writes `audit_log` with the old and new row.
- An absent row is the same as `open`.

The admission transaction reads the global row, the World row and the rows
for the candidate game channels with `FOR SHARE`. A concurrent gate change
then serializes against issuance: a grant is never signed under a minimum
that was already raised in a committed transaction.

**Policy values.** The policy minimum, current sequence and revoked ids come
from the active Downloads generation for the update channel. Platform already
stores that generation, verified at publish time against the pinned Root.
There is no new parsing of unverified bytes. An expired active head counts as
no policy (absent), as for the public route.

## 3. Operator command

```text
oteryn-portal admin-cli game-build-gate set \
  --scope global|world:<uuid>|channel:<world-uuid>/<channel-uuid> \
  [--min-stable N|none] [--min-beta N|none] \
  [--unknown-release admit|refuse] \
  --state open|enforcing|maintenance \
  [--chain-evidence <ref>] --expected-revision R --actor <staff-UUIDv7>
oteryn-portal admin-cli game-build-gate show [--scope ...]
```

**Who may run it.** The command requires permission `game.build_gate.manage`
(new, staff/admin only).

**When it refuses.** It refuses with a nonzero exit and nothing written when:

- the revision is stale;
- a minimum exceeds the active verified policy's `current_release_sequence`
  for that update channel;
- the active release lacks an artifact for a supported platform and
  architecture;
- a raise into `enforcing` comes without `--chain-evidence`.

`maintenance` needs no evidence and no policy. It must always be possible to
stop issuance at once.

**Not allowed.** There is no HTTP mutation route in v1. A staff UI can come
later on the same service function.

## 4. Serving two chains at once (input to ADR-0024)

Step 2 of the planned rotation needs a node to serve A and B together. Two
ways, for Game to choose:

1. **One listener, chain by client hint.** The client sends the TLS 1.3
   `certificate_authorities` extension (RFC 8446 §4.2.4) with the roots it
   trusts, and the node picks the matching chain.
   - Pro: no route change.
   - Con: the client TLS stack and the node must both support the extension.
     A client that sends nothing gets the default chain, which must stay A
     until step 6.
2. **Two route records per game channel during the overlap.** Each record has
   its own port and `tls_server_name`, with an added `trust_epoch` member in
   the route record and the grant.
   - Pro: plain TLS on both sides.
   - Con: a route record and grant revision, owned by the native gateway
     login contract and planned there, not here.

The gate works the same with either. The only Platform-visible input is the
release sequence that starts trusting B (`s_AB`), which the operator enters
as the minimum.

## 5. Observability

**Metrics:**

- `admission_gate_refusals_total{code, update_channel}`;
- a histogram of offered `release_sequence` per update channel, bucketed;
- the share of logins without `release`.

No account or character ids appear as labels.

**Adoption.** Before step 4 the operator reads the share of recent logins
below `s_AB` (`admin-cli game-build-gate show --adoption`). This tells them
how many users the raise will send to the updater.

## 6. Failure behaviour

| Situation | Result |
|---|---|
| Gate tables unreadable | `NATIVE_LOGIN_UNAVAILABLE` (fail closed, no grant) |
| No active policy for the update channel | policy minimum absent; gate records still apply; `current_release_sequence` null in the body |
| Minimum set, policy later rolled back below it | issuance keeps enforcing. `update.current_release_sequence` is below the minimum, so the client cannot update and shows the download page. The operator must lower the minimum, which is audited. Platform raises an alert when the active policy's current sequence is below an enforced minimum. |
| Client sends `release` before Platform supports it | older issuer: `NATIVE_LOGIN_REQUEST_MALFORMED` (contract §3 ordering) |

## 7. Implementation PR outline

1. Migration and the `game_client_build_gates` store in `crates/gameauth`.
2. `Offer.release` in `crates/gateway/src/wire.rs` and
   `crates/gameauth/src/admission/wire.rs`, plus offer digest vectors.
3. The gate in the admission transaction (protocol 2 and 3, recovery), with
   the two new `Error` variants and their bodies.
4. A read of the active Downloads generation by update channel (a
   `crates/downloads` read function, no new route).
5. The `admin-cli game-build-gate` command, permission and audit.
6. Contract fixtures under
   `docs/contracts/fixtures/minimum-client-build-gate-v1/`: request with
   `release`, update-required body, maintenance body with and without
   `update`, malformed `release` vectors.
7. Platform acceptance tests 1-10 of the contract.

Feature flag: `PORTAL_CLIENT_BUILD_GATE_ENABLED`, default off. When off, the
issuer accepts `offer.release` and does not gate. The wire change can then
ship ahead of enforcement.
