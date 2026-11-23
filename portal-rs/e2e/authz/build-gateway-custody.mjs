import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const source = fileURLToPath(new URL('./gateway-custody.rs', import.meta.url));
const build = mkdtempSync(join(tmpdir(), 'authz-custody-build-'));
const name = 'oteryn-authz-gateway-custody';
try {
  copyFileSync(join(root, 'Cargo.lock'), join(build, 'Cargo.lock'));
  writeFileSync(join(build, 'Cargo.toml'), `
[workspace]

[package]
name = "${name}"
version = "0.1.0"
edition = "2024"
publish = false

[[bin]]
name = "${name}"
path = ${JSON.stringify(source)}

[dependencies]
oteryn-gameauth = { path = ${JSON.stringify(join(root, 'crates/gameauth'))} }
sqlx = { version = "0.9.0", default-features = false, features = ["runtime-tokio", "tls-rustls", "postgres"] }
tokio = { version = "1.54.1", features = ["rt-multi-thread", "macros", "time"] }
`);
  const target = resolve(process.env.CARGO_TARGET_DIR || join(root, 'target'));
  execFileSync('cargo', ['build', '--offline', '--manifest-path', join(build, 'Cargo.toml')], {
    cwd: root,
    env: { ...process.env, CARGO_TARGET_DIR: target, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || '2' },
    stdio: ['ignore', 'ignore', 'inherit'],
  });
  console.log(join(target, 'debug', name));
} finally {
  rmSync(build, { recursive: true });
}
