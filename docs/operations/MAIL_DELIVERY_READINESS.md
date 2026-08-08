# Mail delivery readiness

## Purpose

Password recovery and other identity notifications require a delivery-capable mail transport outside local/test environments. Repository defaults remain deliberately inert so a developer checkout cannot accidentally deliver mail.

This readiness check validates configuration shape only. It does **not** send a message, prove provider/network availability, prove DNS reputation, or prove mailbox receipt.

## Safe defaults

- `local` and `testing` may use `MAIL_MAILER=array`.
- `staging` and `production` must use a configured delivery-capable mailer.
- `array`, `log` and `null` transports are rejected for staging/production.
- The selected mailer must exist in `mail.mailers`.
- SMTP requires a non-empty host and a port in the range `1..65535`.
- `MAIL_FROM_ADDRESS` must be a valid address and must not use a reserved test domain.
- Credentials and provider secrets must be injected by the deployment secret-management path and must never be committed, printed by readiness output, or added to fixtures.

## Verification

Run against the environment actually being prepared:

```bash
php artisan mail:verify-delivery-readiness
```

An operator may validate a target environment explicitly without sending mail:

```bash
php artisan mail:verify-delivery-readiness --environment=staging
php artisan mail:verify-delivery-readiness --environment=production
```

The command is provider-neutral. For SMTP it validates only structural host/port readiness; authentication and connectivity remain deployment evidence. Future configured delivery transports are accepted when their configured `transport` is not an inert transport.

`production:verify-configuration` remains the broader production-security gate. Mail delivery readiness is a narrower reusable boundary that can also be exercised before staging password-recovery acceptance.

## SMTP deployment shape

The repository does not select a provider. A deployment may supply values equivalent to:

```dotenv
MAIL_MAILER=smtp
MAIL_SCHEME=tls
MAIL_HOST=<provider-or-relay-host>
MAIL_PORT=587
MAIL_USERNAME=<secret-managed-value>
MAIL_PASSWORD=<secret-managed-value>
MAIL_FROM_ADDRESS=<approved-sender-address>
MAIL_FROM_NAME="Oteryn Platform"
```

Do not copy real credentials into repository files, GitHub issues, workflow logs, screenshots, or task records.

## Relationship to password-recovery acceptance

Passing this check is necessary but not sufficient for staging password-recovery acceptance. The remaining end-to-end proof is intentionally operational:

1. configure a real provider and inject credentials outside Git;
2. run the readiness check successfully on the deployed staging candidate;
3. submit one controlled password-recovery request;
4. the owner observes the mailbox;
5. retain only sanitized evidence such as receipt time, sender, subject and reset-link hostname;
6. never retain the reset token or complete reset URL.

Draft PR #541 remains the owner of the public-domain task reconciliation and must not be marked complete merely because structural mail readiness passes.
