# Transaction locks and live mutation authority

Start `mutation_budget` **before the first lock** on every advisory-taking transaction
and every transaction that will join RBAC, including shared operator issuers. Repeated calls retain the same
three-second database-clock deadline and preserve any shorter caller timeout.
Configure transaction isolation before the budget (and before any query). Acquire on the caller's connection and
retain locks through domain changes, audit and commit. A fresh transaction must
never inherit another transaction's deadline.

The hierarchy is a precedence table, not numeric advisory-key order. Independent
domains omit the levels they do not use. Reacquisition of an already-held lock
is harmless; upgrading a shared lock while holding a later resource is forbidden.
New, private rows are written after acquisition; they are not existing resource
rows to wait for. Resolve immutable ownership without a lock, acquire the full
account set in UUID order, then verify the binding again under resource locks.

| Order | Lock / namespace | Rule and takers |
|---|---|---|
| 1 | Bazaar moderation gate `64002` | Shared for all Bazaar transaction entry points, **before account adapters**, listing locks and issuer rows. Exclusive for **every** staff command before `lock_staff_target`; never upgrade shared to exclusive. This separates listing-first trades (account `FOR SHARE`) from staff account `FOR UPDATE`. |
| 2 | Pre-account domain/request gates; Bazaar existing listing / command target rows; gift-code resources | Listing, then invalidation request/report. Stored control evidence prepares its execution and referenced preview request rows together in UUID order before actor/RBAC locks. Staff prepares these before actor/RBAC. Traders retain the listing before locking its discovered participants; creation inserts a new private listing. Gift redemption takes request key 55 → code row → batch/account key 56 before account rows on both browser and internal paths. Player-data request/account serialization 73 → source 72 → record/session 74/729, and places publication gate 81005230117, precede identity account admission; their projection rows follow it. These request gates are distinct from character issuance 22005160000 at level 4. |
| 3 | `accounts` rows: `FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`, `FOR KEY SHARE` | UUID order for the entire participant set; strongest required mode first. Bazaar Accounts::prepare includes previous leaders, selected offers and the bounded refund batch before any single-account check. RBAC and role writers use `FOR UPDATE`; account eligibility ports use `FOR SHARE`; character creation uses `FOR NO KEY UPDATE`. No late upgrade. |
| 4 | Domain resource acquisition, including character issuance `22005160000` → restore fence → command/projection rows | Events, announcements, news/media/pages, wiki guides/slug reservation, world publication pointers, support tickets/sanctions/workflows, MFA reset evidence and target-session rows. Prepare existing targets through `authorize_mutation_with_resources*`, or revalidate on the same transaction after a retained target wait. |
| 5 | RBAC fence `71010` | Exclusive canonical mutation/role writer; shared bootstrap operator reader. Account and prepared domain locks precede this fence. Shared issuance retains this fence while acquiring the bootstrap authority, under the same acquisition deadline. |
| 6 | Durable session and MFA rows | Session → enrollment → grant; lock first, then evaluate database-clock expiry, security generation, password binding, account eligibility and permissions. Revalidate after subsequent resource waits before accepting effects. |
| 7 | Bootstrap authority, catalog activation pointer, MFA reset request rows, domain effects, ledgers, outboxes and audit | Bootstrap issuance joins RBAC before its authority row and checks permission after that wait. Catalog activation joins RBAC before waiting on its pointer; purchases take accounts then the pointer and never join RBAC. Retain the earlier authority/resource locks. Any additional blocking adapter call must be followed by same-transaction revalidation before commit. |

## Advisory-lock inventory

This table includes keys in SQL migrations (including historical/replaced trigger
bodies). `hash(seed)` means `hashtextextended(binding, seed)`; bindings are scoped
by the owning domain. Different domains do not acquire each other's namespaced
request keys. Within an operation, request serialization precedes its existing
row locks; account sets and wallet sets are ordered by UUID.

| Namespace / keys | Protected resource and acquisition order |
|---|---|
| `64002` | Hierarchy level 1. Migration `20261115110004_fix_04_scheduling.sql` installs statement gates **before implicit listing/bid row locks**, and an INSERT row gate. `20261105230107_t64_moderation.sql` contains the earlier row trigger. Application entry acquisition makes every trigger reacquisition shared and reentrant. Applied migrations are unchanged. |
| `64001`; hash `64` | Bazaar fee revision / report request serialization before corresponding target writes. |
| hash `57`, `59`, `60`, `61`, `64004`, `64005` | Bazaar offer, character/item request, search, destination and subscription serialization, under the moderation gate. |
| `22005160000` (also `22_005_160_000`) | Character command publication/receipt and player-data ownership fence; before restore/command issuance rows. Accounts always precede this fence, including web creation, service submission, G-9 admission, G-10/G-11 admission and both journal read adapters. Issuance precedes restore protection and command/projection rows. Receipt settlement discovers and locks its account before restore and command rows. Also taken by the character-command publication trigger in `20261115110004_fix_04_scheduling.sql`. |
| hash `46`, `47` | Character request/actor serialization before character command/ledger writes, under restore protection. |
| hash `71102`, `12`; `20261105220206` | Page slug, media name, wiki slug reservation; canonical account → resource callback → RBAC. |
| `281510004` | Catalog CLI audit serialization in its separate audit transaction. |
| `5600`, `5601` | Referral graph/campaign serialization before referral rows. The campaign `5600` trigger in `20261111140004_n012_campaign.sql` is included. |
| hash `27` | Premium: account → premium authority row → request serialization, both standalone and caller-owned staff effects. Staff retains the actor/subject account set and RBAC and revalidates after the adapter returns. |
| hash `55`, `56` | Gift redemption: request 55 → code row → batch/account 56 → account (browser UPDATE, internal reward adapter). Browser admission is unlocked; active/verified status is rechecked after both resource waits. |
| hash `29`; `(29,1)` | Purchase and delivery serialization before owning domain rows; staff effects retain their prepared account set and RBAC. |
| `65001`; hash `70` | Statistics promotion/retention singleton and watch request, before corresponding statistics rows. |
| hash `72`, `73`, `74`, `729`; `81005230117` | Player-data source/session/cut/share/places serialization, then projection rows. Ownership reads retain the shared character issuance and restore fences. |
| hash `83` | Party post owner/request serialization before post writes. Staff moderation prepares the post through the RBAC resource callback. |
| Session lease `94000004`; hash `91`, `92`; `94000003` | Collection PvP request/character and league lifecycle serialization before game/league rows. Shared league readers never upgrade. |
| hash `90105`; hash `0` of `payment_provider_webhook:` binding | Coin purchase claim / provider webhook request before payment/claim effects. |
| hash `0` email/identity/abuse bindings; hash `0` of `native-recovery-locator:` | Identity registration/abuse and native locator serialization before corresponding identity/locator rows. |
| Arbitrary signed 64-bit admission key | First eight digest bytes in `gameauth/admission/store.rs`; request serialization before admission authority rows. |
| `715110005` | Native signing trust publication before trust/rotation rows. |
| hash `0` of `admin-control-ownership`, `admin-control-http`; hash `17` | Control ownership → HTTP ownership → scope/request serialization; no RBAC acquired by workers. Staff console revalidates after ownership waits. |
| `71140001`, `71140002` | Analytics ingest/retention and migration schema serialization before analytics fence/watermark rows; SQL up/down migrations included. |
| hash `15120016` reminder JSON array | Reminder idempotency trigger in `20261115120016_fix_a2-15-migrations_reminders_and_expiry.sql`, before reminder uniqueness checks. |
| Test-only `600`, `64006`, `326787645`, hash `90101`, `hashtext` webhook keys | Fixture barriers/ledger serialization; not production lock participants. Tests that deliberately hold `71010` are blockers, not mutation implementations. |

The additive migration `20261116041001_a3_drc_lock_budgets.sql` replaces live
advisory-taking trigger/function bodies with budgeted versions. Historical bodies
remain in the inventory as provenance, not additional live implementations.
The SQL `oteryn_mutation_budget()` function implements the same transaction-local
deadline and shorter-timeout rule as `oteryn_db::mutation_budget`; application
entry points still establish it before any earlier explicit resource lock.

## Authorization after waits

`authorize_mutation_with_resources*` acquires ordered accounts, runs the bounded
resource callback, joins RBAC, and checks current session/MFA/account/permission
state on that connection. Plain `Authorizer::check` is admission-only. It cannot
justify a write after a database wait. `MutationAuthorization` carries the trusted
adapter, not an authorization result cached before a wait.

Retaining RBAC prevents role changes, but **does not stop time**: session and grant
expiry must be checked after the last relevant wait even when resource contents
and optimistic versions did not change. Roll back domain effects if a later
adapter wait causes revalidation to fail. Never acquire another pool connection
while retaining mutation locks, and never start a new acquisition budget inside
an adapter.

Staff kick, logout and MFA reset revalidate after identity adapters return,
including security-revocation counter trigger waits. Sanction writers revalidate
after export-trigger effects. Downloads publication uses the canonical account →
channel resource → RBAC helper, retains the fence through history verification,
and repeats `downloads.manage` on the publication connection before commit.

Browser session rotation prepares both old and new accounts in UUID order before
the session row: the authentication trigger may lock the new account. Session
data writes prepare their current account before the session/referral trigger.
Identity transaction entries establish the budget before account/session rows,
including paths that reach advisory serialization only through signup triggers.

Journal login promotion reuses a subject cut only within one bounded 128-row
batch in the same transaction, retaining account, issuance, restore, feed,
ownership and stream locks. Every record still checks the database clock. This
avoids repeated authority queries exhausting the shared budget during bulk
watermark publication; it does not extend the acquisition deadline.

Privileged writers finish with live authority revalidation after effects and audit
append. Role writers revalidate the actor's session/MFA/account after their own
intentional permission changes, while retaining the RBAC fence. World pointer
preparation scopes its existing 250 ms limit to those pointers, then restores the
incoming limit before RBAC refreshes the original deadline; shorter incoming limits
also bound pointer preparation. No acquisition deadline or configured limit grows.
