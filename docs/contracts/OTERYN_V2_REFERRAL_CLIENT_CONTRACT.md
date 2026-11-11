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
Both verification/sign-in orders are supported. Session rotation preserves pending
attribution. The worker reconciles legacy pending sessions and removes completed,
permanently refused or malformed candidates; transient failures retain evidence
for retry. Unverified accounts remain pending and do not consume the batch.
A signed-in eligible new account can confirm directly. Returning accounts are already bound at invitation time; signing
in and opening their link confirms the existing binding without a second grant.

A new account must have been created after the link/invitation and within seven
days of acceptance, have no previous store purchase, and match the invited
address for an e-mail invitation. Unlimited links have no recipient cap.
Self-referrals, cycles, reciprocal referrals, duplicate attribution to a different
inviter, inactive/unverified accounts and current e-mail-change trade restrictions
are refused. The existing referral Policy interface remains mandatory. Old
referral participation also prevents duplicate campaign attribution.

## Limits and activity

E-mail invitations permit at most 25 sends per inviter in a rolling 24-hour
window, including resends. Addresses are trimmed and normalized to lowercase.
Resends are permitted at least 14 days after the first attempt for that inviter,
recipient and invitation kind. Each actual send creates a durable attempt.
A shared transaction lock serializes checks and budget consumption across nodes.

Bring-back invitations additionally permit 20 sends in a rolling 30-day window.
The recipient must have no website or game activity in the preceding 12 calendar
months. Durable activity high-water marks are initialized from accounts, website
sessions, login history and the latest of game-session entry/end times, then
maintained by triggers on session inserts and end-time updates so ordinary history
retention cannot make an active account appear inactive.
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

The existing loyalty service supplies premium-only consumption points before
adding referral credits. Credits therefore never recursively generate credits.
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
