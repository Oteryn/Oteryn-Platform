# Home hirelings activation status

GAP-65 covers the Platform journey for house hirelings. The
[G-14 proposal](../contracts/OTERYN_V2_HOME_HIRELING_COMMAND_CONTRACT_PROPOSAL.md)
and its Store amendment still require approval before purchase, expedition start,
or reward claim can be activated.

## Player and operator views

- With `PORTAL_HIRELINGS_ENABLED=true`, a verified, active account can open
  **Hirelings** from the account navigation. The Polish and English panel shows
  the availability of Store offers, hireling ownership and professions, current
  expeditions, the weekly allowance, published rewards, start and claim, the
  chest, and expedition results. Missing Game evidence is displayed as
  **Temporarily unavailable**. The panel flag defaults off.
- On the configured staff host, open **House helper status** from `/admin` with
  staff MFA, `admin.access`, and `admin.analytics.view`. The section reports
  the player panel flag, Game projection and activation status, the number of
  unresolved Platform intents, and the subset with a dead or missing outbox job.
  It displays an unavailable ledger state if storage cannot be read.
- The existing T92B command service validates typed start and claim bindings,
  checks trusted ownership, the exact reward-table revision and weekly budget,
  and atomically persists an immutable intent and an outbox job. Identical
  retries return the original operation. The shipped service has no trusted Game
  adapter or consumer. A retained intent stays unresolved until authoritative
  receipt reconciliation is added. A dead or absent outbox job does not convey
  a Game result. Keep those intents for reconciliation.

## Activation dependencies

On the Game side, approve the ownership and lifecycle rules, profession and
reward revisions, weekly caps, return-to-claim semantics, and chest capacity
rules. Implement fenced start and claim effects, Game-side idempotency, and
atomic projection and receipt publication. Supply exact producer revisions,
dedicated peer identities, verified baseline and online/ownership evidence, and
restore and replay qualification. A Game-side test must start one eligible
expedition, collect it once, and confirm the chest and owner history. It must also
exercise a foreign hireling, weekly overflow, an invalid profession, and a stale
reward-table revision.

Both owners must approve the Store `unit_kind` mapping for the hireling grant,
including product/version, customization, reservation, delivery receipt and
reversal policy. After those decisions, Platform can connect the approved
projection and receipt transports, enable CSRF-protected start/claim issuance and
Store checkout separately, reconcile unresolved intents, and show verified history
and return notifications. Receipt draining and restore fencing must be ready
before enabling issuance. Commerce and Game-effect switches default off.
