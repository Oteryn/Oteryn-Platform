# Native login gateway

T52 implements the protocol 2 gateway from the Go gateway's
`internal/{nativelogin,httpapi,platform}` (removed on owner decision 2026-11-11;
last present at commit `2d45732c`, path `legacy/services/game-gateway`). Run the same portal
binary with `oteryn-portal gateway`. This dispatch happens before portal
configuration is loaded: no database URL, pool, migrations or signing keys are used.

Configuration:

| Variable | Default / requirement |
| --- | --- |
| `OTERYN_PLATFORM_BASE_URL` | Required; HTTPS, or HTTP on loopback only; optional path prefix |
| `OTERYN_PLATFORM_SERVICE_TOKEN_FILE` | Required; externally supplied bearer token file, optional final newline |
| `GATEWAY_NATIVE_LOGIN_ENABLED` | `false`; accepts only `true` or `false` |
| `GATEWAY_LISTEN_ADDR` | `0.0.0.0:8080` |
| `GATEWAY_NATIVE_ADMISSION_TIMEOUT` | `5s`, positive and at most `8s` |
| `GATEWAY_REQUEST_TIMEOUT` | `5s`, positive and at most `30s` (readiness) |
| `GATEWAY_VERSION` | `dev` |

The upstream admission path is `/internal/v1/game-auth/native-admissions`.
Admission uses a service bearer token, never mTLS. Requests are sent once, without
redirects or retries. A failure before sending is unavailable; an ambiguous sent
request or invalid success requires reconciliation. Error bodies are validated
against all eleven public status/code/class rows. Readiness probes upstream
`/health`; process health stays independent.

The fixed windows match Go: 30 requests per connection-source IP per minute and
six per SHA-256 of `(attempt_ref, NUL, ticket)` over two minutes. Each table holds
at most 100,000 keys and refuses new keys when full. Forwarding headers never
supply the source IP. Account authorization and account-level controls remain
upstream; the gateway has no authenticated account identity.

## Verification

The checked-in `tests/fixtures/go_vectors.jsonl` was captured from the Go tests at
Platform `d140863b`, without modifying reference files. It contains 185 wire/error
vectors, 94 limiter operations and 46 handler responses. Rust replays all of them,
checking response bytes including newlines, status, retry headers and call counts.
The HTTP integration tests also send all 43 captured issuer responses through the
real Rust upstream client. The Go source is no longer in the tree; the vectors
stay the reference. To regenerate them with Go on PATH, extract the source from
its last commit:

```sh
git archive 2d45732c legacy/services/game-gateway | tar -x -C "$dir"
python3 portal-rs/crates/gateway/tests/capture_go_vectors.py "$dir/legacy/services/game-gateway"
```

The Go E2E journey is retained in `tests/go_e2e`, with only the gateway startup
replaced by a subprocess of the Rust binary. Its fake upstream checks the service
credential and canonical request, commits a grant, returns the same token on retry
with shorter validity, then refuses a conflicting ticket. It also checks logs for
synthetic secrets and runs with deliberately invalid database configuration:

```sh
cd portal-rs
cargo build --locked -p oteryn-portal
cd crates/gateway/tests/go_e2e
GATEWAY_RUST_BINARY=/absolute/path/to/oteryn-portal go test -v -count=1 ./...
```

`tests/game_client_native_login_fixtures.rs` is intentionally empty pending the
coordinator's client fixtures. There are no database integration tests in this
stateless role. No SQL or offline query metadata is added. Deployment, production
activation and future protocol changes remain outside T52.
