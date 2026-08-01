# Tibia Linux reference harness

This directory contains the bounded local harness for task
`OTERYN-20260801-official-linux-client-live-reference`. It launches either a deterministic fake
client or an exact hash-approved official Linux executable. It never accepts credentials on the
command line or through ordinary environment variables.

## Safety boundary

- Run as a dedicated non-root Linux x86-64 user in an interactive X11/WSLg session.
- Keep the official package, executable, raw evidence and temporary profile outside Git on a
  private encrypted volume. Official mode refuses storage whose encryption cannot be proven.
- The fake-client dry run uses a separate network namespace containing only loopback. It attempts
  one reserved `TEST-NET-2` connection and must observe denial.
- Official component mode also denies all outbound networking and forbids authentication. Its only
  purpose is to verify the exact executable can create a window without modification.
- Never set `LD_PRELOAD` or `LD_AUDIT`; never use ptrace, a debugger, injection, hooking, traffic
  alteration, replay or packet decryption.
- Stop on any BattlEye, anti-cheat, client-modification or account-security warning.
- Do not paste credentials into Codex, GitHub, Git, a shell command, workflow input or file. The
  later owner-gated live phase uses manual entry directly in the official client.

The harness retains only redacted JSON. It does not retain raw stdout/stderr, environment values,
process arguments, screenshots or packet captures.

## Required host controls

- Linux x86-64 with Python 3.12 or newer, `git`, `readelf`, `sha256sum`, `findmnt`, `unshare` and
  `libX11.so.6`;
- a dedicated non-privileged user and graphical session;
- unprivileged user/network namespaces, or passwordless `sudo` limited to creating a network
  namespace and dropping back to the calling UID/GID;
- a mode `0700` evidence directory outside the checkout;
- a local `origin/main` ref so the scanner can inspect the complete Git-visible branch diff;
- for official mode, an encrypted filesystem detectable as a crypt-backed device mapper or an
  encrypted filesystem type.

WSL2 may validate the synthetic path when WSLg and unprivileged namespaces work. WSL2 does not
prove that BattlEye accepts virtualization, and its guest cannot prove host BitLocker state. Use a
normal dedicated interactive Linux host if the exact client or BattlEye refuses the VM boundary.

## Synthetic dry run

Disable tracing before invoking the harness:

```bash
set +x
unset LD_PRELOAD LD_AUDIT
evidence_root="$(mktemp -d /tmp/oteryn-tibia-reference.XXXXXX)"
chmod 700 "$evidence_root"
python3 tools/tibia-linux-reference/run.py dry-run \
  --repo-root "$PWD" \
  --evidence-dir "$evidence_root"
```

The command generates four unique synthetic values in memory, transfers them to the fake client
through an anonymous pipe, launches the graphical fake client in a no-network namespace, scans all
prohibited locations and deletes the temporary profile. The retained output is limited to
`<evidence-root>/<session>/publishable/session-manifest.json` and `cleanup-report.json`.

## Exact identity approval

Copy `identity.template.json` to the private evidence volume and replace every placeholder from a
lawfully obtained package. Do not commit the populated file. The previously analyzed executable
was version `15.30.358f69`, SHA-256
`8b25d65ece158723dbb50a1b592c1ec8a3247a650fcd2d299bebdfd133cb5752`, with ELF Build ID
`a7256985ece88dc38f45b9248c6119c22359ae6a`. The package/archive SHA-256 was not preserved in the
repository and remains `UNKNOWN`; obtain and approve it independently.

Verify without executing:

```bash
python3 tools/tibia-linux-reference/run.py verify-identity \
  --repo-root "$PWD" \
  --identity /private/identity.json \
  --package /private/Tibia-package.tar.gz \
  --executable /private/Tibia/bin/client
```

The verifier hashes both files and reads the ELF Build ID. It records only hashes, version, source
classification, timestamp and private-path classification.

## Official component launch (no authentication)

Run only after synthetic dry-run success and encryption proof:

```bash
python3 tools/tibia-linux-reference/run.py official-component \
  --repo-root "$PWD" \
  --evidence-dir /encrypted/private/tibia-reference \
  --identity /encrypted/private/identity.json \
  --package /encrypted/private/Tibia-package.tar.gz \
  --executable /encrypted/private/Tibia/bin/client \
  --observation-seconds 5
```

The executable is not patched or wrapped with injected code. The OS network namespace denies all
outbound traffic, the launcher observes only X11 window IDs and process lifecycle, and the process
group and temporary profile are removed afterward. Do not enter any account data.

## Validation

```bash
python3 -m compileall -q tools/tibia-linux-reference
PYTHONPATH=tools/tibia-linux-reference \
  python3 -m unittest discover -s tools/tibia-linux-reference/tests -v
python3 tools/tibia-linux-reference/run.py validate-manifest \
  tools/tibia-linux-reference/examples/session-manifest.synthetic.json
```

Review retained publishable JSON before attaching it anywhere. Raw evidence, populated identity
files and official binaries are never GitHub artifacts.
