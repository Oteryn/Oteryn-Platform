import { spawn, spawnSync } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

function signalGroup(group, signal) {
  try {
    process.kill(-group, signal);
    return true;
  } catch (error) {
    if (error.code === 'ESRCH') return false;
    if (signal === 0 && error.code === 'EPERM') return true;
    throw error;
  }
}

export class HarnessProcesses {
  #groups = new Set();

  launch(program, args, options) {
    const child = spawn(program, args, { ...options, detached: true });
    if (child.pid !== undefined) {
      this.#groups.add(child.pid);
      child.once('exit', () => {
        if (!signalGroup(child.pid, 0)) this.#groups.delete(child.pid);
      });
    }
    const done = new Promise(resolve => {
      let error;
      child.once('error', failure => { error = failure; });
      child.once('close', (status, signal) => resolve({ status, signal, error }));
    });
    return { child, done };
  }

  #signal(signal) {
    let failure;
    for (const group of this.#groups) {
      try {
        if (!signalGroup(group, signal)) this.#groups.delete(group);
      } catch (error) {
        failure ??= error;
      }
    }
    if (failure) throw failure;
  }

  #hasLiveGroups() {
    this.#signal(0);
    if (this.#groups.size === 0) return false;
    // kill(0) also sees orphan zombies in containers whose PID 1 does not
    // reap them. Use process state to distinguish those from running children.
    const snapshot = spawnSync('ps', ['-e', '-o', 'pgid=,stat='], { encoding: 'utf8', timeout: 1_000 });
    if (snapshot.status !== 0) throw Object.assign(new Error('Could not verify harness process groups'), { cause: snapshot.error });
    return snapshot.stdout.trim().split('\n').some(line => {
      const [group, state] = line.trim().split(/\s+/);
      return this.#groups.has(Number(group)) && !/^[ZX]/.test(state);
    });
  }

  async #waitForExit(timeout) {
    const deadline = performance.now() + timeout;
    while (this.#hasLiveGroups()) {
      const remaining = deadline - performance.now();
      if (remaining <= 0) return false;
      await delay(Math.min(100, remaining));
    }
    return true;
  }

  async stop(log) {
    try {
      this.#signal('SIGINT');
      if (await this.#waitForExit(10_000)) return true;
    } catch (error) {
      log('could not verify graceful harness shutdown', error.code ?? error.message);
    }
    log('forcing shutdown of stalled children');
    try {
      this.#signal('SIGKILL');
      if (await this.#waitForExit(2_000)) return true;
      log('harness process groups still running after SIGKILL');
    } catch (error) {
      log('could not stop and verify harness process groups', error.code ?? error.message);
    }
    return false;
  }
}
