# Referral rollout qualification

GAP-57 Platform implementation follows the [proposed referral client and delivery contract](../contracts/OTERYN_V2_REFERRAL_CLIENT_CONTRACT.md).

## Player flow

Open **Account → Referrals**. An active, verified account can create a reusable link or request an e-mail invitation. The reusable link remains open while the campaign is active. Emailed codes expire 14 days after the latest queued attempt or successful delivery, whichever is later. A legacy e-mail invitation without attempts uses its creation time. A permitted resend renews the code while preserving the original attribution and reward windows.

Confirmation uses the portal session. An anonymous new invitee continues to registration, and an anonymous returning invitee continues to sign-in. The portal verifies eligibility when the account is bound. The referrals page shows the invitee's own attribution, the inviter's anonymized qualifying purchase status, and the signed-in account's reward records. Purchase qualification and entitlement creation use existing authoritative Platform transactions. Starter, outfit, addon and welcome records show Game delivery as unavailable. A starter entitlement can expire while held. Applied premium and accrued referral loyalty are read from Platform records.

## Operator checks

On the configured admin host, open **Admin → Referral campaign status** at `/admin/referrals?lang=en` or use `lang=pl`. The panel requires the existing staff MFA and `admin.access` checks. It reports the process flag, database campaign switch, connection activity fence, coverage, SMTP and premium configuration, mail backlog, paid qualifications, premium awards and held Game entitlements. It exposes aggregate counts and offers no controls to change state.

Leave `PORTAL_REFERRALS_ENABLED` and the database campaign switch off until rollout is authorized. Follow the [deployment coverage procedure](../../portal-rs/docs/FLAG_BOUNDARY.md#referral-deployment-and-activity-coverage) before relying on inactivity history. Both continuous deployment coverage and per-account history must reach 12 months. SMTP configuration is an indication of configuration only. Verify a real test mail and inspect the backlog separately.

On an authorized isolated deployment, verify that eligible acceptance creates one attribution, first paid store activity qualifies it once, and retries leave one reward per operation. Check that self-referral, ancestral cycles, an expired e-mail code and malformed input are refused. Verify premium through the existing ledger and verify that Game entitlements remain held until a delivery contract is approved.

## Game work required for production

- Qualify client navigation that opens the configured HTTPS portal in the system browser using the player's locale and portal session.
- Approve the entitlement contract and map starter, recruiter and welcome rewards to authoritative Game products and character scopes.
- Qualify the Game login and inactivity evidence before enabling bring-back eligibility in production.
- Qualify delivery and authenticated receipts across retries, duplicate operations, expiry and service interruption. Prove one Game effect per durable entitlement operation and reconcile receipts with the Platform reward state.
