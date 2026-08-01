# Tibia Linux BattlEye callback addendum

## Scope

This addendum records the successful bounded rerun of the previously queued callback extraction for the official Tibia Linux client 15.30.358f69.

Evidence identity:

- client SHA-256: `8b25d65ece158723dbb50a1b592c1ec8a3247a650fcd2d299bebdfd133cb5752`;
- persistent analysis run: `30246435256-1`;
- successful callback extraction: workflow run `30495579436`, rerun job `90815528899`;
- text-only artifact: `8752558221`, `tibia-linux-protobuf-battleye-bounded-v2`;
- successful exact-string recovery: workflow run `30526055084`, job `90817099201`;
- text-only artifact: `8752725268`, `be-callback-semantics-v3-recovered`;
- no proprietary binary was uploaded or committed.

## Callback table passed to `BEClient.so::Init`

The setup call resolves `Init`, invokes it with interface version `2`, supplies the protected endpoint, and passes the following callback table in this order:

| Table slot | Address | Recovered role | Direct network effect |
|---|---:|---|---|
| 0 | `0x7418f0` | BattlEye message/log sink | none |
| 1 | `0x741de0` | restart-required/status callback | none |
| 2 | `0x741b50` | BattlEye-produced byte-buffer forwarding callback | no direct socket write; emits an application signal carrying copied bytes |

### `0x7418f0` — message/log sink

The callback accepts a null-terminated byte string, converts it to a Qt string and writes it through the Qt logging path with the literal prefix `BattlEye:`.

This callback does not call a socket API and does not mutate game-session transport state. Its role is diagnostic output.

### `0x741de0` — restart-required/status callback

The callback accepts an integer reason/status value. Recovered literals establish two restart conditions:

- `Restarting client is necessary, service isn't running properly`;
- `Restarting client is necessary, update required`.

It records restart-required state in the BattlEye wrapper and emits a Qt signal when the wrapper is active. It does not itself write network bytes.

The exact user-interface reaction after the signal remains outside this bounded extraction, but the callback role is no longer unknown: it requests or records a mandatory client restart because of BattlEye service/update state.

### `0x741b50` — byte-buffer forwarding callback

The callback accepts a data pointer and length, validates the null/length combination, copies the bytes into owned storage and emits Qt signal index `1` with the copied buffer.

It does not directly call `QTcpSocket::write`, `send`, `connectToHost` or another socket primitive. Its immediate effect is an application-level handoff of bytes produced by `BEClient.so`.

This proves that the BattlEye library has a callback path for returning opaque bytes to the Tibia client. The bounded evidence does not prove:

- the outer Tibia protocol message type used downstream;
- whether every protected session sends such bytes;
- whether these bytes are included in the first game-server login request;
- which server-side BattlEye fields are mandatory.

Those items remain `UNKNOWN`; they must not be converted into a bypass claim.

## Revised conclusion for step 1

The three callback roles are now recovered to the application boundary:

1. diagnostic log message;
2. restart-required/status notification;
3. opaque BattlEye byte-buffer handoff to the client through Qt signaling.

Only the third callback can contribute data toward the game transport, and it does so indirectly. Static evidence still does not establish the final wire envelope or server enforcement decision.

## Step 2 safe dry-run

A synthetic localhost-only dry-run was executed outside the repository against a mock endpoint. It exercised this bounded state sequence:

1. local TCP connect;
2. synthetic challenge;
3. synthetic login request;
4. synthetic login success;
5. one harmless turn-in-place action;
6. synthetic state update;
7. disconnect.

The dry-run validates test-control logic and the stop-after-one-action rule only. It is not evidence about CipSoft servers, the official protobuf framing, BattlEye enforcement or protected/unprotected route acceptance.

## Live-test boundary

No official account login, protected-world connection or gameplay action was performed. A decisive live test requires all of the following outside autonomous repository analysis:

- the researcher's own disposable account and character;
- a fresh legitimate loginservice session key;
- explicit acceptance of account-sanction risk;
- no patching, hooking, disabling or impersonation of BattlEye;
- immediate stop at the first decisive rejection or session acceptance;
- no publication of credentials, session data or reusable bypass material.

Because those credentials and live-service conditions were not available to the autonomous task, the official-server acceptance result remains `NOT_RUN` rather than guessed.
