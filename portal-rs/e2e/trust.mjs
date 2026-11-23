import { execFileSync } from 'node:child_process';
import { randomUUID, X509Certificate } from 'node:crypto';
import { mkdirSync, readFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { getCACertificates, setDefaultCACertificates } from 'node:tls';

// Chromium 145 reads the Linux NSS user database. Newer Chromium also
// recognizes this location when it exists. Import only the disposable CA.
export function trustHarnessCa(file) {
  const pem = readFileSync(file, 'utf8');
  if (!new X509Certificate(pem).ca) throw new Error('Expected a harness CA certificate');
  const directory = join(homedir(), '.pki', 'nssdb');
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  const database = `sql:${directory}`;
  const label = `Oteryn E2E ${randomUUID()}`;
  const roots = getCACertificates('default');
  const certutil = args => execFileSync('certutil', [...args, '-d', database], { stdio: 'ignore' });
  certutil(['-A', '-n', label, '-t', 'C,,', '-i', file]);
  setDefaultCACertificates([...roots, pem]);
  return () => {
    setDefaultCACertificates(roots);
    certutil(['-D', '-n', label]);
  };
}
