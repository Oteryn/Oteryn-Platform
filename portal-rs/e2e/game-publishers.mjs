// Run game-data.spec.mjs against harness.mjs --game-data. Pass its report.json
// in PORTAL_E2E_GAME_DATA_REPORT. The worker and browser use the same database.
import crypto from 'node:crypto';
import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {expect} from '@playwright/test';
import {localPostgresUrl} from './postgres-url.mjs';

const json = file => JSON.parse(fs.readFileSync(file, 'utf8'));
const fixture = file => json(new URL(`../../docs/contracts/fixtures/${file}`, import.meta.url));
const commandFixture = file => json(new URL(
  `../crates/characters/src/commands/fixtures/${file}.json`, import.meta.url,
));
const seconds = () => Math.floor(Date.now() / 1000);
const commands = '/internal/v1/game-auth/character-commands';
const publicV1 = '/internal/v1/game-public';
const publicV2 = '/internal/v2/game-public';
const epoch = '1';

export class GamePublishers {
  constructor(reportFile) {
    this.report = json(reportFile);
    this.env = json(path.join(this.report.outputDirectory, 'secrets', 'env.json'));
    this.internal = new URL(this.report.baseUrls.internal);
    this.origin = this.env.OTERYN_PUBLIC_ORIGIN;
    const database = localPostgresUrl(this.env.OTERYN_DATABASE_URL);
    if (this.internal.protocol !== 'https:' || this.internal.hostname !== '127.0.0.1' ||
        !['127.0.0.1', 'localhost'].includes(new URL(this.origin).hostname) ||
        this.env.OTERYN_ENV !== 'testing' ||
        !/^\/portal_e2e_[a-f0-9]+$/.test(database.pathname)) {
      throw new Error('Game publishers require a disposable local harness');
    }
    this.env.OTERYN_DATABASE_URL = database.href;
  }

  seed(entity, input) {
    return this.cli(['seed', entity, JSON.stringify(input)], `seed ${entity}`);
  }

  cli(args, label) {
    try {
      return JSON.parse(execFileSync(this.report.binary, args, {
        env: this.env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], timeout: 30_000,
      }));
    } catch {
      // Seed arguments and captured output can contain disposable credentials.
      throw new Error(`Harness CLI failed: ${label}`);
    }
  }

  async post(identity, route, payload) {
    const {cert, key} = this.report.clients[identity];
    const body = JSON.stringify(payload);
    return new Promise((resolve, reject) => {
      const req = https.request(new URL(route, this.internal), {
        method: 'POST',
        ca: fs.readFileSync(this.report.caFile),
        cert: fs.readFileSync(cert),
        key: fs.readFileSync(key),
        servername: this.report.internalServerName,
        minVersion: 'TLSv1.3',
        ALPNProtocols: ['http/1.1'],
        timeout: 10_000,
        headers: {'content-type': 'application/json', 'content-length': Buffer.byteLength(body)},
      }, res => {
        const chunks = [];
        res.on('data', chunk => chunks.push(chunk));
        res.on('error', reject);
        res.on('end', () => {
          if (res.statusCode !== 200) {
            reject(new Error(`${route} returned HTTP ${res.statusCode}`));
            return;
          }
          try {
            resolve(JSON.parse(Buffer.concat(chunks).toString('utf8')));
          } catch {
            reject(new Error(`${route} returned invalid JSON`));
          }
        });
      });
      req.on('timeout', () => req.destroy(new Error(`${route} timed out`)));
      req.on('error', reject);
      req.end(body);
    });
  }

  async completeCreation(operationId, characterId, expectedIntent) {
    const source_authority = this.env.PORTAL_CHARACTER_COMMAND_SOURCE_AUTHORITY;
    const send = (step, payload) => this.post('character-commands', `${commands}/${step}`, {
      ...commandFixture(payload), source_authority, ...(step === 'read' ? {operation_id: operationId} : {}),
    });
    const {pending} = await send('pending', 'pending-request');
    const entry = pending.find(item => item.operation_id === operationId);
    expect(entry).toMatchObject({command: 'CreateCharacter'});
    const intent = await send('read', 'read-request');
    const contract = fixture('character-create-v2/create-intent-knight.json');
    expect(intent).toMatchObject({
      contract_version: contract.contract_version,
      variant: contract.variant,
      command: contract.command,
      audience: contract.audience,
      operation_id: operationId,
      source_revision: entry.source_revision,
      ...expectedIntent,
    });
    expect(Number(intent.expires_at_source)).toBeGreaterThanOrEqual(seconds());
    await this.publish('character-commands', `${commands}/receipt`, {
      ...commandFixture('create-receipt'),
      source_authority,
      operation_id: intent.operation_id,
      character_id: characterId,
      account_id: intent.account_id,
      world_id: intent.target_world_id,
      name: intent.requested_name,
      projection_epoch: epoch,
      projection_revision: '1',
      decided_at: String(seconds()),
    });
    return {characterId, accountId: intent.account_id, worldId: intent.target_world_id, name: intent.requested_name};
  }

  async publish(identity, route, payload) {
    expect(await this.post(identity, route, payload), route).toMatchObject({result: 'accepted'});
  }

  async portfolio({accountId, characterId, worldId, name}) {
    const source_authority = this.env.OTERYN_LCFA_SOURCE_AUTHORITY;
    const observed = String(seconds());
    const route = '/internal/v1/game-auth/native-account-characters';
    await this.publish('lcfa', route, {
      contract_version: 1, operation: 'PublishAccountCharactersV1', source_authority,
      account_id: accountId, projection_epoch: epoch, projection_revision: '1',
      source_observed_at: observed,
      characters: [{character_id: characterId, world_id: worldId, name, availability: 'AVAILABLE'}],
    });
    await this.portfolioWatermark(observed);
  }

  async portfolioWatermark(observed = String(seconds())) {
    if (observed === this.portfolioWatermarkAt) return;
    await this.publish('lcfa', '/internal/v1/game-auth/native-account-characters/watermark', {
      contract_version: 1, operation: 'PublishProjectionWatermarkV1',
      source_authority: this.env.OTERYN_LCFA_SOURCE_AUTHORITY,
      projection_epoch: epoch, complete_through: observed, observed_at: observed,
    });
    this.portfolioWatermarkAt = observed;
  }

  async publicCharacter({characterId, worldId, name}, deathId) {
    const source_authority = this.env.PORTAL_PUBLIC_PROJECTIONS_AUTHORITY;
    const observed = String(seconds());
    const publish = (route, payload) => this.publish('public-projections', route, payload);
    const reference = {name, vocation: 4, level: '412'};
    await publish(`${publicV1}/character-profile`, {
      contract_version: 1, operation: 'PublishCharacterProfileV1', source_authority,
      character_id: characterId, projection_epoch: epoch, projection_revision: '1',
      source_observed_at: observed,
      profile: [{world_id: worldId, ...reference, guild: [], guild_visible: false}],
    });
    await publish(`${publicV1}/world-online`, {
      contract_version: 1, operation: 'PublishWorldOnlineV1', source_authority,
      world_id: worldId, projection_epoch: epoch, projection_revision: String(Date.now()),
      source_observed_at: observed, characters: [reference],
    });
    const deaths = fixture('public-game-families-v2/character_deaths_creature_v2.json');
    await publish(`${publicV2}/character-deaths`, {
      ...deaths, source_authority, character_id: characterId, projection_epoch: epoch,
      projection_revision: '1', source_observed_at: observed,
      deaths: [{...deaths.deaths[0], death_occurrence_id: deathId, occurred_at: observed}],
    });
    for (const family of ['character_profile', 'world_online', 'character_deaths']) {
      const version = family === 'character_deaths' ? 2 : 1;
      await publish(`${version === 2 ? publicV2 : publicV1}/watermark`, {
        contract_version: version, operation: `PublishPublicProjectionWatermarkV${version}`,
        source_authority, family, projection_epoch: epoch, complete_through: observed,
        observed_at: observed,
      });
    }
  }
}

export async function startGameDataBrowser({server, caFile, publicPort, authority}) {
  const smtpCredentials = {
    OTERYN_SMTP_USERNAME: 'game-data',
    OTERYN_SMTP_PASSWORD: crypto.randomBytes(32).toString('hex'),
  };
  // Browser login enqueues a new-device notification. Drain the real mail
  // transport locally so the worker never sends fixture messages off-host.
  const { SMTPServer } = await import('smtp-server');
  const tls = { key: fs.readFileSync(server.key), cert: fs.readFileSync(server.cert) };
  const smtp = new SMTPServer({
    ...tls, secure: true,
    onAuth({ username, password }, _session, done) {
      if (username !== smtpCredentials.OTERYN_SMTP_USERNAME || password !== smtpCredentials.OTERYN_SMTP_PASSWORD) {
        return done(new Error('Invalid local SMTP credentials'));
      }
      done(null, { user: username });
    },
    onData(stream, _session, done) {
      stream.once('end', done);
      stream.resume();
    },
  });
  await new Promise((resolve, reject) => {
    smtp.once('error', reject);
    smtp.listen(0, '127.0.0.1', resolve);
  });
  const browserProxy = https.createServer(tls, (req, res) => {
    const upstream = http.request({
      host: '127.0.0.1', port: publicPort, path: req.url,
      method: req.method, headers: req.headers,
    }, response => {
      res.writeHead(response.statusCode, response.headers);
      response.pipe(res);
    });
    upstream.on('error', () => { res.writeHead(502); res.end(); });
    req.pipe(upstream);
  });
  await new Promise((resolve, reject) => {
    browserProxy.once('error', reject);
    browserProxy.listen(0, '127.0.0.1', resolve);
  });
  const browserHost = 'localhost';
  const origin = `https://${browserHost}:${browserProxy.address().port}`;
  return {
    env: {
      OTERYN_ACCOUNT_LOGIN: 'true',
      OTERYN_SMTP_HOST: '127.0.0.1',
      OTERYN_SMTP_TLS_MODE: 'implicit',
      OTERYN_SMTP_FROM: 'portal@example.invalid',
      OTERYN_MAIL_FROM: 'portal@example.invalid',
      SSL_CERT_FILE: caFile,
      OTERYN_ACCOUNT_OVERVIEW_ENABLED: 'true',
      OTERYN_ACCOUNT_PRIVACY_ENABLED: 'true',
      OTERYN_CHARACTER_CREATE_WEB_ENABLED: 'true',
      OTERYN_CHARACTER_COMMANDS_ENABLED: 'true',
      OTERYN_CHARACTER_COMMAND_SOURCE_AUTHORITY: authority,
      OTERYN_CHARACTER_COMMAND_CREATE_ENABLED: 'true',
      OTERYN_CHARACTER_PROFILES_ENABLED: 'true',
      OTERYN_ACTIVE_CHARACTER_LIMIT: '10',
      OTERYN_TOTAL_CHARACTER_LIMIT: '20',
      PORTAL_PUBLIC_PROFILES_ENABLED: 'true',
      PORTAL_ONLINE_ENABLED: 'true',
      PORTAL_DEATHS_ENABLED: 'true',
      PORTAL_DEATHS_V2_INGEST_ENABLED: 'true',
      ...smtpCredentials,
      OTERYN_SMTP_PORT: String(smtp.server.address().port),
      OTERYN_PUBLIC_ORIGIN: origin,
      OTERYN_WEBAUTHN_RP_ID: browserHost,
      OTERYN_NOTIFY_UNSUBSCRIBE_KEY: crypto.randomBytes(32).toString('hex'),
      OTERYN_NOTIFY_UNSUBSCRIBE_URL: new URL('/en/unsubscribe', origin).href,
    },
    async close() {
      await Promise.all([
        new Promise(resolve => smtp.close(resolve)),
        new Promise((resolve, reject) => {
          browserProxy.close(error => error ? reject(error) : resolve());
          browserProxy.closeAllConnections();
        }),
      ]);
    },
  };
}
