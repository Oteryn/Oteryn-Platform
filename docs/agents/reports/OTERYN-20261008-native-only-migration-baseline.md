# Native-only Platform migration baseline

Owner direction: 2026-10-08. Governing Issue: #1474. Jira: KAN-41.

## Evidence scope

Direct Platform file inspection at `db1ddecbe5559065a961bb1edd11794b5a40e214`; repository tree and live PR metadata. This is an initial dependency matrix, not a complete call-site audit, runtime test or Game contract-coverage claim.

| Surface | Verified dependency | Required replacement / removal | Existing lane |
| --- | --- | --- | --- |
| config/database.php | Four Canary MySQL connections and canary_runtime Redis | Accepted Game APIs/projections; remove connections and credentials | #1474 |
| app/Providers/AppServiceProvider.php | Concrete Canary provisioning, creation, transfer DI bindings | Native contract consumers; no direct Game storage | #277 / #1474 |
| config/game-auth.php | Protocol 1 default; Oteryn OTClient; legacy ticket audience | Native client identity and ticket issuance, no Canary fallback | #1419; preserve #1471 ownership |
| deploy/synology/compose.yml | Canary container/image/data volume; database inputs; private session proxy | Native-only Platform topology with separately operated Game endpoint | #1339 / #1461 / #1456 |
| deploy/synology/compose.yml | Gateway points to canary-session-internal | Native Gateway admission route | #1419 |
| Native catalog lifecycle | ADR 0034 retains legacy compatibility as current track; #1460 documents legacy-shaped persistence | Reconcile accepted architecture to eventual native-only lifecycle; qualify native snapshots | #1460 / #330 / #489 |

## Tree-discovered surfaces requiring call-site inspection

- app/Accounts/Actions/ProvisionCanaryAccount.php and IdentityCanaryAccount.
- app/CanaryIntegration/**, including account provisioning, character creation/transfer, privilege verifiers and runtime Redis reader.
- app/PublicGameData/CanaryGameDataRepository.php and CanaryChannelRuntimeService.php.
- database/provisioning/canary-*.sql.template and historical migrations.
- services/game-gateway: distinguish existing native handler from legacy session client and shared transport.
- Marketplace/Characters contracts and public consumers; no native replacement is claimed from filenames.
- Deploy scripts, release contract, rollback, health checks and CI workflows: enumerate executable Canary service assumptions before removing Compose dependencies.
- Downloads/client selection, fixtures and acceptance routes: trace user journeys rather than renaming identifiers.

## First implementation slices

1. Inventory ticket issuer/controller/OAuth policy and claimed #1471 paths; allocate the native client/ticket successor only after overlap resolution.
2. Trace public data repository consumers and available accepted Game projection contracts. Missing producer capability is explicit unavailable state, not fake data.
3. Reconcile deployment repair candidates to the native-only final topology. Do not merely delete canary from Compose while scripts still require it.
4. Finish native catalog architecture disposition before changing persistence. Legacy row preservation during migration is not permanent compatibility authorization.
5. Cut over qualified consumers, remove adapters/configuration, then verify native-only build, boot and physical E2E.

## Qualification boundaries

Main platform-gate/test/runtime-tests passed at the inspected head. Main acceptance, checkpoint-validation and deploy checks have failures; root causes remain unverified here. Native-only readiness cannot be claimed from green unit tests or merged LCFA alone.

Deployment data, accepted Character lifecycle contracts and complete projection coverage remain UNKNOWN. No destructive migration, live host change, Game repository mutation or production enablement occurs in this baseline.

Integration capability: no current session native merge-async operation was discovered; delegated capability has not been qualified. This draft is prepared for review, not autonomous protected integration. No direct merge or generic auto-merge substitution is permitted.
