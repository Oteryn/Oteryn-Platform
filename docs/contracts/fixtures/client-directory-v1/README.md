# Client directory v1

Byte-exact vectors for `GET /v1/client/directory`, the native client's
non-secret world and channel directory. Contract:
`docs/contracts/OTERYN_V2_CLIENT_DIRECTORY_CONTRACT.md`. A change to any file
here is a contract change.

- Consumer read at `Oteryn/Oteryn-Game@e2152d050038cecdb3728c222af2ac184e1cac5a`,
  `crates/platform-client/src/lib.rs` (`parse_directory`) and
  `crates/platform-contracts/src/lib.rs` (read-only, owner grant 2026-11-05).
- Platform has no handler yet (plan §5.2, T36b); these vectors are the shape
  that handler must produce. Base: `Oteryn/Oteryn-Platform@5510ab0a840a009eecdab01fd988823a258d1f30`.
- Manifest digests: `portal-rs/crates/gameauth/src/oauth/oauth_fixture_tests.rs`,
  which also checks each vector against its declared `client_outcome`.

## Cases

`manifest.json` lists the files with byte length and SHA-256 and one entry per
case. `client_outcome` is what a conforming v1 consumer does with the body:
`accept`, or `reject:<error>` with the Game `PlatformClientError` variant.
Files have no trailing newline.

| case | request | body | client outcome |
|---|---|---|---|
| directory-anonymous | no `Authorization` | `directory-anonymous.json` (`characters: []`) | accept |
| directory-owner | `Authorization: Bearer <synthetic token>` | `directory-owner.json` | accept |
| directory-rejected-route-key | any | `directory-rejected-route-key.json` (channel has `endpoint`) | reject: `ForbiddenGameplayField` |
| directory-unknown-pvp-type | any | `directory-unknown-pvp-type.json` (`pvp_type: HARDCORE_PVP`) | reject: `InvalidPayload` |

The last two are bodies a conforming Platform never sends; they pin the
consumer's fail-closed behaviour. The token is the synthetic value from
`native-characters-v1`, not a credential.
