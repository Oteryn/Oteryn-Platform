# Registration abuse protection

Registration is disabled by default. Enabling it requires an explicit operator
update of `registration_abuse_policy.enabled` to true. A missing policy row or
database error denies registration. The flag also applies to the trusted Rust
registration function's password check.

There is no external captcha. The breach corpus is the Have I Been Pwned range
API with a 30-day PostgreSQL cache (owner decision 2026-11-21); it is the shared
breached-password check (`crate::breach`) used by registration, password change,
password reset and recovery-key recovery. The registration policy flag gates only
registration; the other flows always run the check. SHA-1 is only the lookup
format of the range API; account password storage remains Argon2id.

Lookup: the check reads the cached row for the five-character uppercase SHA-1
prefix where `complete = true` and `expires_at > now()`, and decides from it
without HTTP. Otherwise it sends one `GET {base}{PREFIX}` with `Add-Padding: true`
and `User-Agent: oteryn-portal` (3 s total timeout, 1 MiB body limit, no redirects
or proxies). `base` defaults to `https://api.pwnedpasswords.com/range/` and can be
set with `OTERYN_PWNED_PASSWORDS_URL` (plain `http` is accepted only for a loopback
host). Every response line must be `SUFFIX:COUNT` (35 uppercase hex digits, a
decimal count); count-0 padding lines are dropped and the remaining suffixes are
the complete range, upserted with `complete = true` and `expires_at = now() + 30
days`. Only the prefix leaves the process; the password, the full hash and the
suffix list are never logged. The check runs before any transaction is opened.

Rows imported by operators before 2026-11-21 kept only suffixes with a breach
count of at least 100 and are incomplete: the migration marks every existing row
`complete = false`, and such rows never decide a lookup. A row counts only when
`complete = true`. Operators do not need to import ranges any more; if they do,
they must import every suffix of a prefix with a positive count and set
`complete = true`, never partial results, padding or failures.

An API outage, timeout, non-200 status, oversized or malformed body with no fresh
complete cached row returns HTTP 503 for registration (and a temporary error for
the other flows); it never accepts the password. A failed cache write after a
successful fetch still decides from the fetched range. Breached passwords return
422 on registration and a refusal message elsewhere. Neither case creates
accounts, consent records or mail, changes a password, or uses a reset token or
recovery key.

The operator tool is `deploy/oracle/password-ranges.py` (`fetch DIR --min-count N`,
then `sql DIR --expires-days D | psql -v ON_ERROR_STOP=1`). It records N in
`DIR/min-count` and writes `complete = true` only for `--min-count 1`; a
thresholded import is written `complete = false` and never replaces a complete
row. The pre-production import of 2026-11-21 (`--min-count 100`) is therefore
ignored by the check, and the live range API decides. Production requires the
complete corpus (owner decision 2026-11-21): the live check with its cache
provides it; a `--min-count 1` import is optional and only pre-fills the cache.

Challenges require the existing browser session's CSRF verification. Work is 16
leading SHA-256 zero bits over the ASCII string `TOKEN:NONCE`; TOKEN is 64 uppercase
hex characters and NONCE contains at most 16 decimal digits. Each token contains
256 cryptographically random bits, binds to the form CSRF token and expires after
five minutes of database time. Atomic consumption prevents replay across workers.
Issuing again replaces the prior challenge for that form. Work is consumed before
password lookup and account creation, even when subsequent registration is refused.
Invalid/expired proofs return 422, disabled registration returns 503, and CSRF
failures remain 403. Successful and duplicate-address responses retain T03's
generic redirect.

The same-origin form script computes work locally using Web Crypto and yields
between batches. JavaScript and a secure browser context are required. Unsupported
clients receive a localized refusal, never an unprotected fallback. Refusal pages
provide a keyboard-accessible link to load a fresh form. Passwords/full hashes
never leave Platform Identity or appear in errors. Challenge responses use
`Cache-Control: no-store`; proofs are submitted in POST bodies.

Database tests read `PORTAL_TEST_DATABASE_URL`, create private schemas, enable only
their own policy, and provision explicitly synthetic complete ranges
(`complete = true`) or a local range mock on 127.0.0.1. Without the
variable they print a skip notice. No SQL query macros or offline metadata are added.
