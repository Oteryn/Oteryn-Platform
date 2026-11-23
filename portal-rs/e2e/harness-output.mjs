import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

export function prepareHarnessOutput(out) {
  fs.mkdirSync(out, { recursive: true, mode: 0o700 });
  const fd = fs.openSync(out, fs.constants.O_RDONLY | fs.constants.O_DIRECTORY | fs.constants.O_NOFOLLOW);
  const release = () => fs.closeSync(fd);
  try {
    // flock locks the inherited directory descriptor and Node retains it until cleanup.
    const lock = spawnSync('flock', ['-n', '-E', '75', '3'], { stdio: ['ignore', 'ignore', 'ignore', fd] });
    if (lock.status === 75) throw new Error(`${out} is in use by another harness run`);
    if (lock.status !== 0) throw new Error('Could not lock the harness output directory (requires flock)');
    const locked = fs.fstatSync(fd, { bigint: true });
    const current = fs.lstatSync(out, { bigint: true, throwIfNoEntry: false });
    if (current?.ino !== locked.ino || current.dev !== locked.dev) {
      throw new Error(`${out} was replaced during lock acquisition`);
    }

    const marker = path.join(out, '.portal-e2e-harness');
    const marked = fs.lstatSync(marker, { throwIfNoEntry: false });
    if (!marked && fs.readdirSync(out).length === 0) {
      fs.writeFileSync(marker, '', { flag: 'wx', mode: 0o600 });
    } else if (!marked?.isFile()) {
      throw new Error(`${out} exists and is not a harness directory`);
    }
    const dirs = {
      certs: path.join(out, 'certs'), clients: path.join(out, 'certs', 'clients'),
      witness: path.join(out, 'witness'), logs: path.join(out, 'logs'),
      secrets: path.join(out, 'secrets'), bin: path.join(out, 'bin'),
    };
    const reportFile = path.join(out, 'report.json');
    for (const file of [dirs.certs, dirs.witness, dirs.logs, dirs.secrets, dirs.bin, reportFile]) {
      fs.rmSync(file, { recursive: true, force: true });
    }
    return { dirs, marker, reportFile, release };
  } catch (error) {
    release();
    throw error;
  }
}
