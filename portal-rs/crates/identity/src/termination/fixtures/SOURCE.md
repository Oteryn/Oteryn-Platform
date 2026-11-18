Game producer: Oteryn/Oteryn-Game at `553802fa149ff774b5313f3fcaa45a160e3f17ef`.

- `closure.json` and `lifecycle.json` are byte-exact copies from
  `docs/contracts/fixtures/account-deletion/`, including their final LF.
  The embedded wire strings exclude that framing. The Game fixtures contain no
  signing material.
- `pending.json` and `read.json` are the request bytes from
  `docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §§4.1–4.2,
  excluding Markdown framing. The serializer is
  `crates/gameplay/native-admission-source/src/character_commands.rs`
  (`encode_list`, `encode_read`). The HTTP test substitutes the issued closure
  command and operation ID into the read example.
- `watermark.json` is the output of the `Watermark` serializer in
  `crates/gameplay/durability/src/durability/character_command/account_deletion/publish.rs`,
  using the primary authority, epoch 3, observation time 103 and cut 100.
  Game constructs this body at runtime. The emitted operation is
  `PublishProjectionWatermarkV1`, version 2, family `account_closure`.
- `docs/contracts/fixtures/account-deletion/CONTRACT-NOTES.md` lines 84–97
  specify the provisioned Character mTLS identity and the v1 pull/read wrappers.
  The nested closure intent and all closure publications remain v2.
