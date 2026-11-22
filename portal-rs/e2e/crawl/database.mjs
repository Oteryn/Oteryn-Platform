import { createHash, createHmac, pbkdf2Sync, randomBytes } from 'node:crypto';

function scramVerifier(password) {
  const salt = randomBytes(16);
  const iterations = 4096;
  const saltedPassword = pbkdf2Sync(password, salt, iterations, 32, 'sha256');
  const clientKey = createHmac('sha256', saltedPassword).update('Client Key').digest();
  const storedKey = createHash('sha256').update(clientKey).digest('base64');
  const serverKey = createHmac('sha256', saltedPassword).update('Server Key').digest('base64');
  return `SCRAM-SHA-256$${iterations}:${salt.toString('base64')}$${storedKey}:${serverKey}`;
}

export function crawlDatabase(name, adminURL) {
  if (!/^portal_e2e_[a-f0-9]{12}$/.test(name)) throw new Error('Invalid crawl database name');
  const identifier = `"${name}"`;
  const password = randomBytes(32).toString('hex');
  const url = new URL(`/${name}`, adminURL);
  url.username = name;
  url.password = password;
  let roleRequested = false;
  return {
    url: url.href,
    ownerClause: ` OWNER ${identifier}`,
    get roleRequested() { return roleRequested; },
    async createRole(admin) {
      roleRequested = true;
      try {
        // PostgreSQL can log DDL statements on error. Send only the verifier.
        await admin.query(`
          CREATE ROLE ${identifier} LOGIN PASSWORD '${scramVerifier(password)}'
            NOSUPERUSER NOCREATEDB NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS
        `);
      } catch (error) {
        if (error.code === '42710') roleRequested = false;
        throw error;
      }
    },
    async dropRole(admin) {
      if (roleRequested) await admin.query(`DROP ROLE IF EXISTS ${identifier}`);
    },
    async restrictAccess(admin) {
      await admin.query(`REVOKE CONNECT, TEMPORARY ON DATABASE ${identifier} FROM PUBLIC`);
    },
  };
}
