import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  loadRepositoryInputs,
  validatePortalMediaStateEvidence,
} from './validate-portal-media-state-evidence.mjs';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function loadMediaFragments(repoRoot) {
  const root = path.join(repoRoot, 'docs/testing/portal-media-state-surfaces');
  if (!fs.existsSync(root)) return [];

  return fs.readdirSync(root, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .sort((left, right) => left.name.localeCompare(right.name))
    .flatMap((entry) => {
      const value = readJson(path.join(root, entry.name));
      if (!Array.isArray(value?.surfaces) || value.surfaces.length === 0) {
        throw new Error(`Media state fragment ${entry.name} must define a non-empty surfaces array.`);
      }
      return value.surfaces;
    });
}

function loadStrictInputs(repoRoot = defaultRepoRoot) {
  const inputs = loadRepositoryInputs(repoRoot);
  inputs.contract.surfaces = [
    ...(Array.isArray(inputs.contract.surfaces) ? inputs.contract.surfaces : []),
    ...loadMediaFragments(repoRoot),
  ];
  return inputs;
}

export function validatePortalMediaStrictClosure(inputs = loadStrictInputs()) {
  const report = validatePortalMediaStateEvidence(inputs);
  const errors = [...report.errors];

  if (inputs.contract?.strict_closure !== true) {
    errors.push('portal media evidence strict_closure must be true before Issue #357 can close.');
  }

  if (report.gap_state_count !== 0) {
    errors.push(`strict portal media closure requires zero gap states; found ${report.gap_state_count}.`);
  }

  return {
    ...report,
    strict_closure: inputs.contract?.strict_closure === true,
    errors,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  let report;
  try {
    report = validatePortalMediaStrictClosure();
  } catch (error) {
    report = {
      strict_closure: false,
      schema_version: null,
      portal_surface_count: 0,
      classified_surface_count: 0,
      execution_profile_count: 0,
      media_consumer_count: 0,
      supporting_endpoint_count: 0,
      evidenced_state_count: 0,
      gap_state_count: 0,
      errors: [`Cannot validate strict portal media closure: ${error.message}`],
      warnings: [],
    };
  }

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
