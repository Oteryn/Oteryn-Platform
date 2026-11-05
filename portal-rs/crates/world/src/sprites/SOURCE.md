# Sprite format provenance and worker use

SOURCE: https://github.com/Oteryn/Oteryn-Game
Commit: 857f15f376907c592bb28d854fccff18a15cdf87

Read-only format references:

- `crates/client-assets/src/appearance.rs`: protobuf field numbers, first group/phase,
  index order `[phase][z][y][x][layer]`, empty sprite zero, creature base layer.
- `crates/client-assets/src/catalog.rs`: sprite range and layout types 0–3.
- `crates/client-assets/src/sheets.rs`: 32-byte wrapper, LZMA size correction,
  384×384 BMP, BGRA masks, magenta transparency and row-major slot indexing.
- `tools/game-atlas-appearances/export.py`: independent field/phase validation,
  south direction index 2 for four-direction outfits.
- `content/assets/files/catalog-content.json`: populated source catalogue.
- `content/assets/files/appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat`
  and `content/assets/files/sprites-*.bmp.lzma`: actual asset locations.
- `content/assets/catalog/index.json` is a directory placeholder at this commit.

No game artwork or asset files are copied. All fixtures in `tests.rs` are synthesized
from these format descriptions, including tiny protobuf messages and uniform colour
sprites in compressed sheets. The full sheet dimensions are required by the format,
but compressed fixtures are generated in memory and contain only a few occupied slots.

Call `oteryn_world::sprites::generate` from a blocking worker executor with
`SpriteFeature { enabled: true }`, a full pinned commit SHA, the release's
`content/assets/files` directory, an output root and explicit appearance requests.
Default `SpriteFeature` refuses work before filesystem access. No route is mounted.
The caller must authenticate the pinned checkout; a version string is provenance,
not proof that an arbitrary directory came from Git. Inputs must be immutable while
running. Requests map game entity keys to appearance IDs at the world importer boundary.

Canonical items use one item, empty fluid, no hook or location. Creatures use the
first standing frame group, phase zero, south, base layer only, no addons or mount.
Explicit pattern coordinates allow callers to choose item variants and creature
orientations. Tinting and addons are outside this thumbnail mechanic. Different
size layers are aligned at the bottom right, then alpha composed. Transparent
appearances produce a 32×32 transparent image. Displacement affects placement in a
world tile, not the intrinsic thumbnail bounds.

Output is lossless WebP with deterministic names and a JSON manifest carrying input
SHA-256 digests, world commit, selected patterns, dimensions and output digests.
The directory is `<root>/<commit>/<manifest-sha256>/`. A temporary sibling directory
is renamed only after every image and manifest has been written. Repeat calls verify
existing bytes and return `reused = true` without rewriting them. Corrupt existing
artifacts are refused; a changed input produces a different manifest hash. Activation
and serving are responsibilities of the importer and static-file publisher. Output
roots are trusted worker-owned directories. No database, SQL macros or credentials
are required by this pipeline.
