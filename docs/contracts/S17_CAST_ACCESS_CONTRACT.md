# S17 cast account admission — phase 1

Companion Game base: `81504ec792c186a3adcdc95aef8610d0862334cf`.
Accepted design: Game `docs/architecture/design/S17-CAST-DESIGN.md`, PR #3000,
revision `956a1cdff6188381b25419251654f63a8b19536f`, section 6 owner decisions.
Platform source base: `c75f1cea567f0b5c6b31951f5da58cb91f0a5d49`.

0968 owns account controls, short-lived cast policy and hashed single-use
observer reservations. There is no Character FK or character selection. Game
owns live cast lifecycle, native visibility, delay, moderation and immutable
0967 audit. Platform owns password KDF and the global admission budget.
Restart does not restore a cast-on session. Optional password defaults absent;
password-required starts remain closed until their owner installs a verifier.

## Boundaries

All new configuration defaults off. Gateway `GATEWAY_CAST_WATCH_ENABLED` must
explicitly enable POST `/v1/casts/watch`. It accepts bounded strict JSON:
`accountTicket`, UUIDv7 `attemptId`, opaque hex `castId`, optional `password`,
and selected `chat`. Unknown fields, including character selectors, fail.
Gateway derives source IP from the TCP peer and forwards through its existing
service-authenticated POST `/internal/v1/casts/watch`. No client-provided IP
header/body field is trusted. The existing account-only native admission ticket
is redeemed once before password processing; current account security generation
and usability are checked. Incorrect/replayed/unavailable admission returns the
same noncacheable 409, with bounded attempt limits and a 250ms response floor.

Game uses POST `/internal/v1/casts/node`. `GuardCastNode` requires verified TLS
1.3 server variables and an exact configured certificate subject-to-node mapping;
request headers cannot impersonate it. Operations are announce, stop, consume,
renew and release. They are bounded and independently disabled by `casts.enabled`.
Game pins the Platform CA, uses its separate TLS client identity, prohibits
redirects/proxies and caps response size. Deployment must populate `casts.nodes`
and `casts.routes` with verified node IDs and public routes before activation.

`announce` binds cast, owner account, epoch, access revision, delay, capacity and
password-required state. It grants a five-second lease and cannot remove an
existing password. Password mutation is POST `/casts/{cast}/password`, protected
by existing web authentication, current identity/session, CSRF and throttling.
The owner supplies expected access revision and nullable password. Argon2id
uses 19456KiB, two iterations and one thread. Rotation increments access revision
and deletes existing reservations. Game receives no reusable password or hash.

## Reservations and moderation

Admission serializes account-root then cast-policy rows. At most two live viewer
sessions exist per account across casts, and fifty per cast. Optional passwords
are verified outside the transaction and the exact policy is rechecked inside.
A cryptographically random `ocv1.` credential is stored only as SHA256 and may be
consumed once by its bound authenticated node. Consume/renew verifies the current
account generation, cast policy, epoch, access revision and selected-chat flag.
Successful responses have purpose `oteryn-cast-observer-v1`, bound identities,
reservation and five-second expiry. Game uses conservative monotonic deadlines
sampled before HTTP IO, renews every two seconds and cannot extend a reservation
with viewer keepalive. Wrong-purpose native/player credentials are refused.

Game mute applies to the account in that cast and suppresses new viewer chat;
kick removes one viewer session and calls node-authenticated release. Leave,
expiry, stop and password rotation also free/invalidate slots. Kick is not a ban.
Viewer chat and caster delivery remain Game-owned and never traverse this API.
No Discord, Twitch, OBS or client UI is included.

## Focused verification

The eight `legacy/tests/Feature/Casts` PostgreSQL cases execute individually against a
dedicated local database. They cover optional/wrong password, account-only/global
limits, fifty viewers, single-use/node binding/kick, expiry, two real concurrent
cross-cast contenders, private starts and authenticated HTTP ticket redemption.
The concurrency test observes both independent connections blocked on the account
row before release. No full PostgreSQL suite is required or run for this slice.
Focused Gateway tests cover default-off routing, strict account-only requests,
purpose separation and existing configuration validation. Pint and scoped
PHPStan level 10 apply to the new owner. Game supplies separate native/protocol,
consent barrier, audit and real PostgreSQL tests.
