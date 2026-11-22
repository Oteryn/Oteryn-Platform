# Browser form resubmission

Start the disposable PostgreSQL, HTTPS browser launcher, worker and SMTP sink described in the [browser qualification guide](../README.md). From `portal-rs/e2e`, use the launcher's private environment file.

```sh
node run.mjs /absolute/scratch/env.json test --config double-submit/playwright.config.mjs
```

The suite inherits the Polish and English projects at 1440 × 900 and 390 × 900. It uses one worker and disables retries, traces and screenshots. Chromium's back/forward cache is enabled so the ticket history case can resubmit the original rendered form.

| Form | Evidence |
| --- | --- |
| Player ticket creation | Concurrent submissions and browser history replay redirect to the same ticket. Reload issues a GET. PostgreSQL retains one ticket and one initial message. Revoked, rotated and unrelated session tokens receive a localized 403 and create zero tickets. |
| Staff ticket reply | A parallel replay and a different stale reply each receive a localized 409. The accepted reply produces one message, one audit entry, one notification, one completed outbox job and one SMTP delivery. |
| CMS page save | The English source and the Polish translation use the real editorial form. Concurrent submissions and a stale tab leave one accepted revision and audit entry. The index reload and public page show the saved content. |
| Staff role creation and permission replacement | Duplicate creation produces one role and one creation audit. An identical permission replacement produces one audit. Two tabs submitting different explicit replacements produce ordered audit records with both permission sets. |
| Character profile save | An LCFA portfolio enters through the launcher's real mTLS listener. The two submissions and competing tab share one privacy revision. One save advances it and writes one event. The reloaded profile shows the saved comment. |

The requested inventory path `portal-rs/e2e/forms.json` is absent from the branch base for [#2002](https://github.com/Oteryn/Oteryn-Platform/issues/2002). The route inventory in `../inventory.json` lacks rendered form identities and fields. This suite covers the forms above, and reconciliation against the full form inventory is pending.
