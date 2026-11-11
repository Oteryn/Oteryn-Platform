# Referral client contract

Status: **PROPOSED** for client navigation and game delivery. Portal campaign
rules and store qualification are implemented behind default-off flags.

## Client boundary

The game client only opens `/{lang}/account/referrals` on the configured portal
HTTPS origin in the system browser. Supported languages are `pl` and `en`. The
browser uses the portal session. No bearer tokens, payment evidence or reward
claims are placed in the URL. The client neither embeds the page nor grants
rewards. There is no new game packet, client callback or referral API.

## Activation and browser flow

`PORTAL_REFERRALS_ENABLED=true` enables the browser routes and reconciliation
worker. `referral_campaign_settings.enabled` is the authoritative campaign switch
for invitations and store qualification; the migration initializes it to false.
Disabling the switch prevents new rewards. Previously earned loyalty credits
remain part of loyalty snapshots. Existing store and premium flags still apply.
Enabling the browser requires `OTERYN_PUBLIC_ORIGIN` to be an HTTPS origin.

`/{lang}/account/referrals` requires a current active, verified portal account.
It displays the reusable link, new-player e-mail and bring-back forms, anonymized
accepted invitation statuses, pending entitlements, premium rewards and loyalty
credits. Every browser POST uses the existing session CSRF and origin checks.
The inviter always comes from the session. E-mail submission gives the same
response for eligible and refused recipients; it does not reveal account state.
The page uses Polish/English Fluent messages and responsive accessible components.

A reusable link or e-mail link opens `/{lang}/referrals/{code}`. Its confirmation
POST stores attribution in the existing browser session and opens registration.
Verification and authenticated session assignment synchronously bind the eligible
new account in the same database transaction, before its first store purchase.
Both verification/sign-in orders are supported. Verification captures attribution
before credential-change triggers revoke browser sessions; session revocation remains
in the same transaction. Session rotation preserves pending attribution. The worker reconciles legacy pending sessions and removes completed,
permanently refused or malformed candidates; transient failures retain evidence
for retry. Eligibility storage errors return unavailable, never permanent denial. Unverified accounts remain pending and do not consume the batch.
A signed-in eligible new account can confirm directly. Returning accounts are already bound at invitation time; signing
in and opening their link confirms the existing binding without a second grant.

A new account must have been created after the link/invitation and within seven
days of acceptance, have no previous store purchase, and match the invited
address for an e-mail invitation. Unlimited links have no recipient cap.
Self-referrals, cycles, reciprocal referrals, duplicate attribution to a different
inviter, inactive/unverified accounts and current e-mail-change trade restrictions
are refused. The existing referral Policy interface remains mandatory. Old
referral participation also prevents duplicate campaign attribution. Legacy invitations
check the combined legacy/campaign ancestry and reject already-attributed invitees.

## Limits and activity

E-mail invitations permit at most 25 sends per inviter in a rolling 24-hour
window, including resends. Addresses are trimmed and normalized to lowercase.
Resends are permitted at least 14 days after the first attempt for that inviter,
recipient and invitation kind. Each actual send creates a durable attempt.
Enqueue checks use attempt creation times. Delivery independently rechecks the
rolling budget using successful delivery times, under a shared transaction lock
held through send and commit across nodes. This delivery lock (5601) is separate
from referral qualification (5600), so SMTP latency does not block purchases.
Queued attempts remain pending when the delivery budget is exhausted, so an SMTP backlog cannot bypass the limit.
Other inviters remain eligible for delivery.

Bring-back invitations additionally permit 20 sends in a rolling 30-day window,
checked both at enqueue and delivery.
The recipient must have no website or game activity in the preceding 12 calendar
months. Durable activity high-water marks are initialized from accounts, website
sessions, login history and the latest of game-session entry/end times, then
maintained by triggers on session inserts and end-time updates so ordinary history
retention cannot make an active account appear inactive. Authenticated game-login
promotion also advances this mark before discarding an expired presentation
payload: a session that began over twelve months ago but ended recently remains
recent activity without restoring the expired history row.
A missing activity row or less than 12 months of recorded coverage denies
eligibility. The activity row's creation time is the start of that coverage;
retained game-login history alone (30 days) never proves twelve-month inactivity.
Existing accounts therefore need twelve months of capture after migration, or a
trusted historical coverage backfill, before bring-back eligibility is known.
The original invitation time and eligibility remain fixed across resends and
returning logins.

## Authoritative qualification and rewards

| Trigger | Beneficiary | Effect |
| --- | --- | --- |
| Eligible new account attribution | Invitee | Cosmetic entitlement for the first three created characters, expiring seven days after account creation |
| First paid store purchase by the first qualifying invitee | Inviter | Recruiter outfit entitlement |
| Three distinct paying invitees | Inviter | Recruiter addon 1 entitlement |
| Ten distinct paying invitees | Inviter | Recruiter addon 2 entitlement |
| Each six premium-consumption loyalty points earned by an invitee | Inviter | One durable loyalty credit; retain the remainder |
| First character login after a bring-back invitation | Returning account | Welcome-back present for that exact character |
| First qualifying purchase of 30–89 premium days within 90 days | Inviter | Five premium days |
| First qualifying purchase of at least 90 premium days within 90 days | Inviter | Ten premium days |

The accepted store purchase transaction calls qualification after its wallet
debit and delivery records. Refused purchases and rolled-back transactions grant
nothing. Distinct paying invitees are counted once. A buddy's first qualifying
purchase fixes its tier; later purchases do not add another award or extend the
90-day deadline. Separate purchases are not accumulated toward a tier.

Both snapshot reads and stream tail polls supply premium-only consumption points before
adding referral credits. Only new-account invitations earn loyalty credits; their
lock discovery matches that eligibility rule. Credits never recursively generate credits.
`referral_loyalty_credits` is keyed by operation and survives subsequent snapshot
recomputation. Snapshot shape, byte encoding and revision behavior are unchanged.
The premium ledger applies buddy rewards using the durable reward UUID as its
operation key, in the same transaction as the applied marker. The server-side
premium entry point verifies a stored campaign award; browser input cannot choose
an account, duration or administrative privilege.

Game login qualification uses the existing promoted `pd_game_logins` path. A
unique welcome operation binds the first observed qualifying character once.
All game rewards are recorded in the existing outbox as `referral.entitlement`,
with a unique operation key, entitlement UUID, account, kind, quantity, character
scope, optional exact character and expiry. Starter scope is
`FIRST_THREE_CREATED`: order characters by authoritative creation time, breaking
ties by character UUID; quantity is three and expiry is account creation + seven
days. Outfit scope is `ACCOUNT`; welcome scope is `EXACT_CHARACTER`.

Game delivery is pending, not acknowledged. These records have `run_after` set to
infinity until an approved game delivery adapter releases them. The ordinary
worker cannot consume them or mistake them for completed delivery. No game wire,
product identifier or direct gameplay mutation is invented by this contract.

## Mail delivery and recovery

The campaign worker uses the existing notification SMTP sender. Each attempt has
a stable message UUID, a stored delivery marker, and a row lock across sending.
SMTP is at least once after a crash between sending and committing the marker.
Unsent attempts remain durable when SMTP is unconfigured or unavailable. The
worker processes bounded batches and retries on subsequent schedule slots.
Premium awards and account attribution are reconciled independently of SMTP.

## Open item

shared payment fingerprint refusal: deferred, no trusted fingerprint in PaymentEvent

## Transaction lock order

Every qualification path discovers its complete account set before locking:
store purchases, loyalty snapshots/streams, campaign link/email/accept/signup,
and worker attribution. Acquire account rows FOR UPDATE (including inviter, invitee
and every subsequent account foreign-key target) in ascending account UUID order,
then existing premium_authority rows FOR UPDATE in ascending account UUID order, then
qualification advisory lock 5600. Preparation takes the strongest account and
existing authority locks up front,
so later authentication, policy reads and FK checks cannot upgrade them. Missing
authority rows needed for publication are inserted before 5600. No new account or
premium_authority row/FK lock may be acquired after 5600.
The shared database lock function rejects an expanded account set with SQLSTATE
40001 so a stale discovery retries in a new transaction rather than reversing the
order. Rust purchase, snapshot, stream and campaign operations (including worker
attribution) retry the entire transaction, from discovery through commit, at most
three times, only for 40001 (serialization failure) or 40P01 (deadlock). Attempts
use a short jittered backoff; exhaustion returns the existing unavailable response.
Purchase retries reuse the operation key, and committed replays return the original
answer without another debit, order or reward. Other failures are not retried.
Row triggers inherit account locks from their caller, which may already hold the
invitee before the older inviter. Shared preparation therefore uses NOWAIT for
all account and existing authority rows when invoked by a trigger, translating
contention to SQLSTATE 40001. It never waits in the inherited, potentially reversed
order. Ordered callers prepare the complete set before updating the account.
Session and verification triggers propagate preparation conflicts and expanded
sets as SQLSTATE 40001. Failed login rolls back authenticated account assignment
and preserves the original session attribution for a fresh transaction retry; it
must never commit login without binding an eligible invitation. Worker
reconciliation is only a fallback for legacy pending sessions, since a purchase
before reconciliation makes a previously unbound account ineligible.
Session attribution runs after the password-reset
account-lock guard. Verification attribution runs before credential-change session
revocation and must propagate 40001 (and deadlocks), rolling back the entire
verification including token consumption and session deletion. Retry verification
in a fresh transaction; a conflict must never commit verification without attribution.
Stream preparation also precedes account adapter reads so their SHARE/UPDATE locks
cannot introduce an upgrade cycle.
Worker mail delivery holds only the separate delivery lock 5601 across SMTP;
5600 is never held across network I/O. Referral-premium reconciliation uses the
ordinary premium ledger and does not acquire 5600.
