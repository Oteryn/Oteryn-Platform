# Tibia Linux reference harness

This directory contains the bounded Linux research harness for task
`OTERYN-20260810-tibia-linux-reference-harness`.

The current task authorizes **synthetic, no-network validation only**. The code preserves a
fail-closed identity/component foundation for a separately owner-gated future phase, but this task
does not authorize an official Tibia login, official-service contact, real credentials, or an
official-client/BattlEye execution.

## Safety boundary

- Run as a dedicated non-root Linux x86-64 user in an interactive X11 session.
- Keep any future official package, populated identity manifest, raw evidence and temporary profile
  outside Git on a private encrypted volume.
- The fake-client dry run enters a distinct Linux network namespace. Success requires that the
  namespace exposes only `lo`, that IPv4 and IPv6 documentation-prefix connections fail, and that
  a `.invalid` DNS lookup cannot resolve.
- Synthetic credentials are generated per run, transferred through an anonymous pipe, zeroed in
  the encoded parent buffer after transfer, and must be absent from Git-visible state, process
  arguments, retained environment reports, stdout/stderr, evidence, temporary files and shell
  history.
- Never set `LD_PRELOAD` or `LD_AUDIT`; never use ptrace, debugger attachment, hooking, injection,
  binary patching, packet decryption, traffic alteration/replay/injection or anti-cheat bypass.
- Do not paste credentials into ChatGPT/Codex, GitHub, Git, workflow inputs, shell arguments,
  ordinary environment variables, repository files, logs or screenshots.
- GitHub Actions runs only the deterministic fake client. It never downloads or invokes an
  official package and never uploads runtime artifacts.

The threat claim is intentionally bounded: a passing dry run demonstrates that the generated
synthetic corpus is absent from the enumerated tested surfaces under the documented execution
model. It is not a proof against an arbitrary compromised kernel, administrator, hypervisor,
hardware keylogger, or other host-level adversary.

## CI environment isolation

GitHub Actions contains platform runtime variables that may include credentials. The workflow
therefore launches every harness/test Python process with `env -i` and an explicit allowlist. The
security preflight remains fail-closed: it still rejects token/secret-like ordinary environment
variables rather than adding CI exceptions.

Xvfb is started with TCP listening disabled. The component step uses
`DISPLAY=unix/:99`, forcing X11 through `/tmp/.X11-unix/X99`; this filesystem socket remains
reachable after the fake client enters its isolated network namespace. Core dumps are disabled and
the workflow uses `umask 077`.

## Required local controls

- Linux x86-64 with Python 3.12 or newer, `git`, `readelf`, `sha256sum`, `findmnt`, `lsblk`,
  `unshare` and `libX11.so.6`;
- a dedicated non-privileged user and graphical session;
- unprivileged user/network namespaces, or passwordless `sudo` limited to creating a network
  namespace and dropping back to the calling UID/GID;
- a mode `0700` evidence directory outside the checkout;
- a local `origin/main` ref so the scanner can inspect the complete Git-visible branch diff.

For any future official component gate, additionally require a private evidence filesystem whose
encryption is proven by preflight and an owner-approved identity file containing the exact package
SHA-256, executable SHA-256, ELF Build ID, client version and package source. Device-mapper naming
alone is not encryption evidence: a block-backed volume must report `TYPE=crypt` through `lsblk`,
or use one of the explicitly recognized encrypted filesystem types. Historical hashes from PR #391
are reference evidence only and are not an approval for a current package.

## Synthetic dry run

Disable tracing and use a clean process environment. Example:

```bash
set +x
umask 077
ulimit -c 0
evidence_root="$(mktemp -d /tmp/oteryn-tibia-reference.XXXXXX)"
safe_home="$(mktemp -d /tmp/oteryn-tibia-home.XXXXXX)"
chmod 700 "$evidence_root" "$safe_home"

env -i HOME="$safe_home" PATH="$PATH" DISPLAY="unix/:99" \
  LANG="C.UTF-8" LC_ALL="C.UTF-8" \
  PYTHONPATH="$PWD/tools/tibia-linux-reference" \
  python3 tools/tibia-linux-reference/run.py dry-run \
    --repo-root "$PWD" \
    --evidence-dir "$evidence_root"
```

The command generates four unique synthetic values in memory, transfers them through an anonymous
pipe, launches the graphical fake client under OS network denial, scans prohibited locations and
deletes the temporary profile. Retained runtime output is limited to redacted JSON under
`<evidence-root>/<session>/publishable/`.

## Identity verification foundation

`identity.template.json` contains placeholders only. A populated identity file must remain on the
future private evidence volume and must never be committed.

Verification is non-executing:

```bash
python3 tools/tibia-linux-reference/run.py verify-identity \
  --repo-root "$PWD" \
  --identity /private/identity.json \
  --package /private/client-package \
  --executable /private/client
```

The verifier compares exact package and executable hashes and ELF Build ID. A populated manifest
must come from an independently owner-approved package; self-hashing an unapproved package does not
make it approved.

## Preserved future component command

The historical harness also contains `official-component`, which launches only an exact
identity-approved executable under outbound network denial and forbids authentication. **Do not run
that command under task `OTERYN-20260810-tibia-linux-reference-harness`.** Its use requires a
separate owner-gated task after a compatible interactive Linux host, encrypted private evidence
volume and exact package identity have all been proven.

A BattlEye/environment refusal is terminal evidence for that host and must never be worked around.

## Validation

```bash
python3 -m compileall -q tools/tibia-linux-reference
PYTHONPATH=tools/tibia-linux-reference \
  python3 -m unittest discover -s tools/tibia-linux-reference/tests -v
python3 tools/tibia-linux-reference/run.py validate-manifest \
  tools/tibia-linux-reference/examples/session-manifest.synthetic.json
```

The dedicated GitHub workflow additionally runs the graphical fake-client component in a fresh
network namespace and verifies that no raw/proprietary artifact is staged.
