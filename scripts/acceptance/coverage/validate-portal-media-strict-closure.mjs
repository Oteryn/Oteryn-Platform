import path from 'node:path';
import { pathToFileURL } from 'node:url';
import {
  loadRepositoryInputs,
  validatePortalMediaStateEvidence,
} from './validate-portal-media-state-evidence.mjs';

export function validatePortalMediaStrictClosure(inputs = loadRepositoryInputs()) {
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
