PLAT3-DELETION implements the candidate Game-backed termination boundary as a dormant
library adapter. Governing source: Oteryn/Oteryn-Game at
3b49a20feec3ba29700bb4b2fa28fa9b539b79a2, PR #3010, CANDIDATE revision 1:

- docs/contracts/OTERYN_GAME_ACCOUNT_DELETION_FINALIZATION_V1.md
- docs/contracts/fixtures/account-deletion/{README.md,closure.json}
- docs/contracts/game-events/v1/plat3_authority.proto (AccountClosureDecisionV1)
- docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json (only event 7 belongs here;
  events 6/10/11 remain with their owners)
- docs/contracts/CROSS_REPOSITORY_CONTRACT_LOCK.json (pending, not canonical)

No Game files or shared contract registrations are changed. See fixtures/SOURCE.md
for byte provenance and the unsigned-source/synthetic-test-signature distinction.

Composition and transaction boundaries
--------------------------------------
GameBoundary wraps the existing local Boundary for sudo, blockers and credential
revocation; it never treats ActorProof as an account identifier. Terminations still
verifies the current password. The browser's existing CSRF/rate limiting and
request-time any-account_characters refusal remain in their existing HTTP adapter.
Default application wiring is unchanged and still refuses finalization.

The requested hook allocates a distinct monotone lifecycle generation atomically.
The due worker first commits an immutable CloseAccount command in a separate account
transaction. An unavailable/missing receipt cannot roll that command back. Pending
and read methods return original bytes, including expired unacknowledged commands;
no timeout synthesizes a receipt. A signed terminal expired/stale-generation refusal
can produce a fresh Close attempt after the recovery reconciler supplies its locked
floor. The original deadline/generation remain unchanged. Source revisions are
serialized and monotone across accounts; the pending backlog is bounded at 40.

The final worker locks the account and rechecks local blockers, the reconciled
recovery fence, receipt signature/hash/bindings, proof hash, current closure snapshot
and fresh account_closure watermark, and the independently authenticated complete
empty LCFA pair. LCFA feed/snapshot locks serialize withdrawal with proof use.
CLOSED_DETACHED is the Game's offline-fence/domain-preparation attestation; Platform
never infers that attestation from an empty local row count. Missing/partial work
stays pending. Credential erasure, exclusive FINALIZE command and finalized_at
commit together. Cancellation serializes on the same account, advances the fence,
and commits the exclusive CANCEL outbox decision. Delivery order cannot resurrect
an older close; Game must honor the cancellation tombstone defined by the contract.
Cancellation before any close was issued has no remote intent to resolve.

Receipt ingestion atomically retains the exact receipt and event-7 protobuf payload
in a private audit outbox. Permanent commands, generations and receipt replay state
are separate from the P90D audit payload. Equal receipt replays make no new effect;
changed signed receipts quarantine local proof. Equal projection pairs compare
content rather than observation time. Conflicts and revision gaps remain invalid;
an empty snapshot withdraws proof and a higher epoch clears its watermark.

Remaining fail-closed seams / enablement
---------------------------------------
All Settings fields default false. Enabling the feature requires explicit v2
qualification, reversible-quarantine acceptance, disposition/retention acceptance
(including bank/entitlement policy), and an operator for restoration conflicts.
These fields record external owner acceptance; this implementation chooses none.
The request-character gate cannot be relaxed through these settings.

A production RecoveryFence must lock the owning nonrollback reconciler's fence
through the transaction; UnavailableRecoveryFence always denies. A fixed supplied
Ed25519 verifying key must come from qualified Character publisher trust, never
from an inbound message. Detached signatures are verified over exact original
contract bytes; they add no JSON members/envelope. The pinned deletion fixtures do
not supply a signature transport. A qualified transport must bind that verifier to
the existing dedicated Character mTLS identity, rate limits and CC route/cadence
bounds before calling the library. No new public/internal routes are exposed here.

Full outbox resync after a gap/conflict, generation-selected receipt replacement
following restore, and expired resolution-attempt reissuance are not qualified by
this candidate. They stay pending; operators must not rewrite immutable evidence.
Restoration must be acknowledged by a signed CANCELLED receipt and current Game
projection before any future UX may claim restoration complete. The current default
HTTP boundary cannot enter this Game flow. Game kick/lease/portfolio/domain runtime
race tests require the separately implemented Game producer; no Game-runtime
conformance is claimed by Platform's local signed fixtures.

The event foundation publisher must provide the existing envelope and
TransactionEventRef/audit-retention/legal-hold handling for the staged event-7
payload before enablement; this module never invents a new envelope or dispatches
an audit event itself. Migration rollback means disable and retain authoritative
replay evidence; dropping closure state could allow an old command to execute.
