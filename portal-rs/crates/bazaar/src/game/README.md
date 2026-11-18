# Bazaar Game boundary

[PLAT3.md](PLAT3.md) documents the consumer of Game CANDIDATE rev 1 at
`3b49a20feec3ba29700bb4b2fa28fa9b539b79a2` and owner acceptance on 2026-11-06.
The portal and worker select real adapters with `OTERYN_BAZAAR_ENABLED=true`;
default-off composition keeps the existing unavailable response.

The private mTLS listener implements the six Game-initiated exchanges, extends
the single Character receipt facility and validates shared ANL envelopes.
Status, watermark, HTTP success, timeout and absence never prove eligibility,
custody or delivery. Parameterized SQL runs in caller-owned transactions;
adapters perform no remote I/O inside them.

## Owner defaults

Accepted proposals cover v2 receipts, obligations, snapshot survival, Inbox/item
classes, Gold's 250 bps ceil fee/headroom and common restore scope. Conservative
configurable choices are 30-day Character cooldown and 3650-day proof retention.
Escrow identity, history coverage, nonrollback fence and public signing keys must
be explicitly provisioned. See [the complete Owner defaults](PLAT3.md#owner-defaults)
and [runtime configuration](../../../../e2e/ENVIRONMENT.md).

## Lifecycle state table

This inventory describes Platform at `55b790ff`. Listing state, Game custody,
command disposition, moderation and the recovery fence are separate pieces of
state. For example, a completed character sale can still have a pending Unlock,
and a draft can already hold a Game asset. A STATUS projection alone cannot
prove that any movement committed.

Paths in the tables are relative to this directory unless shown from the
repository root. Line numbers refer to the base commit.

### State and write sites

| State or condition | Current writers and activation rules |
| --- | --- |
| Listing absent → `draft` | Game auction creation writes the listing and consent, then stages Lock or Escrow in the same transaction (`runtime/mod.rs:356–368`). Legacy constructors share the default (`../repository.rs:116`, `../items/service.rs:103`, `../characters/service.rs:155`). |
| `draft` → `active` | The Game worker requires fresh, retained escrow and listing permission (`runtime/worker.rs:343–374`). Legacy activation writes at `../repository.rs:147`, `../items/service.rs:139`, `../characters/service.rs:212`. |
| `matched` | Legacy offer acceptance, fixed-price purchase or want-to-buy matching writes it (`../repository.rs:424`, `../items/service.rs:396,508,541`). The Game auction worker has no writer for this state. |
| `active` → `awaiting_delivery` | Coin settlement binds funds and records the sale (`../repository.rs:620`). Gold settlement records the sale and hold revision (`runtime/gold_worker.rs:344–352`). |
| `awaiting_delivery` → `completed` | Coin delivery captures reserved funds in the completion transaction (`../repository.rs:638–662`, `coin_ports.rs:462–468`). Gold marks the sale and listing complete after retained Payout (`runtime/gold_worker.rs:453–459`). |
| `cancelled` | Item escrow refusal, refused character escrow Bind after Unlock, ordinary completed return, approved invalidation, and completed settlement compensation write it (`runtime/worker.rs:215–249,884–895`, `runtime/invalidation.rs:147–162`, `authority_exchange.rs:342–349`). Legacy cancellation writes at `../repository.rs:497` and `../items/service.rs:574`. |
| `expired` | Legacy expiration writes at `../repository.rs:534` and `../items/service.rs:657`. Game auctions stay active while an unsold return is pending and end as cancelled. |
| Listing flags | Seller cancel sets `cancel_requested`. Resume updates the session hash of a non-refused asset. The worker records refusals and marks a seller-balance overflow for cancellation after releasing the coin reservation (`runtime/mod.rs:583–606`, `runtime/worker.rs:208–224,389–401`). |
| Seller custody with no Bazaar record | Initial state, or a deposit with current audited no-effect evidence. A refused or never-issued deposit can have no custody ID. The resync listing scan exempts applicable rows (`authority_evidence.rs:43–66`). |
| Item `ESCROWED` | Committed Escrow plus matching STATUS proves the deposit. The worker stages Escrow under the published policy. Item eligibility and tradeability are checked independently of custody (`runtime/worker.rs:98–145`, `authority_evidence.rs:265–388`). |
| Item `DELIVERED` / `RETURNED` | Committed Deliver or Return and matching STATUS place the item in buyer or seller custody. Conflicting committed dispositions for one escrow ID are quarantined (`runtime/worker.rs:428–459,486–522,844–879`, `authority.rs:161–175`, `authority_evidence.rs:265–388`). |
| Character `locked` | Lock receipt supplies a signed snapshot bound to the issuance epoch. STATUS remains `locked` while committed CC1 escrow, settlement and return transfers change the custodian among the seller, configured escrow account and buyer (`authority.rs:239–266`, `runtime/worker.rs:61–87,597–781,917–965`, `runtime/characters.rs:204–339`). |
| Character `DELIVERED` / `RETURNED` / `UNLOCKED` | STATUS can report these states. Return or settlement needs CC1 proof and Unlock needs the retained lock, status revision and no conflicting use. The worker may issue Unlock after the commercial listing is terminal (`game_projection.rs:97–133`, `runtime/characters.rs:295–374,488–545`, `runtime/worker.rs:468–470,783–822`). |
| `INVALID`, missing, stale, superseded or quarantined custody | STATUS can store `INVALID`; missing or stale evidence is unavailable. Lower epoch/revision is superseded, equal matching content is duplicate, and equal conflicting content quarantines the subject (`game_projection.rs:97–133,249–304`, `authority_evidence.rs:129–202,265–288`). |
| Deposit, Lock, Escrow, Bind, Deliver, Return, Unlock | Deposit is the worker action that chooses Lock for a character and Escrow for an item (`runtime/worker.rs:53–145`). Deliver is staged after funding, Return after safe cancellation or compensation, and Unlock after the character handoff or certified no-effect. Character movements first stage an unpublished CC1 intent, issue one of the three purpose-bound Binds, then publish CC1 after a committed Bind (`runtime/worker.rs:428–459,486–529,687–781,783–879`). |
| Intent absent → `STAGED` → dispatched | Staging writes immutable bytes in `bazaar_game_outbox`. Dispatch is a separate row, which makes the intent visible to Game. The command state API reports a dispatched nonterminal attempt as `PENDING` (`game_store.rs:98–135`, `authority_exchange.rs:11–179,384,428–450`). |
| Intent `REOPENED`, retired, or replaced | Reopening updates old-fence outbox state without rewriting the payload. Cleanup advances its attempt pointer only on audited rejection. Character transfer retirement is append-only; eligible attempts can be replaced under a new operation ID (`game_store.rs:139–158`, `runtime/mod.rs:128–201`, `runtime/worker.rs:551–561,687–705`, `runtime/characters.rs:571–634`). |
| Intent terminal `COMMITTED` or `REJECTED` | The receipt stores the Game result. A second authority decision supplies the audit required by `retained`. Pending, expired without a receipt, lower-generation, and quarantined attempts cannot prove an effect or no effect (`authority.rs:118–187,448`, `authority_evidence.rs:227–263`). Exact replay returns `Duplicate`, valid first admission returns `Stored`, conflict or a committed-to-rejected reversal returns `Quarantined`. Malformed, unbound, unauthorized or unknown operations are refused. |
| Gold Hold / Release / Payout | Bid phases are `pending`, `accepted`, `releasing` and `refused`. Hold tracks amount/revision, and a full release or payout consumes the hold. Hold is staged with a bid; Release handles outbid, excess or cleanup; Payout follows proven delivery (`runtime/gold_worker.rs:72,133–156,211–268,274–302,344–459`, `runtime/gold.rs:12–77,151–251,298–414`). |
| Coin reservation / disposition | Reservation amount follows the winning bid. Settlement attaches the hold to a disposition, then exactly one capture or release closes it (`coin_ports.rs:250,315–334,392–397,462–468`). |
| Moderation clear → held, requested, approved → clear | Suspend creates or replaces a hold revision. Request binds that revision; a second actor approves and enqueues reconciliation. Approval does not itself move custody. The worker retires the exact hold after safe invalidation, compensation, or verified irreversible handoff (`../moderation.rs:473–530`, `runtime/invalidation.rs:42–163`, `authority_exchange.rs:342–348`, `runtime/worker.rs:285–311`). |
| Projection fence unknown, matching or stale | The issuer advances monotonically. New projection epoch reopens old operations and makes all earlier projections ineligible. A new restore notice fences the runtime under the shared Character restore lock; the runtime also checks its fence file and the issuer row (`game_projection.rs:247–260`, `game_store.rs:139–158`, `authority.rs:81–113`, `runtime/config.rs:172–182`). |
| Resync required → scanning → finished | Require policy for every configured world and current STATUS for known custody. Check the referenced receipt or CC1 proof, then scan retained operations. Listing and operation cursors commit bounded batches and completion inserts a fence-specific marker. Serving and trading additionally require a fresh watermark. While unready, an already-issued item Deliver can be read for reconciliation only after expiry plus five seconds (`authority_evidence.rs:13–127,204–225`, `runtime/worker.rs:16–42`, `authority_exchange.rs:388–395`). |

### Transition outcomes

CORRECT means the observed outcome follows the current contract, including
refusals. DEFECT means the behavior at the base disagrees with the contract.
The names below are the suffixes of the PostgreSQL tests
`lifecycle__<state>__<event>`. Each table identifies the state dimension the
entry point consumes. Where an event is shared, the event's preconditions
identify the relevant custody, funding or moderation variant.

#### Listing requests

Each cell keeps the listing's enum value. These are fresh request keys with
authorized actors, matching fences and available evidence. An awaiting-delivery
suspend in this table has no dispatched handoff. A matched or expired fixture
represents an existing legacy state and exercises the shared request boundary.

| Listing | Bid | Seller cancel | Staff suspend |
| --- | --- | --- | --- |
| Draft | CORRECT · refuse, auction inactive | CORRECT · request return | CORRECT · create hold |
| Active | CORRECT · reserve and change leader with buyer ownership | CORRECT · request return with no leader | CORRECT · create hold |
| Matched | CORRECT · refuse, auction inactive | CORRECT · refuse, settlement already matched | CORRECT · create hold |
| Awaiting delivery | CORRECT · refuse, auction inactive | CORRECT · refuse, settlement in progress | CORRECT · refuse before dispatch |
| Completed | CORRECT · refuse, asset no longer in escrow | CORRECT · refuse, terminal | CORRECT · refuse, terminal |
| Cancelled | CORRECT · refuse, auction inactive | CORRECT · refuse, terminal | CORRECT · refuse, terminal |
| Expired | CORRECT · refuse, auction inactive | CORRECT · refuse, terminal | CORRECT · refuse, terminal |

Bid checks destination ownership before escrow eligibility and auction state
(`runtime/mod.rs:414–453`, `../repository.rs:167–213`). Seller cancel checks
safe return, owner, lifecycle, leader and pending gold releases
(`runtime/mod.rs:557–606`). Suspension locks the listing and checks the
handoff before creating the hold (`../moderation.rs:446–474`).

#### Custody STATUS publication

These events update the Platform projection. A newer STATUS replaces the
stored projection. A lower revision is superseded. A same-revision exact
replay is a duplicate. A same-revision different payload quarantines the
subject while preserving the previous bytes. Each case leaves the commercial
listing unchanged. The comparison uses the same ordering for each wire label
(`game_projection.rs:97–133,249–304`).

| Custody STATUS | Newer | Older | Replay | Conflict |
| --- | --- | --- | --- | --- |
| `locked` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |
| `ESCROWED` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |
| `DELIVERED` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |
| `RETURNED` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |
| `UNLOCKED` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |
| `INVALID` | CORRECT · Applied | CORRECT · Superseded | CORRECT · Duplicate | CORRECT · Quarantined |

#### Game command reconciliation

`read_state` calls the Game reconciliation exchange for the specific
operation. The exchange reads dispatch, outbox and current-generation receipt
rows (`authority_exchange.rs:398–455`).

| Intent | Read state |
| --- | --- |
| `intent_staged` | CORRECT · omitted from the response because no dispatch exists |
| `intent_pending` | CORRECT · PENDING |
| `intent_committed` | CORRECT · COMMITTED |
| `intent_rejected` | CORRECT · REJECTED |
| `intent_reopened` | CORRECT · REOPENED until a current result exists |

#### Worker, authority and recovery events

| State | Event | Outcome at the base | Classification |
| --- | --- | --- | --- |
| `absent` | `create` | Authorized create commits a draft and a dispatched deposit intent atomically (`runtime/mod.rs:284–369`). | CORRECT |
| `draft` | `pending_lock` | Stays draft while Lock is pending. No transfer Bind publishes before the committed Lock proof (`runtime/worker.rs:343–348`). | CORRECT |
| `draft` | `advance_escrowed` | Becomes active after current escrow and seller authorization (`runtime/worker.rs:343–374`). | CORRECT |
| `draft` | `advance_cancel_requested` | Dispatches Return even if item sale policy is absent. The retained Return closes the draft (`runtime/worker.rs:320–341,884–895`). | CORRECT |
| `active` | `expire_unbid` | Dispatches Return and keeps the listing active until the committed return closes it (`runtime/worker.rs:376–380,825–895`). | CORRECT |
| `active` | `expire_held` | Retains custody and leaves listing active. Suspension has no approved invalidation (`runtime/worker.rs:156–164`). | CORRECT |
| `active` | `expire_funded` | Moves to awaiting delivery and fences the buyer reservation (`runtime/worker.rs:382–404`, `../repository.rs:620`). | CORRECT |
| `active` | `invalidate` | Approved hold revision allows Return. Committed return completes cancellation and retires the hold (`runtime/invalidation.rs:42–163`). | CORRECT |
| `awaiting_delivery` | `commit_delivery` | Commits completion and coin capture once (`runtime/worker.rs:406–466`, `../repository.rs:638–662`). | CORRECT |
| `awaiting_delivery` | `refuse_delivery` | Stays awaiting while Return is pending, then refunds and cancels (`runtime/worker.rs:226–228,476–529`, `authority_exchange.rs:318–349`). | CORRECT |
| `completed` | `worker_tick` | Completed item stays completed. No new asset intent issues (`runtime/worker.rs:468–472,908–914`). | CORRECT |
| `cancelled` | `worker_tick` | Cancelled item stays cancelled. No new asset intent issues (`runtime/worker.rs:468–472,908–914`). | CORRECT |
| `receipt_committed` | `replay` | Identical bytes return Duplicate. Different valid bytes for the same operation and generation return Quarantined (`authority.rs:138–146`). | CORRECT |
| `receipt_absent` | `stale_generation` | First admission at a lower generation returns Conflict without a receipt or listing update (`authority.rs:148–151`). | CORRECT |
| `active` | `bid_gold` | Dispatches Hold and persists the pending bid. No leader is admitted before receipt (`runtime/gold_worker.rs:6–73,145–161`). | CORRECT |
| `gold_pending` | `commit_hold` | Accepts the bid from the retained Hold and sets the leader (`runtime/gold_worker.rs:145–252`). | CORRECT |
| `gold_pending` | `refuse_hold` | Refuses the bid without a leader or release of nonexistent funds (`runtime/gold_worker.rs:145–161`). | CORRECT |
| `active` | `bid_foreign_destination` | Refuses before reserving funds, because the destination does not belong to the buyer (`runtime/mod.rs:414–438`). | CORRECT |
| `draft` | `refuse_escrow` | Refused item deposit closes the draft. A later epoch can finish resync without a custody ID (`runtime/worker.rs:208–225`, `authority_evidence.rs:43–66`). | CORRECT |
| `completed` | `unlock_character` | Preserves completed and dispatches one Unlock after committed handoff (`runtime/worker.rs:468–470,783–822`). | CORRECT |
| `fence_stale` | `restore_notice` | Refuses settlement work until the configured generation catches up with the accepted restore notice (`authority.rs:90–111`, `runtime/worker.rs:18–39`). | CORRECT |
| `fence_matching` | `new_epoch` | Current projections advance issuer epoch and reopen prior intents. The old fence is unavailable and the new fence resyncs with retained same-generation receipts (`game_projection.rs:247–261`, `game_store.rs:139–158`, `authority_evidence.rs:45–127`). | CORRECT |

| State | Event | Classification and reason |
| --- | --- | --- |
| Draft | Create same listing | N/A · create targets absence. Exact key replay returns the same row without running a new transition. |
| Active | Create same listing | N/A · same request binding. |
| Matched | Create same listing | N/A · same request binding. |
| Awaiting delivery | Create same listing | N/A · same request binding. |
| Completed | Create same listing | N/A · same request binding. |
| Cancelled | Create same listing | N/A · same request binding. |
| Expired | Create same listing | N/A · same request binding. |
| Matched | Game worker tick | N/A · legacy matching is outside the Game listing batch (`runtime/worker.rs:908–914`). |
| Expired | Game worker tick | N/A · legacy expiry is outside the Game listing batch (`runtime/worker.rs:908–914`). |

Receipt events key by operation and projection epoch events key by issuer floor.
These events use the corresponding authority or projection state in the table.

### Review packet at this base

The review packet cites older source positions. The exact base includes
`ee927b40`, `487b2f55`, `efe543ba` and `35743571`. The tests below are existing
coverage of narrower recovery variants, in addition to the lifecycle cells
above.

| Review IDs | Behavior at the base and existing coverage |
| --- | --- |
| f04-bazaar-item#1, #4, #5, #6 | Held expiration retains custody, foreign coin destinations are refused, cancellation bypasses sale policy, and a rejected item deposit closes. See the corresponding lifecycle cells. |
| f04-bazaar-item#2, f07-bazaar-recovery#2 | The no-effect deposit does not need a custody STATUS during resync. See the refused escrow cell and `recovery_tests.rs:697`. |
| f05-bazaar-character#2, f07-bazaar-recovery#3 | Late Lock admission uses the issuance epoch. See `recovery_tests.rs:554`. |
| f05-bazaar-character#3, f07-bazaar-recovery#4 | Same-generation CC1 proof survives an epoch-only rebuild. See `recovery_tests.rs:593`. |
| f05-bazaar-character#4, #5, #6 | Stale return Bind can be replaced, approved invalidation can publish a committed escrow Bind, and retired keys remain available for admitted Lock proof. See `invalidation_tests.rs:367,398,429`. |
| f06-bazaar-gold#1, f07-bazaar-recovery#8 | Receipt binding permits five seconds of issue skew. See `recovery_tests.rs:611,749,775`. |
| f06-bazaar-gold#3, f07-bazaar-recovery#7 | Restored rejected delivery or payout with a prior committed receipt quarantines the operation and prevents resync completion. See `recovery_tests.rs:631–692`. |
| f06-bazaar-gold#4, f07-bazaar-recovery#5 | First settlement Bind in recovery can validate without the resync marker. See `recovery_tests.rs:806`. |
| f06-bazaar-gold#5, w2-bazaar#2 | A first terminal-refusal Release can validate after return reattestation. See `recovery_tests.rs:727`. |
| f06-bazaar-gold#6, f07-bazaar-recovery#6 | A recovering payout can use the retained CC1 transfer despite changed receipt bytes. See `recovery_tests.rs:794`. |
| f06-bazaar-gold#10 | An old snapshot is filtered from the active catalogue during recovery instead of making the page unavailable. See `authority_tests.rs:2910`. |
| k01-epoch-binding#5 | Retirement remains immutable and the current refused Bind supplies the generation-specific evidence. See `authority_tests.rs:5376`. |
| k01-epoch-binding#6 | Accepted restore notice fences Bazaar while the configuration is behind. See the restore notice cell and `recovery_tests.rs:570`. |
| k01-epoch-binding#8 | Store delivery intake and its restore fence are Store entry points and have no Bazaar listing transition. Store now compares the receipt generation with acknowledged Character restore notices (`portal-rs/crates/store/src/delivery.rs:181–189`). |
| f06-bazaar-gold#11 | Identity checks active wallet reservations and current listing owner/leader. The pending outbid Gold release actor is still absent from this predicate. Closure admission is outside the Bazaar state table (`portal-rs/crates/identity/src/account_lifecycle_http.rs:48–64`, `portal-rs/crates/identity/src/termination/game_closure/runtime.rs:249–264`). |
| f05-bazaar-character#7 | Character Services still checks pending CC1 commands without a Bazaar outbox lookup. Service admission is outside the Bazaar state table (`portal-rs/crates/characters/src/services/mod.rs:174–191`). |
