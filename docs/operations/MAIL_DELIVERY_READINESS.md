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
- An explicitly configured SMTP scheme must be `smtp` or `smtps`; omit `MAIL_SCHEME` to use the framework default. STARTTLS on port 587 uses `smtp`, not a `tls` scheme.
- `MAIL_FROM_ADDRESS` must be a valid address and must not use a reserved test domain.
- Credentials and provider secrets must be injected by the deployment secret-management path and must never be committed, printed by readiness output, or added to fixtures.

## Verification

Run on the environment actually being prepared:

```bash
php artisan mail:verify-delivery-readiness
```

The command always validates the application's configured `APP_ENV`; it has no environment override. This prevents a staging or production check from being accidentally downgraded to the inert local/testing policy.

The command is provider-neutral. For SMTP it validates structural scheme/host/port readiness; authentication and connectivity remain deployment evidence. Future configured delivery transports are accepted when their configured `transport` is not an inert transport.

`production:verify-configuration` remains the broader production-security gate and reuses the same mail delivery verifier. Mail delivery readiness is a narrower boundary that can also be exercised before staging password-recovery acceptance.

## SMTP deployment shape

The repository does not select a provider. A STARTTLS deployment on the common submission port may supply values equivalent to:

```dotenv
MAIL_MAILER=smtp
MAIL_SCHEME=smtp
MAIL_HOST=<provider-or-relay-host>
MAIL_PORT=587
MAIL_USERNAME=<secret-managed-value>
MAIL_PASSWORD=<secret-managed-value>
MAIL_FROM_ADDRESS=<approved-sender-address>
MAIL_FROM_NAME="Oteryn Platform"
```

`MAIL_SCHEME` may be omitted to use the framework default. Use `smtps` only when the selected provider explicitly requires implicit TLS for that endpoint. Do not configure `MAIL_SCHEME=tls`.

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
