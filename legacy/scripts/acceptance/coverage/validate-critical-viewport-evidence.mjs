import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const coverageRoot = path.dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = path.resolve(coverageRoot, '../../..');
const requiredCriticalViewports = ['desktop-1440x1000', 'tablet-820x1180', 'mobile-390x844'];

function evidenceContainsAllMarkers(repoRoot, evidence) {
  if (!evidence || !Array.isArray(evidence.markers) || evidence.markers.length === 0) return false;
  const evidencePath = path.resolve(repoRoot, evidence.file ?? '');
  if (!evidencePath.startsWith(`${repoRoot}${path.sep}`) || !fs.existsSync(evidencePath)) return false;
  const source = fs.readFileSync(evidencePath, 'utf8');
  return evidence.markers.every((marker) => source.includes(marker));
}

export function validateCriticalViewportEvidence({ contract, repoRoot = defaultRepoRoot }) {
  const errors = [];
  const profiles = new Map((contract.profiles ?? []).map((profile) => [profile.id, profile]));
  const critical = (contract.surfaces ?? []).filter((record) => record.kind === 'rendered' && record.critical === true);
  let derivedMappings = 0;

  for (const record of critical) {
    const evidenceById = new Map((record.evidence ?? []).map((evidence) => [evidence.id, evidence]));

    for (const viewport of requiredCriticalViewports) {
      const explicit = record.viewports?.[viewport];
      if (explicit) {
        const profile = profiles.get(explicit.profile);
        const project = profile?.projects?.find((candidate) => candidate.name === explicit.project);
        const evidence = evidenceById.get(explicit.evidence);
        const blockingChromium = profile?.blocking === true && project?.browser === 'chromium';

        if (explicit.mode === 'test-controlled') {
          const hasViewportMarker = Array.isArray(evidence?.markers)
            && evidence.markers.length >= 2
            && evidenceContainsAllMarkers(repoRoot, evidence);
          if (!blockingChromium || !hasViewportMarker) {
            errors.push(`${record.id} critical viewport ${viewport} is not tied to blocking zero-retry Chromium test-controlled evidence.`);
          }
        } else if (!blockingChromium || project?.viewport !== viewport) {
          errors.push(`${record.id} critical viewport ${viewport} is not tied to a blocking zero-retry Chromium project.`);
        }
        continue;
      }

      let proved = false;
      for (const mapping of Object.values(record.viewports ?? {})) {
        const profile = profiles.get(mapping.profile);
        const project = profile?.projects?.find((candidate) => candidate.browser === 'chromium' && candidate.viewport === viewport);
        const evidence = evidenceById.get(mapping.evidence);
        if (profile?.blocking !== true || !project || !evidenceContainsAllMarkers(repoRoot, evidence)) continue;

        const configPath = path.resolve(repoRoot, profile.config_file ?? '');
        if (!configPath.startsWith(`${repoRoot}${path.sep}`) || !fs.existsSync(configPath)) continue;
        const configSource = fs.readFileSync(configPath, 'utf8');
        const specBasename = path.basename(evidence.file ?? '');
        if (specBasename === '' || !configSource.includes(specBasename)) continue;

        proved = true;
        derivedMappings += 1;
        break;
      }

      if (!proved) {
        errors.push(`${record.id} critical viewport ${viewport} has neither an explicit mapping nor a blocking project-selected evidence mapping.`);
      }
    }
  }

  return {
    critical_surface_count: critical.length,
    required_viewports: requiredCriticalViewports,
    derived_mapping_count: derivedMappings,
    errors,
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const { loadBaseline } = await import('./validate-portal-evidence-dimensions.mjs');
  const report = validateCriticalViewportEvidence(loadBaseline());
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (report.errors.length > 0) process.exitCode = 1;
}
