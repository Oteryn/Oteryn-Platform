import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(coverageRoot, '../../..');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function readSurfaceFragments(root) {
  if (!fs.existsSync(root)) return [];

  return fs.readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const value = readJson(path.join(root, entry.name));
      return Array.isArray(value) ? value : (Array.isArray(value.surfaces) ? value.surfaces : []);
    });
}

function nonEmptyString(value) {
  return typeof value === 'string' && value.trim() !== '';
}

function stringArray(value) {
  return Array.isArray(value) && value.every((item) => nonEmptyString(item));
}

function browserEvidenceCount(surface) {
  if (!surface || !Array.isArray(surface.evidence_layers) || !surface.evidence_layers.includes('playwright')) return 0;

  return (Array.isArray(surface.evidence) ? surface.evidence : []).reduce((count, evidence) => {
    if (!nonEmptyString(evidence?.file)) return count;
    const markers = Array.isArray(evidence.markers) ? evidence.markers.filter(nonEmptyString) : [];
    return count + markers.length;
  }, 0);
}

export function validateBackendFrontendCompleteness({ productLedger, frontendLedger, surfaces }) {
  const errors = [];
  const allowedLayerStatuses = new Set(frontendLedger?.allowed_layer_statuses ?? []);
  const productCapabilities = Array.isArray(productLedger?.capabilities) ? productLedger.capabilities : [];
  const frontendCapabilities = Array.isArray(frontendLedger?.capabilities) ? frontendLedger.capabilities : [];
  const productById = new Map();
  const frontendById = new Map();
  const surfaceById = new Map();

  if (frontendLedger?.schema_version !== 1) errors.push('backend/frontend completeness schema_version must be 1.');
  if (frontendLedger?.parent_issue !== 326) errors.push('backend/frontend completeness parent_issue must be 326.');
  if (!nonEmptyString(frontendLedger?.claim_boundary)) errors.push('backend/frontend completeness claim_boundary must be a non-empty string.');
  if (allowedLayerStatuses.size === 0) errors.push('allowed_layer_statuses must be a non-empty array.');

  for (const capability of productCapabilities) {
    if (!nonEmptyString(capability?.id)) continue;
    if (productById.has(capability.id)) errors.push(`Duplicate product capability id: ${capability.id}`);
    productById.set(capability.id, capability);
  }

  for (const surface of Array.isArray(surfaces) ? surfaces : []) {
    if (!nonEmptyString(surface?.id)) continue;
    if (surfaceById.has(surface.id)) errors.push(`Duplicate portal surface id: ${surface.id}`);
    surfaceById.set(surface.id, surface);
  }

  for (const record of frontendCapabilities) {
    const id = record?.id;
    if (!nonEmptyString(id)) {
      errors.push('Every backend/frontend capability record must have a non-empty id.');
      continue;
    }
    if (frontendById.has(id)) errors.push(`Duplicate backend/frontend capability id: ${id}`);
    frontendById.set(id, record);

    const product = productById.get(id);
    if (!product) {
      errors.push(`Backend/frontend capability is not present in the canonical product ledger: ${id}`);
      continue;
    }

    if (typeof record.user_facing !== 'boolean') errors.push(`${id}.user_facing must be boolean.`);
    for (const field of ['backend_status', 'frontend_status', 'integration_status']) {
      if (!allowedLayerStatuses.has(record[field])) errors.push(`${id}.${field} has unsupported status ${JSON.stringify(record[field])}.`);
    }
    if (!stringArray(record.surface_ids)) errors.push(`${id}.surface_ids must be a string array.`);

    const surfaceIds = Array.isArray(record.surface_ids) ? record.surface_ids : [];
    if (new Set(surfaceIds).size !== surfaceIds.length) errors.push(`${id}.surface_ids contains duplicates.`);

    const requiresException = record.user_facing === false || record.frontend_status === 'not_applicable' || product.delivery_status === 'not_applicable';
    if (requiresException && !nonEmptyString(record.exception_reason)) errors.push(`${id} requires a bounded exception_reason.`);

    if (product.delivery_status === 'implemented') {
      if (record.user_facing === true) {
        for (const field of ['backend_status', 'frontend_status', 'integration_status']) {
          if (record[field] !== 'implemented') errors.push(`${id} is product implemented but ${field} is ${record[field]}.`);
        }
      } else {
        if (record.backend_status !== 'implemented') errors.push(`${id} non-UI implemented capability must have implemented backend_status.`);
        if (record.frontend_status !== 'not_applicable') errors.push(`${id} non-UI implemented capability must have not_applicable frontend_status.`);
        if (record.integration_status !== 'implemented') errors.push(`${id} non-UI implemented capability must have implemented integration_status.`);
      }
    }

    if (product.delivery_status === 'partial') {
      const layers = [record.backend_status, record.frontend_status, record.integration_status];
      if (layers.every((status) => status === 'implemented')) errors.push(`${id} is product partial but all backend/frontend layers are implemented.`);
      if (!layers.includes('partial')) errors.push(`${id} is product partial but no backend/frontend layer is partial.`);
    }

    if (product.delivery_status === 'missing') {
      for (const field of ['backend_status', 'integration_status']) {
        if (record[field] === 'implemented') errors.push(`${id} is product missing but ${field} is implemented.`);
      }
      if (record.user_facing === true && record.frontend_status !== 'missing') {
        errors.push(`${id} is a missing user-facing capability but frontend_status is ${record.frontend_status}.`);
      }
    }

    if (product.delivery_status === 'not_applicable') {
      for (const field of ['backend_status', 'frontend_status', 'integration_status']) {
        if (record[field] !== 'not_applicable') errors.push(`${id} is product not_applicable but ${field} is ${record[field]}.`);
      }
    }

    const needsIntegratedSurface = record.integration_status === 'implemented' || record.integration_status === 'partial';
    if (needsIntegratedSurface && surfaceIds.length === 0) errors.push(`${id} has ${record.integration_status} integration but no portal surface_ids.`);
    if (!needsIntegratedSurface && surfaceIds.length > 0) errors.push(`${id} has no integrated frontend but still references portal surfaces.`);

    let browserMarkers = 0;
    for (const surfaceId of surfaceIds) {
      const surface = surfaceById.get(surfaceId);
      if (!surface) {
        errors.push(`${id} references unknown portal surface ${surfaceId}.`);
        continue;
      }
      if (surface.status !== 'covered') errors.push(`${id} references portal surface ${surfaceId} with status ${surface.status}.`);
      browserMarkers += browserEvidenceCount(surface);
    }

    if (record.user_facing === true && record.integration_status === 'implemented' && browserMarkers === 0) {
      errors.push(`${id} is integrated and user-facing but its referenced surfaces have no stable Playwright evidence markers.`);
    }
  }

  for (const id of productById.keys()) {
    if (!frontendById.has(id)) errors.push(`Canonical product capability has no backend/frontend record: ${id}`);
  }
  for (const id of frontendById.keys()) {
    if (!productById.has(id)) errors.push(`Unexpected backend/frontend capability id: ${id}`);
  }

  return {
    schema_version: frontendLedger?.schema_version ?? null,
    product_capability_count: productById.size,
    backend_frontend_capability_count: frontendById.size,
    portal_surface_count: surfaceById.size,
    errors,
  };
}

export function loadRepositoryInputs(root = repoRoot) {
  const productLedger = readJson(path.join(root, 'docs/testing/product-completeness-benchmark.json'));
  const frontendLedger = readJson(path.join(root, 'docs/testing/product-backend-frontend-completeness.json'));
  const portalManifest = readJson(path.join(root, 'scripts/acceptance/coverage/portal-coverage-manifest.json'));
  const fragments = readSurfaceFragments(path.join(root, 'scripts/acceptance/coverage/surfaces'));

  return {
    productLedger,
    frontendLedger,
    surfaces: [...(Array.isArray(portalManifest.surfaces) ? portalManifest.surfaces : []), ...fragments],
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try {
    report = validateBackendFrontendCompleteness(loadRepositoryInputs());
  } catch (error) {
    report = {
      schema_version: null,
      product_capability_count: 0,
      backend_frontend_capability_count: 0,
      portal_surface_count: 0,
      errors: [`Cannot validate backend/frontend completeness: ${error.message}`],
    };
  }

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
