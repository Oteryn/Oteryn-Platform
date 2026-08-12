# OTERYN-20260812 Worldmap dispatch evidence

## Scope

Durable bounded reverse-engineering checkpoint for `OTERYN-20260811-tibia-client-analysis` on draft PR #1006. No proprietary Tibia binary or extracted asset is stored here; only addresses, hashes, control-flow observations, and GitHub Actions evidence.

## Verified client identity

- Client: `/data/client-15.32.df7b29/bin/client`
- Size: `51965216`
- SHA-256: `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`
- Successful dispatch run: `31580692975`
- Successful dispatch job: `94062813472`
- Runner: `oteryn-synology-staging`
- Owned container: `oteryn-tibia-client-analysis`
- Canonical `oteryn-staging` container inventory diff before/after: unchanged in the successful job.

## PROVEN — exact Worldmap Qt metaobject

`TWorldmapProtocolMessageHandler` resolves to:

```text
QMetaObject VA       0x3087800
Qt stringdata VA     0x1cd8a54
Qt metadata VA       0x1cd8820
static_metacall      0xdf2a60
jump table VA        0x1d8bd10
metadata header      13 0 0 0 14 14 0 0 0 0 0 0 0 1
class                tibia::worldmap::TWorldmapProtocolMessageHandler
method_count         14
signal_count         1
```

Method indices decoded from the Qt6 metadata table:

```text
0  publishGameAction
1  handleFullMapMessage
2  handleLeftColumnMessage
3  handleRightColumnMessage
4  handleTopRowMessage
5  handleBottomRowMessage
6  handleTopFloorMessage
7  handleBottomFloorMessage
8  handleFieldDataMessage
9  handleCreateOnMapMessage
10 handleChangeOnMapMessage
11 handleDeleteOnMapMessage
12 handleAmbientLightMessage
13 handleTibiaTimeMessage
```

## PROVEN — exact handler bodies

```text
handleFullMapMessage       -> 0xcec8d0
handleFieldDataMessage     -> 0xcd3190
handleCreateOnMapMessage   -> 0xcecc70
handleChangeOnMapMessage   -> 0xcecf40
handleDeleteOnMapMessage   -> 0xcd4e20
```

The Qt jump-table case for method index 1 is `0xdf2b88` and tail-jumps to `0xcec8d0`. The case for method index 8 is `0xdf2cd8` and tail-jumps to `0xcd3190`.

## PROVEN — common map-data routine

`handleFieldDataMessage` directly calls `0x19a8a80` at `0xcd3224` and returns immediately afterwards. Directional Worldmap cases (`LeftColumn`, `RightColumn`, `TopRow`, `BottomRow`) also converge on `0x19a8a80` after constructing region-like arguments.

Therefore `0x19a8a80` is directly verified as a shared routine used by FieldData and multiple directional map-update paths.

## INFERENCE — role of `0x19a8a80`

High confidence: `0x19a8a80` is the central routine that applies decoded map-field data over a supplied region/range. Its full content-order and appearance semantics remain to be recovered/proven.

## PROVEN — Coordinate schema

An embedded standard protobuf `FileDescriptorProto` for `shared.proto` proves:

```text
Coordinate.x = field 1, uint32
Coordinate.y = field 2, uint32
Coordinate.z = field 3, uint32
```

The beginning of `handleFullMapMessage` reads a nested object and copies three adjacent 32-bit values from generated-object offsets `+0x18`, `+0x1c`, `+0x20`. Exact identity of those generated offsets with Coordinate fields remains an inference until tied directly to the generated runtime type.

## INFERENCE — generated protobuf defaults

Current static evidence gives these high-confidence candidates, but they are not yet promoted to PROVEN type identities:

```text
0x313a820 -> Coordinate default instance candidate
0x313a860 -> MapFieldData default instance candidate
0x314b480 -> AppearanceInstance default instance candidate
```

`0x313a820` recurs in FullMap/FieldData/Create/Change/Delete at coordinate dereference sites. `0x313a860` occurs at map-field payload selection before accesses around `+0x28`. `0x314b480` is used by Create/Change when selecting an appearance-like payload.

## Rejected hypotheses

- `0xde9ca0` is not a Worldmap static metacall. Exact Qt stringdata identifies it as `tibia::sessiondump::TSessiondumpPlayer`. Do not reuse the earlier ordinal-based association.
- Full `.text` traversal with Python/Capstone is not an appropriate bounded method on this runner; native streaming `objdump` succeeded and should remain preferred.
- Do not start from encrypted TCP unless the decoded protobuf path is disproved by new evidence.

## Runtime capture / authentication boundary

The final acceptance proof is still missing:

```text
(x, y, z) -> ordered field/tile contents -> appearance/type IDs
```

Static reverse can continue without account credentials. However, a real decoded FullMap/FieldData message is normally produced only after the client has an active game-world session (initial world entry or a subsequent map update). Therefore an authenticated game session is likely required for the final bounded runtime capture.

This is currently an **INFERENCE**, not proof that credentials must be supplied manually. Before requesting any credential from the user, the continuation agent must inspect the owned runtime for an already usable safe session/test-account mechanism without exposing secrets. Never print, persist, commit, or copy passwords, tokens, session keys, account data, or protected character data into GitHub Actions logs or repository files.

If no safe authenticated session is available, stop at that authority boundary and ask the user only to perform the minimum interactive login needed to establish the session. Do not ask the user to paste credentials into chat, workflow inputs, repository secrets created ad hoc, scripts, or logs.

## Pending trace state

Several bounded trace runs were triggered while the self-hosted runner queue was congested. At the last reliable checkpoint, a trace run/job had acquired the runner but its final result was not retained as verified evidence in the durable record. Do not assume success or failure from chat history. Inspect live GitHub workflow state and use only completed run/job evidence.

## Exact continuation objective

1. Verify live branch HEAD, PR #1006 state, CI, path ownership, runner/runtime state and ownership labels.
2. Read this report plus the active task/handover; do not reconstruct from chat.
3. Inspect completed trace runs and persist any newly verified result.
4. Fully trace `0x19a8a80` enough to prove field iteration/content ordering and identify appearance/type extraction.
5. Tie the default-instance candidates above to concrete generated protobuf types where possible.
6. Select the lowest-risk interception point after `TProtobufServerMessageTranslator` and before mutation of `TWorldMapStorage`.
7. Determine whether the owned runtime already has a safe authenticated-session mechanism. Do not expose credentials.
8. If a safe session exists, perform one bounded decoded-message capture and normalize it to deterministic `(x,y,z) -> ordered contents -> appearance/type IDs` records.
9. If interactive login is genuinely required, checkpoint all static findings and stop only at that explicit user-authority boundary with precise login instructions.
10. After proof, clean temporary analysis workflows before any terminal merge decision; preserve proprietary client material outside Git.

<!-- BEGIN GENERATED TRACE EVIDENCE -->
<!-- BEGIN GENERATED TRACE 315-next -->
## Generated bounded common-map-data trace

- Client size: `51965216`
- Client SHA-256: `e6c244bd39fe2e0632f6f000efd3147164696efa8e901718668e0442325ff7fe`
- Common routine: `0x19a8a80`
- Decoded instructions in bounded window: `774`
- Direct call targets: `0x1983960`, `0x1b13c80`, `0x1ab4e50`, `0x1aab450`, `0xceca50`, `0x177c150`, `0x4de900`, `0x6b3a90`, `0x4df670`, `0x6b3ae0`, `0x1b22930`, `0x4daf40`, `0x4df5a0`, `0x4da460`
- Observed positive register-relative offsets: `0x0`, `0x1`, `0x8`, `0xc`, `0x10`, `0x18`, `0x1c`, `0x20`, `0x28`, `0x30`, `0x38`, `0x3c`, `0x40`, `0x48`, `0x57`, `0x58`, `0x60`, `0x68`, `0x70`, `0x78`, `0x80`, `0x88`, `0x8c`, `0x90`, `0x98`, `0xa0`, `0xa8`, `0xb0`, `0xc0`, `0xc8`, `0xd0`, `0xd8`, `0xe0`, `0xe8`, `0xf0`, `0xf8`, `0x100`, `0x110`, `0x118`, `0x120`, `0x130`, `0x138`, `0x140`, `0x150`, `0x158`, `0x160`, `0x168`, `0x170`, `0x178`

### Internal control-flow edges
```text
0x19a8ab8 jne -> 0x19a8abe
0x19a8b0e je -> 0x19a8b2b
0x19a8b31 je -> 0x19a8b51
0x19a8b39 jne -> 0x19a965f
0x19a8b5a je -> 0x19a95ef
0x19a8bfc je -> 0x19a8c19
0x19a8c1f je -> 0x19a8c3f
0x19a8c27 jne -> 0x19a9640
0x19a8cc4 je -> 0x19a9340
0x19a8d2d je -> 0x19a9358
0x19a8d45 jne -> 0x19a8e12
0x19a8d4b jmp -> 0x19a94ca
0x19a8d60 je -> 0x19a8f40
0x19a8d7c je -> 0x19a96cb
0x19a8d8c je -> 0x19a8f18
0x19a8db7 je -> 0x19a9080
0x19a8dc7 je -> 0x19a9060
0x19a8dd9 je -> 0x19a9072
0x19a8def jne -> 0x19a8f00
0x19a8e0c jae -> 0x19a90a0
0x19a8e68 je -> 0x19a8e88
0x19a8e70 jne -> 0x19a9548
0x19a8ee3 jne -> 0x19a8d50
0x19a8eec je -> 0x19a8ddf
0x19a8ef2 jmp -> 0x19a8da2
0x19a8f0d jmp -> 0x19a8df5
0x19a8f2e je -> 0x19a8ddf
0x19a8f34 jmp -> 0x19a8da2
0x19a8f64 je -> 0x19a96f4
0x19a8f79 jb -> 0x19a96e0
0x19a8f82 jne -> 0x19a9695
0x19a8faf je -> 0x19a8fc5
0x19a8fbb je -> 0x19a9570
0x19a8fc8 je -> 0x19a9583
0x19a8ffe jb -> 0x19a8fe8
0x19a9054 je -> 0x19a8ddf
0x19a905a jmp -> 0x19a8da2
0x19a906c jne -> 0x19a8ddf
0x19a907a jmp -> 0x19a8ddf
0x19a909a jmp -> 0x19a8ddf
0x19a90b3 je -> 0x19a94d7
0x19a90f1 jb -> 0x19a9686
0x19a912e je -> 0x19a966c
0x19a913a jmp -> 0x19a9151
0x19a914f je -> 0x19a917f
0x19a915f je -> 0x19a9144
0x19a916b jne -> 0x19a9140
0x19a917d jne -> 0x19a9151
0x19a91a5 jne -> 0x19a91c7
0x19a91a7 jmp -> 0x19a9228
0x19a91bc je -> 0x19a9205
0x19a91c5 je -> 0x19a9220
0x19a91ce je -> 0x19a91be
0x19a91e5 je -> 0x19a9520
0x19a91f5 jne -> 0x19a91b0
0x19a9203 jne -> 0x19a91be
0x19a9214 jne -> 0x19a91c7
0x19a922b je -> 0x19a9240
0x19a924f je -> 0x19a927e
0x19a9265 jne -> 0x19a9634
0x19a9278 je -> 0x19a95c0
0x19a9298 jne -> 0x19a92bb
0x19a929a jmp -> 0x19a9318
0x19a92b0 je -> 0x19a92fb
0x19a92b9 je -> 0x19a9310
0x19a92c2 je -> 0x19a92b2
0x19a92db je -> 0x19a94a8
0x19a92eb jne -> 0x19a92a0
0x19a92f9 jne -> 0x19a92b2
0x19a930a jne -> 0x19a92bb
0x19a931b je -> 0x19a9330
0x19a9338 jmp -> 0x19a9367
0x19a934e jmp -> 0x19a8cd3
0x19a936f je -> 0x19a93ae
0x19a9386 je -> 0x19a95d0
0x19a9396 je -> 0x19a9598
0x19a93a8 je -> 0x19a95aa
0x19a93c5 jne -> 0x19a9490
0x19a93de je -> 0x19a93ef
0x19a93e9 je -> 0x19a9558
0x19a9405 jb -> 0x19a8b78
0x19a9429 jne -> 0x19a9433
0x19a942d jg -> 0x19a9624
0x19a945a jne -> 0x19a964d
0x19a9469 je -> 0x19a947a
0x19a9474 je -> 0x19a9674
0x19a949d jmp -> 0x19a93cb
0x19a94c5 jmp -> 0x19a92b2
0x19a950d je -> 0x19a9240
0x19a9518 jmp -> 0x19a9240
0x19a953c jmp -> 0x19a91be
0x19a9550 jmp -> 0x19a8e7b
0x19a9565 jmp -> 0x19a93ef
0x19a957d jne -> 0x19a8fce
0x19a9586 je -> 0x19a9039
0x19a958c jmp -> 0x19a9008
0x19a95a4 jne -> 0x19a93ae
0x19a95b2 jmp -> 0x19a93ae
0x19a95cb jmp -> 0x19a927e
0x19a95ea jmp -> 0x19a93ae
0x19a960d jne -> 0x19a9443
0x19a9615 jle -> 0x19a9443
0x19a962f jmp -> 0x19a9435
0x19a963b jmp -> 0x19a927e
0x19a9648 jmp -> 0x19a8c32
0x19a965a jmp -> 0x19a9460
0x19a9667 jmp -> 0x19a8b44
0x19a966f jmp -> 0x19a917f
0x19a9681 jmp -> 0x19a947a
0x19a9689 jns -> 0x19a9690
0x19a96c6 jmp -> 0x19a8f93
0x19a96d7 jmp -> 0x19a8ddf
0x19a96f2 jmp -> 0x19a96b2
```

### Direct callsites of `0x19a8a80`
```text
  cd3224:	e8 57 58 cd 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbb2b:	e8 50 cf cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbb9b:	e8 e0 ce cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbc0b:	e8 70 ce cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbc7b:	e8 00 ce cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbdaa:	e8 d1 cc cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cdbf57:	e8 24 cb cc 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  cec9f8:	e8 83 c0 cb 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
  df2bef:	e8 8c 5e bb 00       	call   19a8a80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1831f0>
```

### Default-instance candidate references
#### `0x313a820`
```text
  6a4439:	48 8d 35 e0 63 a9 02 	lea    0x2a963e0(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  763fdc:	48 8d 0d 3d 68 9d 02 	lea    0x29d683d(%rip),%rcx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  7ccfb1:	48 8d 3d 68 d8 96 02 	lea    0x296d868(%rip),%rdi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  b2e516:	48 8d 0d 03 c3 60 02 	lea    0x260c303(%rip),%rcx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  b39402:	48 8d 15 17 14 60 02 	lea    0x2601417(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bba169:	48 8d 05 b0 06 58 02 	lea    0x25806b0(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bba6c9:	48 8d 35 50 01 58 02 	lea    0x2580150(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bde1b8:	48 8d 05 61 c6 55 02 	lea    0x255c661(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bde918:	48 8d 05 01 bf 55 02 	lea    0x255bf01(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bedff1:	48 8d 05 28 c8 54 02 	lea    0x254c828(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bee1a1:	48 8d 05 78 c6 54 02 	lea    0x254c678(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bee3f9:	48 8d 05 20 c4 54 02 	lea    0x254c420(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bee5a1:	48 8d 05 78 c2 54 02 	lea    0x254c278(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bee5d1:	48 8d 05 48 c2 54 02 	lea    0x254c248(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bee7d9:	48 8d 05 40 c0 54 02 	lea    0x254c040(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  bfe104:	48 8d 0d 15 c7 53 02 	lea    0x253c715(%rip),%rcx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  c14511:	48 8d 15 08 63 52 02 	lea    0x2526308(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  c14fca:	48 8d 15 4f 58 52 02 	lea    0x252584f(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  c15310:	48 8d 15 09 55 52 02 	lea    0x2525509(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  c21f71:	48 8d 15 a8 88 51 02 	lea    0x25188a8(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  c2bc9c:	48 8d 05 7d eb 50 02 	lea    0x250eb7d(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cd3198:	48 8d 15 81 76 46 02 	lea    0x2467681(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cd4e64:	48 8d 15 b5 59 46 02 	lea    0x24659b5(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cd533d:	48 8d 15 dc 54 46 02 	lea    0x24654dc(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cec8d2:	48 8d 15 47 df 44 02 	lea    0x244df47(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cecc72:	48 8d 15 a7 db 44 02 	lea    0x244dba7(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  cecf63:	48 8d 15 b6 d8 44 02 	lea    0x244d8b6(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e3a1e8:	48 8d 05 31 06 30 02 	lea    0x2300631(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e3a416:	48 8d 05 03 04 30 02 	lea    0x2300403(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e3a44c:	48 8d 05 cd 03 30 02 	lea    0x23003cd(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e4620c:	48 8d 15 0d 46 2f 02 	lea    0x22f460d(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e4653d:	48 8d 15 dc 42 2f 02 	lea    0x22f42dc(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e466e8:	48 8d 15 31 41 2f 02 	lea    0x22f4131(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e4672c:	48 8d 15 ed 40 2f 02 	lea    0x22f40ed(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e46976:	48 8d 15 a3 3e 2f 02 	lea    0x22f3ea3(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  e82e02:	48 8d 15 17 7a 2b 02 	lea    0x22b7a17(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  eae863:	48 8d 35 b6 bf 28 02 	lea    0x228bfb6(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  eae975:	48 8d 0d a4 be 28 02 	lea    0x228bea4(%rip),%rcx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  eaed1e:	48 8d 3d fb ba 28 02 	lea    0x228bafb(%rip),%rdi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  ebbf4d:	48 8d 15 cc e8 27 02 	lea    0x227e8cc(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  ed4ac4:	48 8d 05 55 5d 26 02 	lea    0x2265d55(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  ed4ba3:	48 8d 05 76 5c 26 02 	lea    0x2265c76(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  ed53a6:	48 8d 15 73 54 26 02 	lea    0x2265473(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  ef6053:	48 8d 35 c6 47 24 02 	lea    0x22447c6(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
  efea7b:	48 8d 0d 9e bd 23 02 	lea    0x223bd9e(%rip),%rcx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1747e49:	48 8d 05 d0 29 9f 01 	lea    0x19f29d0(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1747fa9:	48 8d 05 70 28 9f 01 	lea    0x19f2870(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1748131:	48 8d 05 e8 26 9f 01 	lea    0x19f26e8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1748169:	48 8d 05 b0 26 9f 01 	lea    0x19f26b0(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17482ed:	48 8d 05 2c 25 9f 01 	lea    0x19f252c(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1748499:	48 8d 05 80 23 9f 01 	lea    0x19f2380(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1749041:	48 8d 05 d8 17 9f 01 	lea    0x19f17d8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17491c1:	48 8d 05 58 16 9f 01 	lea    0x19f1658(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1786bde:	48 8d 05 3b 3c 9b 01 	lea    0x19b3c3b(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1786c16:	48 8d 05 03 3c 9b 01 	lea    0x19b3c03(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17874d9:	48 8d 05 40 33 9b 01 	lea    0x19b3340(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 178ce11:	48 8d 05 08 da 9a 01 	lea    0x19ada08(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 178d361:	48 8d 05 b8 d4 9a 01 	lea    0x19ad4b8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17a9331:	48 8d 05 e8 14 99 01 	lea    0x19914e8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17af076:	48 8d 35 a3 b7 98 01 	lea    0x198b7a3(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17af2d6:	48 8d 35 43 b5 98 01 	lea    0x198b543(%rip),%rsi        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d01fe:	48 8d 05 1b a6 96 01 	lea    0x196a61b(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d040e:	48 8d 05 0b a4 96 01 	lea    0x196a40b(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d0701:	48 8d 05 18 a1 96 01 	lea    0x196a118(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d0821:	48 8d 05 f8 9f 96 01 	lea    0x1969ff8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d7601:	48 8d 05 18 32 96 01 	lea    0x1963218(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17d93f1:	48 8d 05 28 14 96 01 	lea    0x1961428(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17fca51:	48 8d 05 c8 dd 93 01 	lea    0x193ddc8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17fcbb1:	48 8d 05 68 dc 93 01 	lea    0x193dc68(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 17fccd1:	48 8d 05 48 db 93 01 	lea    0x193db48(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1805c69:	48 8d 05 b0 4b 93 01 	lea    0x1934bb0(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 18145ed:	48 8d 15 2c 62 92 01 	lea    0x192622c(%rip),%rdx        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 18224a1:	48 8d 05 78 83 91 01 	lea    0x1918378(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 18226e1:	48 8d 05 38 81 91 01 	lea    0x1918138(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1822811:	48 8d 05 08 80 91 01 	lea    0x1918008(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1822841:	48 8d 05 d8 7f 91 01 	lea    0x1917fd8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 18229a1:	48 8d 05 78 7e 91 01 	lea    0x1917e78(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1822ae1:	48 8d 05 38 7d 91 01 	lea    0x1917d38(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1822c71:	48 8d 05 a8 7b 91 01 	lea    0x1917ba8(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
 1823cd5:	48 8d 05 44 6b 91 01 	lea    0x1916b44(%rip),%rax        # 313a820 <typeinfo for QSGRectangleNode@@Base+0x12398>
```
#### `0x313a860`
```text
  6a44e9:	48 8d 35 70 63 a9 02 	lea    0x2a96370(%rip),%rsi        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  763fd1:	48 8d 05 88 68 9d 02 	lea    0x29d6888(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde251:	48 8d 05 08 c6 55 02 	lea    0x255c608(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde399:	48 8d 05 c0 c4 55 02 	lea    0x255c4c0(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde509:	48 8d 05 50 c3 55 02 	lea    0x255c350(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde679:	48 8d 05 e0 c1 55 02 	lea    0x255c1e0(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde7e9:	48 8d 05 70 c0 55 02 	lea    0x255c070(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  bde9b1:	48 8d 05 a8 be 55 02 	lea    0x255bea8(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  be2899:	48 8d 05 c0 7f 55 02 	lea    0x2557fc0(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  be2a09:	48 8d 05 50 7e 55 02 	lea    0x2557e50(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cd31fa:	48 8d 15 5f 76 46 02 	lea    0x246765f(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbb16:	48 8d 15 43 ed 45 02 	lea    0x245ed43(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbb86:	48 8d 15 d3 ec 45 02 	lea    0x245ecd3(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbbf6:	48 8d 15 63 ec 45 02 	lea    0x245ec63(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbc66:	48 8d 15 f3 eb 45 02 	lea    0x245ebf3(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbd8f:	48 8d 15 ca ea 45 02 	lea    0x245eaca(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cdbf3c:	48 8d 15 1d e9 45 02 	lea    0x245e91d(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  cec9bb:	48 8d 05 9e de 44 02 	lea    0x244de9e(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
  df2bd4:	48 8d 15 85 7c 34 02 	lea    0x2347c85(%rip),%rdx        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747881:	48 8d 05 d8 2f 9f 01 	lea    0x19f2fd8(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747971:	48 8d 05 e8 2e 9f 01 	lea    0x19f2ee8(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747a61:	48 8d 05 f8 2d 9f 01 	lea    0x19f2df8(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747b51:	48 8d 05 08 2d 9f 01 	lea    0x19f2d08(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747c41:	48 8d 05 18 2c 9f 01 	lea    0x19f2c18(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747d31:	48 8d 05 28 2b 9f 01 	lea    0x19f2b28(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747e71:	48 8d 05 e8 29 9f 01 	lea    0x19f29e8(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1747fd1:	48 8d 05 88 28 9f 01 	lea    0x19f2888(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 17d0a01:	48 8d 05 58 9e 96 01 	lea    0x1969e58(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 17d0b21:	48 8d 05 38 9d 96 01 	lea    0x1969d38(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 18235cd:	48 8d 05 8c 72 91 01 	lea    0x191728c(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
 1823a9d:	48 8d 05 bc 6d 91 01 	lea    0x1916dbc(%rip),%rax        # 313a860 <typeinfo for QSGRectangleNode@@Base+0x123d8>
```
#### `0x314b480`
```text
  6a4465:	48 8d 35 14 70 aa 02 	lea    0x2aa7014(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  6a7cff:	48 8d 1d 7a 37 aa 02 	lea    0x2aa377a(%rip),%rbx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  831691:	48 8d 05 e8 9d 91 02 	lea    0x2919de8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  af4890:	48 8d 05 e9 6b 65 02 	lea    0x2656be9(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be15df:	48 8d 35 9a 9e 56 02 	lea    0x2569e9a(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be1718:	48 8d 35 61 9d 56 02 	lea    0x2569d61(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be18a0:	48 8d 35 d9 9b 56 02 	lea    0x2569bd9(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be1a60:	48 8d 35 19 9a 56 02 	lea    0x2569a19(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be1ba0:	48 8d 35 d9 98 56 02 	lea    0x25698d9(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be1ce0:	48 8d 35 99 97 56 02 	lea    0x2569799(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be1e3e:	48 8d 35 3b 96 56 02 	lea    0x256963b(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be2027:	48 8d 35 52 94 56 02 	lea    0x2569452(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be2169:	48 8d 05 10 93 56 02 	lea    0x2569310(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be45f1:	48 8d 05 88 6e 56 02 	lea    0x2566e88(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be6111:	48 8d 05 68 53 56 02 	lea    0x2565368(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be6381:	48 8d 05 f8 50 56 02 	lea    0x25650f8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be64e1:	48 8d 05 98 4f 56 02 	lea    0x2564f98(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  be6641:	48 8d 05 38 4e 56 02 	lea    0x2564e38(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  bedfc1:	48 8d 05 b8 d4 55 02 	lea    0x255d4b8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  bee171:	48 8d 05 08 d3 55 02 	lea    0x255d308(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  beeccd:	48 8d 05 ac c7 55 02 	lea    0x255c7ac(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  bef42d:	48 8d 05 4c c0 55 02 	lea    0x255c04c(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  c25790:	48 8d 15 e9 5c 52 02 	lea    0x2525ce9(%rip),%rdx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  c2595c:	48 8d 35 1d 5b 52 02 	lea    0x2525b1d(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  cc2ed7:	48 8d 05 a2 85 48 02 	lea    0x24885a2(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  ceccde:	48 8d 05 9b e7 45 02 	lea    0x245e79b(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  cecd4f:	48 8d 35 2a e7 45 02 	lea    0x245e72a(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  cecfb0:	48 8d 05 c9 e4 45 02 	lea    0x245e4c9(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  ced0ef:	48 8d 05 8a e3 45 02 	lea    0x245e38a(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e5fbc1:	48 8d 05 b8 b8 2e 02 	lea    0x22eb8b8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e5fd31:	48 8d 05 48 b7 2e 02 	lea    0x22eb748(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e5fea1:	48 8d 05 d8 b5 2e 02 	lea    0x22eb5d8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e613c8:	48 8d 05 b1 a0 2e 02 	lea    0x22ea0b1(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e6f31c:	48 8d 05 5d c1 2d 02 	lea    0x22dc15d(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  e7c510:	48 8d 05 69 ef 2c 02 	lea    0x22cef69(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  eb223e:	48 8d 0d 3b 92 29 02 	lea    0x229923b(%rip),%rcx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  eb228b:	48 8d 05 ee 91 29 02 	lea    0x22991ee(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  ebbf12:	48 8d 05 67 f5 28 02 	lea    0x228f567(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  ebbfa8:	48 8d 15 d1 f4 28 02 	lea    0x228f4d1(%rip),%rdx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  ed2700:	48 8d 0d 79 8d 27 02 	lea    0x2278d79(%rip),%rcx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  efbe0b:	48 8d 0d 6e f6 24 02 	lea    0x224f66e(%rip),%rcx        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
  efbe61:	48 8d 05 18 f6 24 02 	lea    0x224f618(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1749011:	48 8d 05 68 24 a0 01 	lea    0x1a02468(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1749191:	48 8d 05 e8 22 a0 01 	lea    0x1a022e8(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17493e9:	48 8d 05 90 20 a0 01 	lea    0x1a02090(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17494f9:	48 8d 05 80 1f a0 01 	lea    0x1a01f80(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17495c9:	48 8d 05 b0 1e a0 01 	lea    0x1a01eb0(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1749699:	48 8d 05 e0 1d a0 01 	lea    0x1a01de0(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1749769:	48 8d 05 10 1d a0 01 	lea    0x1a01d10(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1749875:	48 8d 05 04 1c a0 01 	lea    0x1a01c04(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 174c3f1:	48 8d 05 88 f0 9f 01 	lea    0x19ff088(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17544ff:	48 8d 35 7a 6f 9f 01 	lea    0x19f6f7a(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1773b01:	48 8d 05 78 79 9d 01 	lea    0x19d7978(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1773c2c:	48 8d 05 4d 78 9d 01 	lea    0x19d784d(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1773e69:	48 8d 05 10 76 9d 01 	lea    0x19d7610(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 177960c:	48 8d 35 6d 1e 9d 01 	lea    0x19d1e6d(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1787223:	48 8d 05 56 42 9c 01 	lea    0x19c4256(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17a1a99:	48 8d 05 e0 99 9a 01 	lea    0x19a99e0(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17d3ed9:	48 8d 05 a0 75 97 01 	lea    0x19775a0(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17d3ff9:	48 8d 05 80 74 97 01 	lea    0x1977480(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 17de9d3:	48 8d 05 a6 ca 96 01 	lea    0x196caa6(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 181b7fc:	48 8d 35 7d fc 92 01 	lea    0x192fc7d(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 181bbd8:	48 8d 35 a1 f8 92 01 	lea    0x192f8a1(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 181c334:	48 8d 35 45 f1 92 01 	lea    0x192f145(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1822f20:	48 8d 35 59 85 92 01 	lea    0x1928559(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 1822f6c:	48 8d 35 0d 85 92 01 	lea    0x192850d(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 182306c:	48 8d 05 0d 84 92 01 	lea    0x192840d(%rip),%rax        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
 19a8e94:	48 8d 35 e5 25 7a 01 	lea    0x17a25e5(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
```

### Bounded disassembly `0x19a8a80..0x19a9700`
```text
0x19a8a80: push   %r15
0x19a8a82: lea    0x1791d37(%rip),%rax        # 313a7c0 <typeinfo for QSGRectangleNode@@Base+0x12338>
0x19a8a89: push   %r14
0x19a8a8b: push   %r13
0x19a8a8d: push   %r12
0x19a8a8f: push   %rbp
0x19a8a90: push   %rbx
0x19a8a91: lea    0x17cd2e8(%rip),%rbx        # 3175d80 <typeinfo for QSGRectangleNode@@Base+0x4d8f8>
0x19a8a98: sub    $0x188,%rsp
0x19a8a9f: test   %rsi,%rsi
0x19a8aa2: cmove  %rax,%rsi
0x19a8aa6: mov    %rdi,0x8(%rsp)
0x19a8aab: mov    %rdx,0x48(%rsp)
0x19a8ab0: cmpl   $0x1,0x1c(%rsi)
0x19a8ab4: mov    %cl,0x57(%rsp)
0x19a8ab8: jne    19a8abe <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18322e>
0x19a8aba: mov    0x10(%rsi),%rbx
0x19a8abe: movq   $0x0,0xe8(%rsp)
0x19a8ac5: 
0x19a8aca: lea    0x17034f7(%rip),%rax        # 30abfc8 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x1bfe8>
0x19a8ad1: movq   $0x0,0xd8(%rsp)
0x19a8ad8: 
0x19a8add: mov    0x18(%rbx),%esi
0x19a8ae0: movq   $0x0,0xc8(%rsp)
0x19a8ae7: 
0x19a8aec: mov    %rax,0xc0(%rsp)
0x19a8af3: 
0x19a8af4: movq   $0x0,0xd0(%rsp)
0x19a8afb: 
0x19a8b00: movq   $0x0,0xe0(%rsp)
0x19a8b07: 
0x19a8b0c: test   %esi,%esi
0x19a8b0e: je     19a8b2b <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18329b>
0x19a8b10: mov    0x20(%rbx),%rdx
0x19a8b14: lea    0xd0(%rsp),%rdi
0x19a8b1b: 
0x19a8b1c: lea    -0x18d233(%rip),%rcx        # 181b8f0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xc7d0c0>
0x19a8b23: xor    %r8d,%r8d
0x19a8b26: call   1983960 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x15e0d0>
0x19a8b2b: mov    0x8(%rbx),%rax
0x19a8b2f: test   $0x1,%al
0x19a8b31: je     19a8b51 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1832c1>
0x19a8b33: mov    %rax,%rdx
0x19a8b36: and    $0x1,%edx
0x19a8b39: jne    19a965f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183dcf>
0x19a8b3f: call   *%rdx
0x19a8b41: mov    %rax,%rsi
0x19a8b44: lea    0xc8(%rsp),%rdi
0x19a8b4b: 
0x19a8b4c: call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
0x19a8b51: mov    0xd8(%rsp),%edx
0x19a8b58: test   %edx,%edx
0x19a8b5a: je     19a95ef <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183d5f>
0x19a8b60: movq   $0x0,0x30(%rsp)
0x19a8b67: 
0x19a8b69: movq   $0x0,0x28(%rsp)
0x19a8b70: 
0x19a8b72: nopw   0x0(%rax,%rax,1)
0x19a8b78: movslq 0x28(%rsp),%rax
0x19a8b7d: mov    0xe0(%rsp),%rdx
0x19a8b84: 
0x19a8b85: pxor   %xmm0,%xmm0
0x19a8b89: mov    0x8(%rdx,%rax,8),%rbx
0x19a8b8e: lea    0x171b063(%rip),%rax        # 30c3bf8 <QMetaSequence::MetaSequence<QList<QObject*> >::value@@Base+0x33c18>
0x19a8b95: movaps %xmm0,0x160(%rsp)
0x19a8b9c: 
0x19a8b9d: movaps %xmm0,0x150(%rsp)
0x19a8ba4: 
0x19a8ba5: movaps %xmm0,0x170(%rsp)
0x19a8bac: 
0x19a8bad: movaps %xmm0,0x140(%rsp)
0x19a8bb4: 
0x19a8bb5: mov    %rax,0x130(%rsp)
0x19a8bbc: 
0x19a8bbd: mov    0x28(%rbx),%eax
0x19a8bc0: movq   $0x0,0x168(%rsp)
0x19a8bc7: 
0x19a8bcc: mov    0x38(%rbx),%esi
0x19a8bcf: movq   $0x0,0x138(%rsp)
0x19a8bd6: 
0x19a8bdb: mov    %eax,0x158(%rsp)
0x19a8be2: movq   $0x0,0x160(%rsp)
0x19a8be9: 
0x19a8bee: movq   $0x0,0x170(%rsp)
0x19a8bf5: 
0x19a8bfa: test   %esi,%esi
0x19a8bfc: je     19a8c19 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183389>
0x19a8bfe: mov    0x40(%rbx),%rdx
0x19a8c02: lea    0x160(%rsp),%rdi
0x19a8c09: 
0x19a8c0a: lea    -0x22f5a1(%rip),%rcx        # 1779670 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdae40>
0x19a8c11: xor    %r8d,%r8d
0x19a8c14: call   1983960 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x15e0d0>
0x19a8c19: mov    0x8(%rbx),%rax
0x19a8c1d: test   $0x1,%al
0x19a8c1f: je     19a8c3f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1833af>
0x19a8c21: mov    %rax,%rdx
0x19a8c24: and    $0x1,%edx
0x19a8c27: jne    19a9640 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183db0>
0x19a8c2d: call   *%rdx
0x19a8c2f: mov    %rax,%rsi
0x19a8c32: lea    0x138(%rsp),%rdi
0x19a8c39: 
0x19a8c3a: call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
0x19a8c3f: lea    0x140(%rsp),%rax
0x19a8c46: 
0x19a8c47: lea    0x10(%rbx),%rdx
0x19a8c4b: lea    0x17cd0ce(%rip),%rsi        # 3175d20 <typeinfo for QSGRectangleNode@@Base+0x4d898>
0x19a8c52: mov    %rax,%rdi
0x19a8c55: mov    %rax,0x38(%rsp)
0x19a8c5a: call   1ab4e50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x28f5c0>
0x19a8c5f: mov    0x48(%rbx),%eax
0x19a8c62: mov    0x48(%rsp),%rbx
0x19a8c67: xor    %edx,%edx
0x19a8c69: movslq 0x3c(%rbx),%r9
0x19a8c6d: mov    0x38(%rbx),%r8d
0x19a8c71: mov    %eax,0x178(%rsp)
0x19a8c78: lea    0x15b8921(%rip),%rax        # 2f615a0 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xebfe40>
0x19a8c7f: mov    0x40(%rbx),%esi
0x19a8c82: mov    0x48(%rbx),%edi
0x19a8c85: mov    %rax,0x80(%rsp)
0x19a8c8c: 
0x19a8c8d: mov    %r9,%rcx
0x19a8c90: mov    0x30(%rsp),%rax
0x19a8c95: imul   %r8d,%ecx
0x19a8c99: movslq %ecx,%rcx
0x19a8c9c: div    %rcx
0x19a8c9f: cltd
0x19a8ca0: idiv   %esi
0x19a8ca2: mov    0x30(%rsp),%rax
0x19a8ca7: mov    %edx,%r10d
0x19a8caa: xor    %edx,%edx
0x19a8cac: div    %r9
0x19a8caf: mov    %rdx,%rcx
0x19a8cb2: cltd
0x19a8cb3: add    0xc(%rbx),%ecx
0x19a8cb6: idiv   %r8d
0x19a8cb9: mov    0x10(%rbx),%eax
0x19a8cbc: add    0x8(%rbx),%edx
0x19a8cbf: cmpb   $0x0,0x57(%rsp)
0x19a8cc4: je     19a9340 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183ab0>
0x19a8cca: add    %r10d,%eax
0x19a8ccd: sub    %eax,%edi
0x19a8ccf: add    %edi,%edx
0x19a8cd1: add    %edi,%ecx
0x19a8cd3: mov    %eax,0x90(%rsp)
0x19a8cda: mov    0x8(%rsp),%rax
0x19a8cdf: lea    0x60(%rsp),%rdi
0x19a8ce4: mov    %edx,0x88(%rsp)
0x19a8ceb: mov    0x10(%rax),%rsi
0x19a8cef: lea    0x80(%rsp),%rax
0x19a8cf6: 
0x19a8cf7: mov    %ecx,0x8c(%rsp)
0x19a8cfe: mov    %rax,0x10(%rsp)
0x19a8d03: mov    %rax,%rdx
0x19a8d06: mov    (%rsi),%rax
0x19a8d09: call   *0xa0(%rax)
0x19a8d0f: pxor   %xmm0,%xmm0
0x19a8d13: cmpq   $0x0,0x60(%rsp)
0x19a8d19: movq   $0x0,0xb0(%rsp)
0x19a8d20: 
0x19a8d25: movaps %xmm0,0xa0(%rsp)
0x19a8d2c: 
0x19a8d2d: je     19a9358 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183ac8>
0x19a8d33: mov    0x168(%rsp),%eax
0x19a8d3a: xor    %ebp,%ebp
0x19a8d3c: lea    0x16de765(%rip),%r12        # 30874a8 <std::__detail::__to_chars_10_impl<unsigned int>(char*, unsigned int, unsigned int)::__digits@@Base+0xfe5d48>
0x19a8d43: test   %eax,%eax
0x19a8d45: jne    19a8e12 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183582>
0x19a8d4b: jmp    19a94ca <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183c3a>
0x19a8d50: mov    0xa8(%rsp),%r13
0x19a8d57: 
0x19a8d58: cmp    0xb0(%rsp),%r13
0x19a8d5f: 
0x19a8d60: je     19a8f40 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1836b0>
0x19a8d66: movq   %rax,%xmm0
0x19a8d6b: movq   %rbx,%xmm4
0x19a8d70: punpcklqdq %xmm4,%xmm0
0x19a8d74: movups %xmm0,0x0(%r13)
0x19a8d79: test   %rbx,%rbx
0x19a8d7c: je     19a96cb <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183e3b>
0x19a8d82: mov    0x1786d1f(%rip),%rax        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
0x19a8d89: cmpb   $0x0,(%rax)
0x19a8d8c: je     19a8f18 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183688>
0x19a8d92: add    $0x10,%r13
0x19a8d96: addl   $0x1,0x8(%rbx)
0x19a8d9a: mov    %r13,0xa8(%rsp)
0x19a8da1: 
0x19a8da2: mov    0x8(%rbx),%rax
0x19a8da6: lea    0x8(%rbx),%rdx
0x19a8daa: movabs $0x100000001,%rcx
0x19a8db1: 
0x19a8db4: cmp    %rcx,%rax
0x19a8db7: je     19a9080 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1837f0>
0x19a8dbd: mov    0x1786ce4(%rip),%rax        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
0x19a8dc4: cmpb   $0x0,(%rax)
0x19a8dc7: je     19a9060 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1837d0>
0x19a8dcd: mov    0x8(%rbx),%eax
0x19a8dd0: lea    -0x1(%rax),%edx
0x19a8dd3: mov    %edx,0x8(%rbx)
0x19a8dd6: cmp    $0x1,%eax
0x19a8dd9: je     19a9072 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1837e2>
0x19a8ddf: mov    %r12,0xf0(%rsp)
0x19a8de6: 
0x19a8de7: testb  $0x1,0xf8(%rsp)
0x19a8dee: 
0x19a8def: jne    19a8f00 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183670>
0x19a8df5: mov    %r15,%rdi
0x19a8df8: add    $0x1,%rbp
0x19a8dfc: call   1aab450 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x285bc0>
0x19a8e01: movslq 0x168(%rsp),%rax
0x19a8e08: 
0x19a8e09: cmp    %rax,%rbp
0x19a8e0c: jae    19a90a0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183810>
0x19a8e12: mov    0x170(%rsp),%rdx
0x19a8e19: 
0x19a8e1a: movslq %ebp,%rax
0x19a8e1d: pxor   %xmm1,%xmm1
0x19a8e21: mov    0x8(%rdx,%rax,8),%rbx
0x19a8e26: movaps %xmm1,0x110(%rsp)
0x19a8e2d: 
0x19a8e2e: movq   $0x0,0xf8(%rsp)
0x19a8e35: 
0x19a8e3a: movq   $0x0,0x120(%rsp)
0x19a8e41: 
0x19a8e46: mov    0x8(%rbx),%rsi
0x19a8e4a: movaps %xmm1,0x100(%rsp)
0x19a8e51: 
0x19a8e52: mov    0x28(%rbx),%eax
0x19a8e55: mov    %r12,0xf0(%rsp)
0x19a8e5c: 
0x19a8e5d: mov    %eax,0x118(%rsp)
0x19a8e64: test   $0x1,%sil
0x19a8e68: je     19a8e88 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1835f8>
0x19a8e6a: mov    %rsi,%rax
0x19a8e6d: and    $0x1,%eax
0x19a8e70: jne    19a9548 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183cb8>
0x19a8e76: call   *%rax
0x19a8e78: mov    %rax,%rsi
0x19a8e7b: lea    0xf8(%rsp),%rdi
0x19a8e82: 
0x19a8e83: call   1b13c80 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x2ee3f0>
0x19a8e88: lea    0x100(%rsp),%r15
0x19a8e8f: 
0x19a8e90: lea    0x10(%rbx),%rdx
0x19a8e94: lea    0x17a25e5(%rip),%rsi        # 314b480 <typeinfo for QSGRectangleNode@@Base+0x22ff8>
0x19a8e9b: mov    %r15,%rdi
0x19a8e9e: call   1ab4e50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x28f5c0>
0x19a8ea3: mov    0x30(%rbx),%eax
0x19a8ea6: mov    0x10(%rsp),%rcx
0x19a8eab: lea    0x70(%rsp),%rdi
0x19a8eb0: mov    0x8(%rsp),%rsi
0x19a8eb5: mov    %eax,0x120(%rsp)
0x19a8ebc: lea    0xf0(%rsp),%rax
0x19a8ec3: 
0x19a8ec4: mov    %rax,%rdx
0x19a8ec7: mov    %rax,0x20(%rsp)
0x19a8ecc: call   ceca50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0x14e220>
0x19a8ed1: mov    0x70(%rsp),%rax
0x19a8ed6: mov    0x78(%rsp),%rbx
0x19a8edb: mov    %rax,0x18(%rsp)
0x19a8ee0: test   %rax,%rax
0x19a8ee3: jne    19a8d50 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1834c0>
0x19a8ee9: test   %rbx,%rbx
0x19a8eec: je     19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a8ef2: jmp    19a8da2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183512>
0x19a8ef7: nopw   0x0(%rax,%rax,1)
0x19a8efe: 
0x19a8f00: lea    0xf8(%rsp),%rdi
0x19a8f07: 
0x19a8f08: call   177c150 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::basic_string()@@Base+0xbdd920>
0x19a8f0d: jmp    19a8df5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183565>
0x19a8f12: nopw   0x0(%rax,%rax,1)
0x19a8f18: lock addl $0x1,0x8(%rbx)
0x19a8f1d: addq   $0x10,0xa8(%rsp)
0x19a8f24: 
0x19a8f26: mov    0x78(%rsp),%rbx
0x19a8f2b: test   %rbx,%rbx
0x19a8f2e: je     19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a8f34: jmp    19a8da2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183512>
0x19a8f39: nopl   0x0(%rax)
0x19a8f40: mov    0xa0(%rsp),%r14
0x19a8f47: 
0x19a8f48: mov    %r13,%rax
0x19a8f4b: movabs $0x7ffffffffffffff,%rcx
0x19a8f52: 
0x19a8f55: sub    %r14,%rax
0x19a8f58: mov    %rax,0x40(%rsp)
0x19a8f5d: sar    $0x4,%rax
0x19a8f61: cmp    %rcx,%rax
0x19a8f64: je     19a96f4 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183e64>
0x19a8f6a: cmp    %r14,%r13
0x19a8f6d: mov    $0x1,%edx
0x19a8f72: cmovne %rax,%rdx
0x19a8f76: add    %rdx,%rax
0x19a8f79: jb     19a96e0 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183e50>
0x19a8f7f: test   %rax,%rax
0x19a8f82: jne    19a9695 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183e05>
0x19a8f88: mov    $0x10,%eax
0x19a8f8d: xor    %r9d,%r9d
0x19a8f90: xor    %r8d,%r8d
0x19a8f93: movq   0x18(%rsp),%xmm0
0x19a8f99: mov    0x40(%rsp),%rcx
0x19a8f9e: movq   %rbx,%xmm5
0x19a8fa3: punpcklqdq %xmm5,%xmm0
0x19a8fa7: movups %xmm0,(%r8,%rcx,1)
0x19a8fac: test   %rbx,%rbx
0x19a8faf: je     19a8fc5 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183735>
0x19a8fb1: mov    0x1786af0(%rip),%rdx        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
0x19a8fb8: cmpb   $0x0,(%rdx)
0x19a8fbb: je     19a9570 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183ce0>
0x19a8fc1: addl   $0x1,0x8(%rbx)
0x19a8fc5: cmp    %r14,%r13
0x19a8fc8: je     19a9583 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183cf3>
0x19a8fce: lea    -0x10(%r13),%rcx
0x19a8fd2: xor    %eax,%eax
0x19a8fd4: xor    %edx,%edx
0x19a8fd6: sub    %r14,%rcx
0x19a8fd9: shr    $0x4,%rcx
0x19a8fdd: add    $0x1,%rcx
0x19a8fe1: nopl   0x0(%rax)
0x19a8fe8: movdqu (%r14,%rax,1),%xmm2
0x19a8fee: add    $0x1,%rdx
0x19a8ff2: movups %xmm2,(%r8,%rax,1)
0x19a8ff7: add    $0x10,%rax
0x19a8ffb: cmp    %rcx,%rdx
0x19a8ffe: jb     19a8fe8 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183758>
0x19a9000: sub    %r14,%r13
0x19a9003: lea    0x10(%r8,%r13,1),%rax
0x19a9008: mov    0xb0(%rsp),%rsi
0x19a900f: 
0x19a9010: mov    %r14,%rdi
0x19a9013: mov    %rax,0x58(%rsp)
0x19a9018: mov    %r9,0x40(%rsp)
0x19a901d: sub    %r14,%rsi
0x19a9020: mov    %r8,0x18(%rsp)
0x19a9025: call   4de900 <operator delete(void*, unsigned long)@plt>
0x19a902a: mov    0x58(%rsp),%rax
0x19a902f: mov    0x40(%rsp),%r9
0x19a9034: mov    0x18(%rsp),%r8
0x19a9039: mov    %r8,0xa0(%rsp)
0x19a9040: 
0x19a9041: mov    %rax,0xa8(%rsp)
0x19a9048: 
0x19a9049: mov    %r9,0xb0(%rsp)
0x19a9050: 
0x19a9051: test   %rbx,%rbx
0x19a9054: je     19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a905a: jmp    19a8da2 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183512>
0x19a905f: nop
0x19a9060: mov    $0xffffffff,%eax
0x19a9065: lock xadd %eax,(%rdx)
0x19a9069: cmp    $0x1,%eax
0x19a906c: jne    19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a9072: mov    %rbx,%rdi
0x19a9075: call   6b3a90 <std::runtime_error::~runtime_error()@plt+0x1d37c0>
0x19a907a: jmp    19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a907f: nop
0x19a9080: mov    (%rbx),%rax
0x19a9083: mov    %rbx,%rdi
0x19a9086: movq   $0x0,0x8(%rbx)
0x19a908d: 
0x19a908e: call   *0x10(%rax)
0x19a9091: mov    (%rbx),%rax
0x19a9094: mov    %rbx,%rdi
0x19a9097: call   *0x18(%rax)
0x19a909a: jmp    19a8ddf <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x18354f>
0x19a909f: nop
0x19a90a0: mov    0xa8(%rsp),%rbx
0x19a90a7: 
0x19a90a8: mov    0xa0(%rsp),%rax
0x19a90af: 
0x19a90b0: cmp    %rax,%rbx
0x19a90b3: je     19a94d7 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183c47>
0x19a90b9: mov    0x60(%rsp),%rbp
0x19a90be: sub    %rax,%rbx
0x19a90c1: pxor   %xmm0,%xmm0
0x19a90c5: movabs $0x7ffffffffffffff0,%rax
0x19a90cc: 
0x19a90cf: mov    0x0(%rbp),%rdx
0x19a90d3: mov    0x88(%rdx),%r12
0x19a90da: movaps %xmm0,0xf0(%rsp)
0x19a90e1: 
0x19a90e2: movq   $0x0,0x100(%rsp)
0x19a90e9: 
0x19a90ee: cmp    %rbx,%rax
0x19a90f1: jb     19a9686 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183df6>
0x19a90f7: mov    %rbx,%rdi
0x19a90fa: call   4df670 <operator new(unsigned long)@plt>
0x19a90ff: movq   %rax,%xmm0
0x19a9104: mov    0xa8(%rsp),%rsi
0x19a910b: 
0x19a910c: mov    0xa0(%rsp),%rdx
0x19a9113: 
0x19a9114: add    %rax,%rbx
0x19a9117: punpcklqdq %xmm0,%xmm0
0x19a911b: mov    %rbx,0x100(%rsp)
0x19a9122: 
0x19a9123: movaps %xmm0,0xf0(%rsp)
0x19a912a: 
0x19a912b: cmp    %rdx,%rsi
0x19a912e: je     19a966c <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x183ddc>
0x19a9134: sub    %rdx,%rsi
0x19a9137: add    %rax,%rsi
0x19a913a: jmp    19a9151 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1838c1>
0x19a913c: nopl   0x0(%rax)
0x19a9140: addl   $0x1,0x8(%rcx)
0x19a9144: add    $0x10,%rax
0x19a9148: add    $0x10,%rdx
0x19a914c: cmp    %rsi,%rax
0x19a914f: je     19a917f <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1838ef>
0x19a9151: movdqu (%rdx),%xmm3
0x19a9155: mov    0x8(%rdx),%rcx
0x19a9159: movups %xmm3,(%rax)
0x19a915c: test   %rcx,%rcx
0x19a915f: je     19a9144 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1838b4>
0x19a9161: mov    0x1786940(%rip),%rdi        # 312faa8 <__libc_single_threaded@GLIBC_2.32>
0x19a9168: cmpb   $0x0,(%rdi)
0x19a916b: jne    19a9140 <std::__cxx11::basic_string<char, std::char_traits<char>, std::allocator<char> >::compare(char const*) const@@Base+0x1838b0>
0x19a916d: lock addl $0x1,0x8(%rcx)
```
<!-- END GENERATED TRACE 315-next -->
<!-- END GENERATED TRACE EVIDENCE -->
