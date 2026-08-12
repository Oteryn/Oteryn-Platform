# Continuation prompt — OTERYN-20260811 Tibia client analysis

Copy the prompt below verbatim into a fresh agent session.

---

Read and execute the durable state for task `OTERYN-20260811-tibia-client-analysis` and continue autonomously until the decoded-map runtime capture objective is proven or a genuine authority boundary is reached.

Repository: `blakinio/Oteryn-Platform`
Branch: `ops/oteryn-tibia-client-analysis-20260811`
PR: `#1006` (draft)

Do not reconstruct the investigation from chat. Treat live GitHub/repository/runtime state as authoritative and read applicable governance before mutation, especially root `AGENTS.md`, `docs/agents/PROMPTING_STANDARD.md`, `docs/agents/PROMPTING_HANDOVER.md`, `docs/agents/CONTEXT_HANDOFF.md`, `docs/agents/EXECUTION_RESOURCE_HYGIENE.md`, `docs/agents/tasks/active/OTERYN-20260811-tibia-client-analysis.md`, `docs/agents/reports/OTERYN-20260811-tibia-client-analysis-handover.md`, and `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md`.

Primary objective: prove one real decoded Tibia Worldmap runtime capture equivalent to:

`(x, y, z) -> ordered tile/field contents -> appearance/type IDs`

Already PROVEN and not to be rediscovered without contradictory evidence:

- official current client 15.32.df7b29; original bounded executable `/data/client-15.32.df7b29/bin/client`, size 51965216, SHA-256 `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`;
- installed official runtime also exists under `/data/home/.local/share/CipSoft GmbH/Tibia/packages/Tibia/bin/client`;
- official client was launched successfully under Xvfb `:99`, and login form was reached;
- `TWorldmapProtocolMessageHandler`: FullMap `0xcec8d0`, FieldData `0xcd3190`, Create `0xcecc70`, Change `0xcecf40`, Delete `0xcd4e20`;
- shared decoded map routine `0x19a8a80` preserves repeated field/content order and computes world coordinates;
- Coordinate protobuf schema is `x=1:uint32`, `y=2:uint32`, `z=3:uint32`;
- per-content path selects nested payload using helper `0x1ab4e50`, default-instance candidate `0x314b480`, then calls map-content builder `0xceca50`;
- lowest-risk dynamic interception remains after protobuf translation and at/before Worldmap handler / `0x19a8a80`, not encrypted TCP;
- screenshots already proved launcher, client welcome screen and empty login form; credentials are not present in the fresh runtime.

Authentication state and authorization:

- User created GitHub Actions repository secrets named `TIBIA_TEST_EMAIL` and `TIBIA_TEST_PASSWORD` for a test Tibia account.
- These secrets may be used only through GitHub Actions secret injection for this bounded test login. Never print, OCR, persist, commit, screenshot, echo or otherwise expose their values.
- User explicitly requested that login must not originate from the normal household/public IP. Verify tunneled egress before using the secrets.
- Baseline public egress observed before tunnel attempt was `loc=PL`, `warp=off`; do not perform login unless tunnel/VPN verification succeeds.

Current tunnel investigation — IMPORTANT, continue from here rather than repeating earlier attempts:

1. First WARP install run `31595064337` / job `94108491090` timed out while `apt` was configuring `cloudflare-warp`; login was correctly not attempted. Canonical staging preservation check passed.
2. A later probe proved the analysis container had `privileged=false`, no `CapAdd`, no devices, and `/dev/net/tun` absent. Cached package exists at `/var/cache/apt/archives/cloudflare-warp_2026.6.880.0_amd64.deb`. `warp-cli`/`warp-svc` were not installed into normal PATH.
3. Run `31596970952` failed before mutation because the host runner itself does not expose `/dev/net/tun`; canonical staging remained unchanged.
4. Current approach intentionally avoids touching host `/dev`: recreate ONLY the task-owned analysis container with `--cap-add NET_ADMIN`, then create `/dev/net/tun` inside that container with `mknod /dev/net/tun c 10 200`. Preserve `/data` bind and task labels. This is allowed only for the owned container, never canonical staging.
5. Current workflow commit before this handoff was `97427a9c3719703d8f2d6cbd5da2337370f5f158`; current run is `31597251649`, job `94115668719`. At the last observation, step `Recreate only task-owned analysis container with NET_ADMIN` was still `in_progress`. FIRST inspect its final state/log before doing anything else. Do not assume success.
6. The workflow snapshots the owned container image before recreation, preserves `/volume1/docker/oteryn/tibia-analysis:/data`, extracts the cached WARP `.deb` without postinst, starts `warp-svc`, and requires Cloudflare trace `warp=on` before any credential use.
7. If WARP verifies, the workflow relaunches the official client on Xvfb `:99`, injects `TIBIA_TEST_EMAIL`/`TIBIA_TEST_PASSWORD` only into the login step, attempts login through xdotool, unsets the variables, OCRs only an allow-listed result vocabulary, redacts the central dialog area, and uploads `tibia-login-result-safe` PNG. Never weaken this secret boundary.
8. If the current NET_ADMIN/TUN approach fails because the Synology kernel does not support TUN or Docker forbids `mknod`, do not repeatedly retry it. Choose another isolated egress method that genuinely changes the client's public source IP and can carry arbitrary Tibia TCP traffic. A mere HTTP proxy check is insufficient unless the actual Tibia client traffic is guaranteed to use it.

Owned runtime:

- runner label `oteryn-staging`; verified runner `oteryn-synology-staging`;
- container `oteryn-tibia-client-analysis`;
- persistent bind `/volume1/docker/oteryn/tibia-analysis` -> `/data`;
- required labels `com.blakinio.owner=oteryn`, `com.blakinio.purpose=tibia-client-analysis`;
- canonical `oteryn-staging` Compose services are strictly out of scope for stop/recreate/reconfigure/cleanup.

Safety:

- Never modify, restart, stop, recreate, clean or otherwise disturb canonical `oteryn-staging` services, deployment infrastructure, databases, networks, volumes or unrelated containers.
- No blanket Docker cleanup.
- Before every owned-container mutation, verify both ownership labels.
- Do not commit Tibia binaries/assets, credentials, tokens, session material, account information or protected character data.
- Screenshots after authentication must be privacy-safe/redacted and must not reveal account/character/private information unless strictly required and explicitly safe.

After successful tunneled authentication:

1. Confirm an authenticated character/world selection state without exposing private account data.
2. Enter the game world using the test account/character through the verified tunnel.
3. Instrument the already-proven decoded Worldmap boundary, preferably `0x19a8a80` or an immediately adjacent deterministic point after protobuf translation and before `TWorldMapStorage` mutation.
4. Capture one bounded real FullMap/FieldData (or equivalent relevant map update) and normalize it to deterministic evidence: `(x,y,z) -> ordered contents -> appearance/type IDs`.
5. Tie `0x314b480` and the relevant generated-object offsets to concrete protobuf semantics if the live capture permits.
6. Persist only safe bounded evidence in `docs/agents/reports/OTERYN-20260812-worldmap-dispatch-evidence.md` and update the active task checkpoint with PROVEN / DERIVED / UNKNOWN / CONFLICT.
7. Verify canonical staging unchanged after runtime work.
8. Do not declare completion without the real decoded-message capture proof.
9. Before terminal closeout, remove/reduce temporary trace/one-shot workflow scaffolding that should not merge, following repository governance; intentionally retained proprietary runtime data remains outside Git and must have its lifecycle recorded.

Do not stop merely because a workflow, commit or checkpoint was created. Continue autonomously until the capture is proven or a genuine authority/environment boundary is reached. If blocked, leave exactly one concrete `next_action` in durable repository state.