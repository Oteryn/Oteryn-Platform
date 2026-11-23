**Draft — operator details are placeholders.**

Effective 22 November 2026.

## Portal operator

{{operator}}

## Cookies the portal sets

- **`__Host-oteryn_session` — strictly necessary.** Identifies the browser session, keeps you signed in and helps protect forms. It is a session cookie with no browser expiry date set. The session also expires on the server. The CSRF token is tied to the session and passed in forms and other requests without a separate cookie.
- **`__Host-oteryn_mfa_device` — strictly necessary for the trusted-device feature if you choose it.** Recognizes your remembered browser when signing in with multi-factor authentication. The cookie lasts 30 days.
- **`portal_language` — optional preference.** Remembers your language after you use the language switcher. The cookie lasts one year. You can also use URLs starting with `/pl/` or `/en/` directly.

These cookies are sent over HTTPS and are inaccessible to page scripts. The portal does not set advertising or tracking cookies.

## Managing cookies

You can delete or block cookies in your browser settings. Blocking the session cookie may prevent you from signing in and submitting forms. Deleting the other cookies clears the saved language or means you have to verify your sign-in again.

See the [Privacy Policy](/en/legal/privacy) for information about personal data.
