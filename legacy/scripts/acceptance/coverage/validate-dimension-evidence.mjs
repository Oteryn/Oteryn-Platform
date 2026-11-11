import { loadBaseline, validatePortalEvidenceDimensions } from './validate-portal-evidence-dimensions.mjs';
import { validateCriticalViewportEvidence } from './validate-critical-viewport-evidence.mjs';

const baseline = loadBaseline();
const report = validatePortalEvidenceDimensions(baseline);
const critical = validateCriticalViewportEvidence(baseline);
report.critical_viewports = critical;
report.errors.push(...critical.errors);
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (report.errors.length > 0) process.exitCode = 1;
