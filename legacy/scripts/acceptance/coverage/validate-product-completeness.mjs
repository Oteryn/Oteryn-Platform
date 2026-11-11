import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');
const ledgerPath = path.join(repoRoot, 'docs/testing/product-completeness-benchmark.json');

const expectedCapabilityIds = [
  'account.password-recovery',
  'account.high-assurance-recovery-artifact',
  'account.email-change',
  'account.session-inventory-revocation',
  'account.connected-game-account-binding',
  'account.mfa-method-management',
  'account.badges-loyalty-status',
  'account.privacy-controls',
  'account.termination',
  'character.public-information',
  'character.privacy',
  'character.achievement-display',
  'character.public-profile-completeness',
  'character.deletion-restore',
  'character.rename',
  'character.world-transfer',
  'character.main-selection',
  'character.guild-house-account-linkage',
  'commerce.premium-vip',
  'commerce.coin-purchase-balance-delivery',
  'commerce.game-code-redemption',
  'commerce.products-ready-to-use',
  'commerce.customer-histories',
  'commerce.provider-webhook-refunds-chargebacks',
  'support.tickets',
  'support.report-history-limits',
  'support.enforcement-history',
  'support.notifications',
  'public.character-search-detail',
  'public.guild-detail-administration',
  'public.highscores',
  'public.deaths-transfers-kills',
  'public.polls',
  'public.public-enforcement',
  'public.character-bazaar',
  'public.online-server-status',
  'knowledge.creatures-items',
  'knowledge.spells-quests-npcs-achievements',
  'knowledge.maps',
  'knowledge.hunting-calculators',
  'knowledge.battle-pass-presets-huntfinder',
  'knowledge.world-transfer-docs',
  'knowledge.server-specific-systems-events',
];

const errors = [];

function readJson(file) {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch (error) {
    errors.push(`Cannot parse ${path.relative(repoRoot, file)}: ${error.message}`);
    return null;
  }
}

function requireNonEmptyString(value, label) {
  if (typeof value !== 'string' || value.trim() === '') {
    errors.push(`${label} must be a non-empty string.`);
    return false;
  }
  return true;
}

function requireStringArray(value, label, { allowEmpty = false } = {}) {
  if (!Array.isArray(value) || (!allowEmpty && value.length === 0) || value.some((item) => typeof item !== 'string' || item.trim() === '')) {
    errors.push(`${label} must be ${allowEmpty ? 'a' : 'a non-empty'} string array.`);
    return [];
  }
  return value;
}

const ledger = readJson(ledgerPath);
if (!ledger) {
  process.exitCode = 1;
} else {
  if (ledger.schema_version !== 1) errors.push('product completeness schema_version must be 1.');
  if (ledger.audit_issue !== 268) errors.push('audit_issue must be 268.');
  requireNonEmptyString(ledger.baseline_sha, 'baseline_sha');
  requireNonEmptyString(ledger.generated_at, 'generated_at');
  requireNonEmptyString(ledger.claim_boundary, 'claim_boundary');

  const allowedStatuses = new Set(requireStringArray(ledger.allowed_delivery_statuses, 'allowed_delivery_statuses'));
  const allowedRelevance = new Set(requireStringArray(ledger.allowed_relevance, 'allowed_relevance'));
  const allowedEvidenceLevels = new Set(requireStringArray(ledger.allowed_evidence_levels, 'allowed_evidence_levels'));
  const capabilities = Array.isArray(ledger.capabilities) ? ledger.capabilities : [];
  if (capabilities.length === 0) errors.push('capabilities must be a non-empty array.');

  const ids = new Set();
  const expected = new Set(expectedCapabilityIds);
  const domainCounts = new Map();

  for (const capability of capabilities) {
    const id = capability?.id;
    if (!requireNonEmptyString(id, 'capability.id')) continue;
    if (ids.has(id)) errors.push(`Duplicate capability id: ${id}`);
    ids.add(id);

    if (!expected.has(id)) errors.push(`Unexpected capability id not declared by the Issue #268 contract: ${id}`);
    if (!requireNonEmptyString(capability.domain, `${id}.domain`)) continue;
    domainCounts.set(capability.domain, (domainCounts.get(capability.domain) ?? 0) + 1);

    requireNonEmptyString(capability.capability, `${id}.capability`);
    requireNonEmptyString(capability.rationale, `${id}.rationale`);

    if (!allowedStatuses.has(capability.delivery_status)) {
      errors.push(`${id} has unsupported delivery_status ${JSON.stringify(capability.delivery_status)}.`);
    }
    if (!allowedRelevance.has(capability.relevance)) {
      errors.push(`${id} has unsupported relevance ${JSON.stringify(capability.relevance)}.`);
    }

    const benchmarkSources = requireStringArray(capability.benchmark_sources, `${id}.benchmark_sources`);
    for (const source of benchmarkSources) {
      if (!/^https:\/\//u.test(source)) errors.push(`${id} benchmark source must be HTTPS: ${source}`);
    }

    const gapIssues = requireStringArray(capability.gap_issues, `${id}.gap_issues`, { allowEmpty: true });
    for (const issue of gapIssues) {
      if (!/^#\d+$/u.test(issue)) errors.push(`${id} gap issue must use #<number>: ${issue}`);
    }

    const evidence = Array.isArray(capability.oteryn_evidence) ? capability.oteryn_evidence : [];
    if (!Array.isArray(capability.oteryn_evidence)) errors.push(`${id}.oteryn_evidence must be an array.`);
    if (['implemented', 'partial'].includes(capability.delivery_status) && evidence.length === 0) {
      errors.push(`${id} is ${capability.delivery_status} but has no Oteryn evidence.`);
    }

    for (const [index, item] of evidence.entries()) {
      if (!requireNonEmptyString(item?.file, `${id}.oteryn_evidence[${index}].file`)) continue;
      if (!allowedEvidenceLevels.has(item.level)) {
        errors.push(`${id}.oteryn_evidence[${index}] has unsupported level ${JSON.stringify(item.level)}.`);
      }
      const absolute = path.resolve(repoRoot, item.file);
      if (!absolute.startsWith(`${repoRoot}${path.sep}`)) {
        errors.push(`${id} evidence escapes repository root: ${item.file}`);
        continue;
      }
      if (!fs.existsSync(absolute) || !fs.statSync(absolute).isFile()) {
        errors.push(`${id} references missing evidence file ${item.file}.`);
        continue;
      }
      if (item.marker !== undefined) {
        if (!requireNonEmptyString(item.marker, `${id}.oteryn_evidence[${index}].marker`)) continue;
        if (!fs.readFileSync(absolute, 'utf8').includes(item.marker)) {
          errors.push(`${id} evidence marker not found in ${item.file}: ${item.marker}`);
        }
      }
    }

    if (capability.relevance === 'required' && ['partial', 'missing', 'untested'].includes(capability.delivery_status) && gapIssues.length === 0) {
      errors.push(`${id} is a required ${capability.delivery_status} capability without a focused gap issue.`);
    }
    if (capability.delivery_status === 'implemented' && gapIssues.length > 0) {
      errors.push(`${id} is implemented but still declares gap issues.`);
    }
    if (capability.relevance === 'not_applicable' && capability.delivery_status !== 'not_applicable') {
      errors.push(`${id} is not_applicable in relevance but not in delivery status.`);
    }
  }

  for (const id of expectedCapabilityIds) {
    if (!ids.has(id)) errors.push(`Missing Issue #268 benchmark capability: ${id}`);
  }
  for (const domain of ['account', 'character', 'commerce', 'support', 'public', 'knowledge']) {
    if (!domainCounts.has(domain)) errors.push(`Benchmark domain has no capabilities: ${domain}`);
  }

  const summary = capabilities.reduce((counts, capability) => {
    counts[capability.delivery_status] = (counts[capability.delivery_status] ?? 0) + 1;
    return counts;
  }, {});

  process.stdout.write(`${JSON.stringify({
    schema_version: ledger.schema_version,
    audit_issue: ledger.audit_issue,
    capability_count: capabilities.length,
    delivery_status_counts: summary,
    domain_counts: Object.fromEntries([...domainCounts.entries()].sort(([left], [right]) => left.localeCompare(right))),
    errors,
  }, null, 2)}\n`);

  if (errors.length > 0) process.exitCode = 1;
}
