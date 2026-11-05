# Registration abuse protection

Registration is disabled by default. Enabling it requires an explicit operator
update of `registration_abuse_policy.enabled` to true. A missing policy row or
database error denies registration. The flag also applies to the trusted Rust
registration function's password check.

There are no external calls, external captcha, or external range API. No external
API flag is needed because no such adapter is provided. SHA-1 is only the lookup
format of the local breach corpus; account password storage remains Argon2id.

Before enabling registration, provision `registration_password_ranges` from a
trusted, complete breach corpus. Import each complete five-character uppercase
SHA-1 prefix and all corresponding 35-character uppercase suffixes with positive
breach counts, removing the count field. Publish each complete range atomically
and set `expires_at` to the corpus freshness deadline. An empty array is permitted
only for a prefix verified to have no breached entries. Never convert partial
results, padding entries, import failures, or source unavailability into an empty
range. Corpus distribution and import are operator responsibilities; no synthetic
corpus is shipped as production coverage. Restrict policy/corpus writes to trusted
operators. Missing, expired or malformed ranges return HTTP 503. Breached passwords
return 422. Neither case creates accounts, consent records or mail.

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
their own policy, and provision explicitly synthetic complete ranges. Without the
variable they print a skip notice. No SQL query macros or offline metadata are added.
