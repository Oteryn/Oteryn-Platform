# GAP-53 operational email

Use a one-time mailing for planned downtime or event notices. Players explicitly
opt in under **Account → Notification preferences**. Operational email has its
own preference and is off by default. News, support and mandatory security
notifications keep their existing preferences. Players can turn operational
email off in their account or unsubscribe from the link in either email format.
Mail clients can also use the existing signed one-click mechanism.

## Operator workflow

1. Assign `admin.access` and `mail.operational.compose` to designated authors.
   Assign `admin.access` and `mail.operational.review` to designated reviewers.
   These new permissions are absent from built-in grants. Both roles use the
   staff panel's MFA and live session checks.
2. Open **Staff → Operational email**. Choose all opted-in players, Polish
   account language, or English account language, and enter both versions of
   the subject and body. The saved preview lists the audience counts by
   language. A segment with zero or more than 1,000 eligible recipients is
   refused in full.
3. A different staff member opens the saved preview within 24 hours and
   approves it or rejects it. Approval queues the mailing once. The preview
   fixes the audience and content. Later opt-ins are excluded, and eligibility
   is checked at approval and again before dispatch. If nobody remains
   eligible at approval, create a new preview or reject the old one.
4. Reopen the mailing for queued, mail-server accepted, skipped and unconfirmed
   counts. Staff audit records include preview, approval or rejection, actors,
   campaign ID and counts. The campaign screen and audit show aggregate counts
   without recipient addresses or signed links. Message text is visible to
   the author and designated reviewers on the campaign screen.

The portal needs its existing `OTERYN_NOTIFY_UNSUBSCRIBE_KEY` and authenticated
SMTP configuration. Set `OTERYN_NOTIFY_UNSUBSCRIBE_URL` to the public portal
origin with `/en/unsubscribe` or `/pl/unsubscribe`. The mailer generates the
localized link. Run the existing worker with matching SMTP configuration.
The campaign screen indicates missing send configuration and still allows
preview and rejection. There is one SMTP attempt per approved recipient.
An interrupted or failed attempt can be unconfirmed. The mail server accepting a
message does not establish inbox delivery.

## Release decision

Before granting production roles, the owner should confirm permitted notice
types, the operational sending volume, the author and reviewer roster, and
retention for campaign content and dispatch history. For a launch, verify
delivery and opt-out with controlled accounts and confirm that the sender domain
and SMTP service apply a valid DKIM signature covering both unsubscribe headers.

The audience and messages are Platform-owned. Operators supply approved facts
about downtime and events. The portal uses no Game data or commands to select
recipients, compose a message or schedule a send. Game-side implementation is
unnecessary for this workflow.
