# OTERYN-20260814 Remaining Branch Audit

Issue: #1068

## Reviewed live state

- Protected `main`: `780ad6c8178206b13d001537ba651b6e0bd22219`.
- Historical Branch Audit run: `31841162360`.
- Artifact: `9234290642` (`sha256:a12c3a60be37e413fca897f586ad29e2ffdffefe35f0cad160cb9c7c49354682`).
- Fully accounted live refs: **80**.
- Dispositions: **33 DELETE, 9 OPEN_PR, 1 PROTECTED, 15 RECOVERY, 22 RETAIN**.
- Reviewed exact deletion candidates: **33**.
- Candidate entries digest: `4fd027de83cf893b40ff0e3eb6fb61cec5a5513fd3bd2a852d4491aaec42230c`.
- Audit implementation digest: `3cad7d0210d20e88ac1e495d6004bd55c6fd7ed948727fa14589eb8200f96966`.

## Decision rule

A branch is deletion-eligible only when current immutable evidence proves that deleting the branch label cannot orphan work:

1. `ANCESTOR_OF_MAIN`: the exact branch head is already in protected `main` history.
2. `REACHABLE_FROM_LIVE_ANCHOR`: the exact branch head remains an ancestor of an explicitly preserved live anchor (open PR, active task, protected branch, or recovery ref).
3. `DUPLICATE_HEAD_RETAINED_AS`: another retained branch points at the exact same commit.

Unique unmerged history is retained. Names containing backup/recovery/rollback make a unique branch more conservative (`RECOVERY`) and never authorize deletion. Age/prefix/name alone never authorize deletion.

## DELETE — reviewed exact candidates

- `audit/OTERYN-20260728-product-completeness` @ `583cae5f430998b2bbdf5e60b59d93f09ec6f4c8` — `ANCESTOR_OF_MAIN`
- `audit/native-oauth-revocation-integrity-20260807-copy` @ `f8a727f3aa33cb123cbab5ff0d04a9d3cefcd69c` — `ANCESTOR_OF_MAIN`
- `chore/OTERYN-20260725-synology-editorial-redeploy-cleanup` @ `bd0bd9883e2753c8a385b3297aaed7a1cb2ce429` — `ANCESTOR_OF_MAIN`
- `docs/OTERYN-20260805-route-view-repair-lifecycle` @ `2abfb961201f7f5d359c5b140dba68be492157be` — `ANCESTOR_OF_MAIN`
- `docs/OTERYN-20260807-account-id-boundary-2` @ `022dbcef97b2dd0ff4eeeda11bf053c9c11341e8` — `ANCESTOR_OF_MAIN`
- `docs/OTERYN-20260808-native-v2-integration-boundary` @ `916629f1f0224adaa1392755a29f99fe31570a03` — `ANCESTOR_OF_MAIN`
- `docs/OTERYN-20260813-architecture-programme-state-reconciliation-issue` @ `0e2351c0b590c24b81d64ed9ec7b2bdea0da09c8` — `ANCESTOR_OF_MAIN`
- `docs/archive-github-only-execution-v1-20260802` @ `f8b67b7c84bfa86f41dd70b8116d2e8878b023cb` — `ANCESTOR_OF_MAIN`
- `docs/finalize-issue-691-dependency-refresh` @ `8584a96adfe2fa220b4b9c70e1575c5f7a98ca5b` — `ANCESTOR_OF_MAIN`
- `docs/github-only-execution-v1-20260802` @ `f8b67b7c84bfa86f41dd70b8116d2e8878b023cb` — `ANCESTOR_OF_MAIN`
- `docs/issue-691-closeout` @ `b8f00be56bd648a35e4fe1d4294633d85781c721` — `ANCESTOR_OF_MAIN`
- `docs/lifecycle-closeout-actions-commonmark-20260807` @ `dace403a9d1baa8f622540f38d205c6fbb1aea25` — `ANCESTOR_OF_MAIN`
- `docs/portal-program-allocation` @ `638df04f616c93d80e33e1abf3f2cf0198163e7a` — `ANCESTOR_OF_MAIN`
- `docs/portal-program-allocation-v2` @ `638df04f616c93d80e33e1abf3f2cf0198163e7a` — `ANCESTOR_OF_MAIN`
- `docs/portal-program-allocation-v3` @ `638df04f616c93d80e33e1abf3f2cf0198163e7a` — `ANCESTOR_OF_MAIN`
- `docs/portal-program-allocation-v4` @ `638df04f616c93d80e33e1abf3f2cf0198163e7a` — `ANCESTOR_OF_MAIN`
- `fix/OTERYN-20260801-marketplace-control-trigger` @ `b249e5e9cb864ba01376efb273be323b90bcd500` — `ANCESTOR_OF_MAIN`
- `ops/OTERYN-20260725-synology-editorial-redeploy` @ `bd0bd9883e2753c8a385b3297aaed7a1cb2ce429` — `ANCESTOR_OF_MAIN`
- `ops/OTERYN-20260725-synology-redeploy-diagnostic` @ `bd0bd9883e2753c8a385b3297aaed7a1cb2ce429` — `ANCESTOR_OF_MAIN`
- `ops/OTERYN-20260727-staging-admin-observable-recovery` @ `d75cfc84fc3ea01eaa24556185888123ffbc5f9c` — `ANCESTOR_OF_MAIN`
- `ops/oteryn-tibia-client-analysis-20260811-sessioncheck` @ `103769e3f7255489bd089435fe6f511bbcee2e10` — `REACHABLE_FROM_LIVE_ANCHOR:ops/oteryn-tibia-client-analysis-20260811`
- `ops/oteryn-tibia-livecheck-20260812` @ `103769e3f7255489bd089435fe6f511bbcee2e10` — `REACHABLE_FROM_LIVE_ANCHOR:ops/oteryn-tibia-client-analysis-20260811`
- `ops/oteryn-tibia-session-check` @ `103769e3f7255489bd089435fe6f511bbcee2e10` — `REACHABLE_FROM_LIVE_ANCHOR:ops/oteryn-tibia-client-analysis-20260811`
- `ops/oteryn-tibia-session-check-20260812` @ `103769e3f7255489bd089435fe6f511bbcee2e10` — `REACHABLE_FROM_LIVE_ANCHOR:ops/oteryn-tibia-client-analysis-20260811`
- `repair/issue-586-branch-lifecycle-implementation` @ `828f8fc5c4b64f6b6ac315e527d82d735ce3de50` — `ANCESTOR_OF_MAIN`
- `repair/issue-586-branch-lifecycle-implementation-2` @ `828f8fc5c4b64f6b6ac315e527d82d735ce3de50` — `ANCESTOR_OF_MAIN`
- `repair/issue-658-branch-lifecycle-implementation-v2` @ `47c6caa6b35c2d2af08d06322c6911721370860d` — `ANCESTOR_OF_MAIN`
- `test/OTERYN-20260727-editorial-media-acceptance-prep` @ `05d08714a0b87ee8a453d01bded605ff42de8bbc` — `ANCESTOR_OF_MAIN`
- `test/OTERYN-20260727-support-legal-acceptance-v2` @ `d08062c653a137e1359b5626fda635b170704cd8` — `ANCESTOR_OF_MAIN`
- `x2` @ `3c59fec368c68196851ebc9a205f91c38c1b6947` — `DUPLICATE_HEAD_RETAINED_AS:x`
- `x3` @ `3c59fec368c68196851ebc9a205f91c38c1b6947` — `DUPLICATE_HEAD_RETAINED_AS:x`
- `x4` @ `3c59fec368c68196851ebc9a205f91c38c1b6947` — `DUPLICATE_HEAD_RETAINED_AS:x`
- `x5` @ `3c59fec368c68196851ebc9a205f91c38c1b6947` — `DUPLICATE_HEAD_RETAINED_AS:x`

## RETAIN — unique unmerged history

- `audit/OTERYN-20260802-production-completion-baseline` @ `d4f2da91c2a328d93f9f3521bb45094295e322ea`
- `audit/OTERYN-20260802-production-completion-baseline-rebase` @ `b259465b07fde7e09698e7b07144539d23ddee63`
- `audit/OTERYN-20260803-portal-evidence-staging` @ `d415144f1c300c7bb92d5824a919f3c7e6bb7a61`
- `audit/platform-continuous-payments-20260805` @ `1fd2f3871fdb5809f9ad4c4df2ca7e924f047865`
- `chore/OTERYN-20260729-archive-character-profile-preferences` @ `2d52490c44375066751a645fc10947208056477e`
- `chore/closeout-actions-storage-hygiene` @ `580e029d8e85b06a389e296357b4926af488fdeb`
- `docs/OTERYN-20260802-agent-governance-sync` @ `cf469580c605cf6ba00c52fa7bd78a3994820130`
- `docs/OTERYN-20260808-native-pre-admission-handoff-restack` @ `e85b9b542b82ac07909f5886f86c85071f96d075`
- `docs/OTERYN-20260813-architecture-programme-state-reconciliation-temp` @ `0399d540d69c4c6ea7d7cf0f11fb02e8b786470f`
- `docs/OTERYN-20260813-player-companion-session-analyzer-v1-archive` @ `6f41c5aa6e02ed1f72da80623bd80050aee1e289`
- `docs/archive-issue-691-dependency-refresh` @ `73c6774084f84852239e17afa59de38df3e78269`
- `ops/OTERYN-20260801-cloudflare-edge-audit` @ `dc627cafb1fe4c99ac4dbc30bf3b81f854f38ee9`
- `repair/issue-561` @ `f93923f11a7bdf310cfb724160dbc9480c331f7e`
- `task/OTERYN-20260722-game-ticket-hardening` @ `646ba18d8caefc05d8eb07e536def9a3c9d9819c`
- `test/OTERYN-20260727-support-legal-acceptance` @ `7b2bfa973e2cb82b990d66a4e403166d2c8657da`
- `test/OTERYN-20260730-route-view-navigation-reachability` @ `31065052c7db1160d5d7b6d648373a0394cced61`
- `tmp-do-not-use` @ `adc1b07c81e8df51829c1d139c36c72cedd5c5d0`
- `tmp-placeholder` @ `39d557136c7b843596c1ca4b16345f134da6af69`
- `tmp/rebase-OTERYN-20260730-character-deletion-contract` @ `3dd59d6b1ecadfda9f3192422a32ecc745073044`
- `tooling/OTERYN-20260729-resume-character-rename` @ `5d88fde66899028a5f6f3b814a4a1b9d4398bc73`
- `trigger/OTERYN-20260729-character-profile-archive` @ `df64df021a9315537542818255651e690398915a`
- `x` @ `3c59fec368c68196851ebc9a205f91c38c1b6947`

## RECOVERY — semantic recovery/rollback/backup anchors

- `backup/OTERYN-20260727-tibia-linux-runner-analysis-pre-restack-20260801` @ `34fba18d58e2532f08b95f2e027283f100f23bd0`
- `backup/OTERYN-20260730-viewport-browser-evidence-dimensions-pre-restack` @ `611b130fb50a1fb2661b890b7f80a70675dad58d`
- `backup/OTERYN-20260808-native-pre-admission-handoff-pre-restack` @ `72129cf6f8291df0ba420c32495be80fd754b6d7`
- `chore/OTERYN-20260724-synology-compose-recovery-closeout` @ `743e9d102fd8895c6230f17af6503aa8871f041f`
- `chore/OTERYN-20260727-staging-admin-recovery-cleanup` @ `9f30731e8fcc97ce8d3300a550239270979b5284`
- `docs/agent-session-stall-recovery-20260804` @ `0b6806b18b9e084d80775a6f2b6207dfaf9fa742`
- `fix/OTERYN-20260724-synology-compose-orphan-recovery` @ `604d1c26c253cb5086c240630057ed893c1bd987`
- `ops/OTERYN-20260727-staging-admin-credential-recovery` @ `5f3947d1e7b7f8908c1f83fb02b51485aacc82d5`
- `ops/synology-rollback-schema-safety-1007` @ `516b4a0e1e200f3f94b4432a8ad3c1e2d9af5459`
- `repair/issue-576-recovery-tmp` @ `d3cb258f49b912cfa752f622b24c37c0affcd3bd`
- `task/OTERYN-20260719-password-recovery-credentials` @ `87aec30b783b136e87989b741e787aa3939c4cf1`
- `task/OTERYN-20260721-e2e-migration-rollback-validation` @ `cddaebfb14e1235bcc00ca242ef82a8c49d84e0c`
- `task/OTERYN-20260721-e2e-migration-rollback-validation-archive` @ `1c0acf3299f61d0dcf8abf7f75e3b380d15ff468`
- `task/OTERYN-20260721-e2e-public-dependency-recovery` @ `ecdf19f325d849634d045274c78425ac8f2ec820`
- `task/OTERYN-20260721-e2e-public-dependency-recovery-archive` @ `c6493104c8cf551c2608ca56f8b16d29a8b25a7b`

## OPEN_PR — protected live work

- `agent/oteryn-20260814-public-map-atlas-integration` @ `65633da2a5f5f3f6a65ea5d532cd1a304cf08371` — open PR #1065
- `closeout/OTERYN-20260814-public-edge-architecture` @ `57c75833cc3d28f89e14eb2eb0a40b2c082048a5` — open PR #1070
- `dependabot/composer/laravel/passport-13.7.6` @ `85cbd1cb81031116bd1718288f4ed604a76fa03d` — open PR #1019
- `dependabot/npm_and_yarn/scripts/acceptance/playwright/test-1.62.1` @ `9f2487277f9d3bd12bef1b6548f858c3f53764d5` — open PR #1020
- `feat/OTERYN-20260730-game-catalog-schema-1-3-consumer` @ `8baec8d66c1bab0b618684096300ab491dacacb4` — open PR #338
- `ops/oteryn-tibia-client-analysis-20260811` @ `97f8df9e64e1e4f0520440073e497f24dad929ef` — open PR #1006
- `refactor/issue-1060-public-content-reverse-edge` @ `884cf25df4dfcadfe3b01642cb499d5a6ba7490d` — open PR #1061
- `repair/issue-1068-remaining-branch-audit` @ `23bb0ae08ad3613ff8d8547d3616bf960994589a` — open PR #1069
- `research/OTERYN-20260811-official-linux-offline-launch` @ `f9ff34b37cf81c400a48f7ab9329393416ac304d` — open PR #988

## PROTECTED

- `main` @ `780ad6c8178206b13d001537ba651b6e0bd22219` — protected default branch

## Manual review conclusion

- All 33 `DELETE` entries have exact branch/SHA evidence and one of the accepted non-loss proofs above.
- The four Tibia session refs are all at `103769e3f7255489bd089435fe6f511bbcee2e10`; that commit is an ancestor of live open-PR branch `ops/oteryn-tibia-client-analysis-20260811`, whose audited head is `97f8df9e64e1e4f0520440073e497f24dad929ef`.
- `x2`–`x5` are exact aliases of `x` at `3c59fec368c68196851ebc9a205f91c38c1b6947`; `x` remains retained.
- No `RETAIN`, `RECOVERY`, `OPEN_PR`, or `PROTECTED` ref is included in the deletion approval.
- Apply must rebuild the live candidate set, validate exact SHA and implementation digest, perform a create/delete recovery probe, recheck each candidate proof immediately before deletion, and verify non-candidate refs remain present.

E2E: `NOT_APPLICABLE` — this is repository Git-ref governance; live GitHub inventory, exact-SHA deletion, recovery probe, and post-delete ref verification are the applicable end-to-end evidence.
