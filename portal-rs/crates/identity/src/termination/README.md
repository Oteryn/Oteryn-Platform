PLAT3 account closure
====================

Governing Game source is Oteryn/Oteryn-Game at
3b49a20feec3ba29700bb4b2fa28fa9b539b79a2 (PR #3010, CANDIDATE revision 1):
OTERYN_GAME_ACCOUNT_DELETION_FINALIZATION_V1.md, its account-deletion fixtures,
plat3_authority.proto AccountClosureDecisionV1, GAME_EVENT_FOUNDATION_REGISTRY.json
(event 7 only; 6/10/11 belong to other modules), CROSS_REPOSITORY_CONTRACT_LOCK.json.
All Game inspection uses this immutable SHA. No Game files are changed.
fixtures/SOURCE.md records the exact byte/hash provenance and explains that the
source fixtures contain no signatures; synthetic signing material is test-only.

Production composition
----------------------

Serve and worker use bin/oteryn-portal/src/account_closure.rs. With both
PORTAL_ACCOUNT_TERMINATION_ENABLED=true and PORTAL_ACCOUNT_CLOSURE_ENABLED=true,
the factory installs the real GameBoundary around the existing sudo/password,
wallet/Bazaar/email/any-account_characters checks and credential revocation.
With the feature off it installs UnavailableBoundary. Missing configuration when
on is an error, never a silent fallback. These are the only activation flags;
owner acceptance is no longer represented by additional off-by-default switches.

Game pulls the existing CC direction: POST character-commands/pending and /read,
then publishes to /receipt. Closure v2 uses the same exact route names and command
identity. The new private account-closure and account-closure/watermark routes
accept the contract's canonical snapshots. The production listener also composes
the existing complete LCFA publisher routes under a DIFFERENT certificate identity.
No closure routes are merged into the public browser router.

The required BASE_URL is the advertised HTTPS Platform origin the Game consumer
calls. There is no Game-host push endpoint in the pinned contract, so no outbound
Game API has been invented. The actual TLS 1.3 / HTTP 1.1 listener validates the
client CA and exact configured DER leaf SHA-256. It alone inserts VerifiedClient.
The Character client and LCFA publisher pins must differ; a forged header cannot
establish either identity. Detached Ed25519 publications use x-oteryn-key-id and
x-oteryn-signature (lowercase hex over the exact original body). Both are checked
against configured trust; keys are never selected from publisher-supplied material.
Only file paths to server secrets are configured; no secrets are stored here.

Every command and lifecycle generation commits under the Platform account lock.
The due worker commits CloseAccount in a separate transaction, so missing proof
cannot roll back issuance. The pending rescan retains expired unreceipted commands.
A signed terminal expired/stale refusal may produce a fresh immutable Close attempt
using the reconciled recovery floor. Source revisions are serialized across accounts.
The final transaction rechecks blockers, current recovery fence, receipt/hash/proof,
current closure snapshot/watermark, and the independently authenticated complete
empty LCFA pair. It revokes credentials, enqueues the exclusive FINALIZE resolution,
and sets finalized_at atomically. Cancellation uses the same lock, invalidates the
proof generation and enqueues CANCEL if a Close exists. Its UI says restoration
pending until the matching signed cancellation receipt and snapshot are retained;
old sessions are never restored. Cancellation before issuance needs no remote undo.

Recovery is a real transaction-bound adapter: it locks the native restore barrier,
denies unacknowledged restore notices, and checks immutable command/receipt history
against the explicitly configured reconciled nonrollback recovery floor. Unknown,
missing or downgraded generations stay pending. An operator must reconcile a Game
restore and supply its current floor; enabling a flag never performs reconciliation.
Projection gaps/conflicts remain quarantined, higher epochs withdraw old proof, and
freshness cannot repair conflicting evidence. No timeout guesses a Game outcome.

Owner defaults (decision 2026-11-06)
-----------------------------------

- Accept the candidate v2 command/private projection variants, reversible quarantine,
  Game domain restoration/final disposition and exclusive CANCEL/FINALIZE decisions.
- Keep request-time any-account_characters blocking. Accepting the candidate does
  not silently relax the request gate its text explicitly preserves.
- Grace: 14 days, overridable by existing PORTAL_ACCOUNT_TERMINATION_GRACE_DAYS (1..90).
- Command attempt TTL: 300 seconds; MAX_PENDING: 40; freshness: 30 seconds;
  uncertainty: 1 second. Config overrides may tighten TTL/backlog/freshness bounds.
- Worker interval: 5 seconds, configurable 1..60. Domain side effects are never
  executed on a page-render request or under an account-lock network exchange.
- FINAL_DISPOSITION_POLICY defaults to `game_verified`, overridable to `hold` to
  retain quarantine and deny FINALIZE while still allowing cancellation. This is
  the conservative override for unsettled physical/financial disposition.
- Physical erasure is Game-owned after authenticated FINALIZE, not a Platform timer.
  The conservative default is no Platform deletion of gameplay or financial facts.
  Outstanding bank funds/entitlements and unresolved guild/house/market obligations
  require Game's owner-domain disposition; absence of its committed proof blocks.
  There is no unsupported Platform policy field added to the immutable CC wire.
- Authoritative command, receipt, generation and non-reuse evidence is retained
  separately from the registry's P90D audit payload. The enabled worker expires
  only local audit copies, measured from the original Game decision, and preserves
  all authoritative receipts and generations. No retry resets retention.
  Domain financial retention and legal holds remain with their owning retention
  policy. AccountClosureDecisionV1 is staged atomically with the retained receipt;
  this consumer does not invent a Game EventEnvelope or become a Game audit producer.
- Conflicts require explicit operator reconciliation while closed/pending. No
  automatic repair, donation, forfeiture, name release or speculative refund is chosen.
- No recovery generation, publisher key or certificate identity has a fabricated
  default. Those are required deployment facts, not owner-policy decisions.

Configuration inventory and the exact local transport are in e2e/ENVIRONMENT.md.
The fixture-backed producer under e2e runs outside production, uses generated
throwaway certificates/keys, and asserts every pinned fixture hash. Playwright drives
registration, verification mail, login, sudo, CSRF, deletion request, elapsed grace,
real worker, mTLS Game publications, final account closure and login refusal in pl/en.
Its invalid-signature case proves pending finalization and cancellation after grace.
The test clock advances only its own original deadline before issuance, never proof.
Game kick/lease/domain race conformance still belongs to the real Game producer.

Rollback: disable the closure feature and retain authoritative replay state. Never
rewrite immutable commands or erase the only evidence of an already executed close.
