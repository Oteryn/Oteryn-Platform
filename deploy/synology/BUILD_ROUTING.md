# Synology staging build routing

`Build Synology Staging Images` uses component-aware input classification. The workflow no longer treats the entire `deploy/synology/**` subtree as one runtime-image invalidation domain.

## Image build allocation

- Platform is rebuilt only when a truthful Platform image input changes.
- Game Gateway is rebuilt only when its bounded Synology image inputs change. Its service source boundary is `services/game-gateway/go.mod`, `services/game-gateway/cmd/**`, and `services/game-gateway/internal/**`, plus the Synology Gateway Dockerfile/context controls.
- The privileged deploy-runner is never published by an ordinary protected-main push. It remains a manual/control-plane image path.
- Classifier or image-build control-plane changes fail closed and qualify every relevant image path rather than silently reusing an unproven component.

## Deployment-package-only changes

Runtime deployment-package inputs such as the staging Compose manifest, runtime environment contract, MariaDB initialization, internal Nginx/TLS bootstrap, and deployment/health/recovery scripts can require protected-main staging reconciliation without creating a new Platform or Gateway image identity.

When neither runtime component changed, protected main allocates zero runtime image build jobs and dispatches staging with both changed flags false. `Deploy Synology Staging` then accepts reuse only from persisted current-release component provenance after validating release lineage, immutable digest form, and each image's OCI source revision.

Case A proved this on protected `main@0cd7e76c45736e81cc50db24c6af79ba4a819631`: Build Synology Staging Images run `34202647423` allocated zero image build jobs, and Deploy Synology Staging run `34202722679` successfully reused Platform source `1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19` at `ghcr.io/oteryn/oteryn-platform@sha256:9a84fa634f33c8ae2cc3a0938da16c2d155c70d3b6cea28205f880d0a6bee0df` and Gateway source `1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19` at `ghcr.io/oteryn/oteryn-game-gateway@sha256:a91d9e55328e75628a5e7e9433a369c96d2a16302a21add87758a138b6603347`.

Relative to the former fixed three-entry image matrix, Case A reduced allocated image jobs from 3 to 0 (100%).

## One-component changes

A truthful one-component input allocates only that component's image job. Protected-main provenance for the newly built component is combined with persisted immutable provenance for the unchanged component before staging deployment.

Case B proved this on protected `main@c2e4ff4d035b50a96c88cf99adfb32d60040e317`: Build Synology Staging Images run `34204674858` allocated exactly one `game-gateway` image job, no Platform or deploy-runner image job, and published Gateway `ghcr.io/oteryn/oteryn-game-gateway@sha256:7532c17f31eb314c03a1e6f0a3c0dff5a59c79314080c7168f2eafd4adad73ab` from source `c2e4ff4d035b50a96c88cf99adfb32d60040e317`.

Deploy Synology Staging run `34204735730` then validated persisted release lineage and OCI revisions, reused the unchanged Platform source `1181afbbcf74faa9b19b2f6d05fe28ff7a04fb19` at `ghcr.io/oteryn/oteryn-platform@sha256:9a84fa634f33c8ae2cc3a0938da16c2d155c70d3b6cea28205f880d0a6bee0df`, deployed the new Gateway digest above, passed the full staging health contract, and skipped rollback.

Relative to the former fixed three-entry image matrix, Case B reduced allocated image jobs from 3 to 1 (66.7%).

## Documentation and validation-only paths

Documentation files under the Synology deployment root are not runtime deployment-package inputs merely because of their location. In particular, `README.md`, `PUBLIC_ENDPOINTS.md`, this document, and `.gitignore` are not Synology image/deploy workflow triggers.

Validation-only and operator-only inputs may still run their appropriate repository checks without inventing a staging runtime release. Production deployment remains outside this workflow.

## Safety invariants

- Pull requests never deploy Synology staging.
- Protected `main` and the required `platform-gate`/Merge Queue policy remain authoritative.
- Reused components require persisted source SHA plus immutable digest and OCI revision validation.
- Overall protected-main release identity is tracked separately from per-component source identity.
- Candidate/current/last-good/rollback/recovery state remains fail closed.
- Canary continues to use the separately approved immutable image identity.
