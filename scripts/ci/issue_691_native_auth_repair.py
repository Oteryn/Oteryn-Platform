#!/usr/bin/env python3
from pathlib import Path

path = Path('.github/workflows/native-auth-ephemeral-cutover-rehearsal.yml')
text = path.read_text(encoding='utf-8')

old_permissions = "permissions:\n  contents: read\n  actions: read\n"
new_permissions = "permissions:\n  contents: read\n  actions: read\n  packages: read\n"
if text.count(old_permissions) != 1:
    raise SystemExit('native-auth workflow permissions anchor drifted')
text = text.replace(old_permissions, new_permissions, 1)

stale_env = '''  SOURCE_BUILD_RUN: "30087461815"
  GATEWAY_BUILD_RUN: "30047343772"
  CANARY_BUILD_RUN: "30080248772"
  GATEWAY_ARTIFACT_ID: "8579664843"
  GATEWAY_ARTIFACT_DIGEST: sha256:56909f84aad1c7d58e8ccd52021aa6899fdcd19049c68e634e62384e76f4d142
  CANARY_ARTIFACT_ID: "8591665710"
  CANARY_ARTIFACT_DIGEST: sha256:6b0e13966047c571de2c7a3f948f0f9b54b1619800c4b591cd70adf5a7a860f1
  OTCLIENT_ARTIFACT_ID: "8595332324"
  OTCLIENT_ARTIFACT_DIGEST: sha256:396e0e1fed38c14f43c88cba4e578997ecbd56c2f211ee8b398c712a10c44850
  OTCLIENT_BINARY_SHA256: 9c95ca6e3c26b387f61fcaeb99596d877c1db1bd85a8df1dac310f4a9af03c22
'''
if text.count(stale_env) != 1:
    raise SystemExit('stale native-auth artifact pin block drifted')
text = text.replace(stale_env, '', 1)

build_jobs = r'''jobs:
  build-gateway:
    name: Build exact Game Gateway revision
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    outputs:
      artifact-id: ${{ steps.upload.outputs.artifact-id }}
      artifact-digest: ${{ steps.upload.outputs.artifact-digest }}
    steps:
      - name: Checkout exact Platform/Gateway source
        uses: actions/checkout@v6
        with:
          ref: ${{ env.GATEWAY_REF }}
          persist-credentials: false

      - name: Verify exact Gateway revision
        run: |
          set -euo pipefail
          test "$(git rev-parse HEAD)" = "${GATEWAY_REF}"
          git rev-parse HEAD | tee gateway-commit.txt

      - name: Set up Go
        uses: actions/setup-go@v7
        with:
          go-version-file: services/game-gateway/go.mod
          cache: false

      - name: Test and build Gateway
        working-directory: services/game-gateway
        run: |
          set -euo pipefail
          go test ./...
          go vet ./...
          CGO_ENABLED=0 go build -trimpath -ldflags='-s -w' -o ../../game-gateway ./cmd/game-gateway
          cd ../..
          sha256sum game-gateway | tee gateway-build-sha256.txt

      - name: Upload exact Gateway binary
        id: upload
        uses: actions/upload-artifact@v7
        with:
          name: native-auth-rehearsal-gateway
          path: |
            game-gateway
            gateway-commit.txt
            gateway-build-sha256.txt
          if-no-files-found: error
          retention-days: 7

  build-canary:
    name: Build exact Canary native-auth revision
    runs-on: ubuntu-24.04
    timeout-minutes: 75
    outputs:
      artifact-id: ${{ steps.upload.outputs.artifact-id }}
      artifact-digest: ${{ steps.upload.outputs.artifact-digest }}
    env:
      CC: gcc-13
      CXX: g++-13
      VCPKG_BINARY_CACHE_ACCESS: read
      VCPKG_BINARY_SOURCES: "clear;nuget,https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json,read;nugettimeout,600"
      VCPKG_NUGET_REPOSITORY: https://github.com/blakinio/canary.git
      VCPKG_NUGET_API_KEY: ${{ github.token }}
    steps:
      - name: Checkout exact Canary source
        uses: actions/checkout@v6
        with:
          repository: blakinio/canary
          ref: ${{ env.CANARY_REF }}
          persist-credentials: false

      - name: Verify exact Canary revision
        run: |
          set -euo pipefail
          test "$(git rev-parse HEAD)" = "${CANARY_REF}"
          git rev-parse HEAD | tee canary-commit.txt

      - name: Install Canary build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y ccache ninja-build gcc-13 g++-13 mono-complete
          sudo ln -sf /usr/bin/ninja /usr/bin/ninja-build

      - name: Read Canary vcpkg baseline
        run: |
          set -euo pipefail
          vcpkg_commit_id="$(grep '.builtin-baseline' vcpkg.json | awk -F: '{print $2}' | tr -d ',\" ')"
          test -n "${vcpkg_commit_id}"
          echo "VCPKG_GIT_COMMIT_ID=${vcpkg_commit_id}" >> "${GITHUB_ENV}"

      - name: Set up Canary vcpkg
        uses: lukka/run-vcpkg@b1a0dd252f06b9e25b3c022a9a03bd7a427fb6a2
        with:
          vcpkgGitURL: "https://github.com/microsoft/vcpkg.git"
          vcpkgGitCommitId: ${{ env.VCPKG_GIT_COMMIT_ID }}

      - name: Configure Canary NuGet package cache
        env:
          NUGET_AUTH_TOKEN: ${{ env.VCPKG_NUGET_API_KEY }}
        run: |
          set -euo pipefail
          unset VCPKG_FORCE_SYSTEM_BINARIES
          nuget_path="$(vcpkg fetch nuget 2>&1 | grep -E '^/' | tail -n 1)"
          test -n "${nuget_path}"
          test -f "${nuget_path}"
          mono "${nuget_path}" sources remove -name "GitHubPackages" >/dev/null 2>&1 || true
          mono "${nuget_path}" sources add \
            -source "https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json" \
            -storepasswordincleartext \
            -name "GitHubPackages" \
            -username "${{ github.repository_owner }}" \
            -password "${NUGET_AUTH_TOKEN}"
          mono "${nuget_path}" setapikey "${NUGET_AUTH_TOKEN}" \
            -source "https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json"

      - name: Build exact Canary release
        run: |
          set -euo pipefail
          cmake --preset linux-release \
            -DTOGGLE_BIN_FOLDER=ON \
            -DOPTIONS_ENABLE_CCACHE=ON \
            -DOPTIONS_ENABLE_SCCACHE=OFF
          cmake --build --preset linux-release --parallel 2
          CANARY_PATH="$(find build/linux-release/bin -maxdepth 2 -type f -name canary -perm -111 | head -n 1)"
          test -n "${CANARY_PATH}"
          cp "${CANARY_PATH}" canary-rehearsal
          chmod +x canary-rehearsal
          sha256sum canary-rehearsal | tee canary-build-sha256.txt

      - name: Upload exact Canary binary
        id: upload
        uses: actions/upload-artifact@v7
        with:
          name: native-auth-rehearsal-canary
          path: |
            canary-rehearsal
            canary-commit.txt
            canary-build-sha256.txt
          if-no-files-found: error
          retention-days: 7

  build-otclient:
    name: Build exact controlled OTClient revision
    runs-on: ubuntu-24.04
    timeout-minutes: 75
    outputs:
      artifact-id: ${{ steps.upload.outputs.artifact-id }}
      artifact-digest: ${{ steps.upload.outputs.artifact-digest }}
    env:
      CC: gcc-14
      CXX: g++-14
      VCPKG_BINARY_CACHE_ACCESS: read
      VCPKG_BINARY_SOURCES: "clear;nuget,https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json,read;nugettimeout,600"
      VCPKG_NUGET_REPOSITORY: https://github.com/blakinio/otclient.git
      VCPKG_NUGET_API_KEY: ${{ github.token }}
    steps:
      - name: Checkout exact OTClient source
        uses: actions/checkout@v6
        with:
          repository: blakinio/otclient
          ref: ${{ env.OTCLIENT_REF }}
          persist-credentials: false

      - name: Verify exact OTClient revision
        run: |
          set -euo pipefail
          test "$(git rev-parse HEAD)" = "${OTCLIENT_REF}"
          git rev-parse HEAD | tee otclient-commit.txt

      - name: Install OTClient build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            autoconf autoconf-archive automake ccache gcc-14 g++-14 \
            libgl1-mesa-dev libglu1-mesa-dev libltdl-dev libtool libtool-bin \
            libx11-dev libxcursor-dev libxi-dev libxinerama-dev libxrandr-dev \
            linux-libc-dev make mono-complete ninja-build perl pkg-config
          sudo ln -sf /usr/bin/ninja /usr/bin/ninja-build

      - name: Read OTClient vcpkg baseline
        run: |
          set -euo pipefail
          vcpkg_commit_id="$(grep '.builtin-baseline' vcpkg.json | awk -F: '{print $2}' | tr -d ',\" ')"
          test -n "${vcpkg_commit_id}"
          echo "VCPKG_GIT_COMMIT_ID=${vcpkg_commit_id}" >> "${GITHUB_ENV}"

      - name: Set up OTClient vcpkg
        uses: lukka/run-vcpkg@b1a0dd252f06b9e25b3c022a9a03bd7a427fb6a2
        with:
          vcpkgGitURL: "https://github.com/microsoft/vcpkg.git"
          vcpkgGitCommitId: ${{ env.VCPKG_GIT_COMMIT_ID }}

      - name: Configure OTClient NuGet package cache
        env:
          NUGET_AUTH_TOKEN: ${{ env.VCPKG_NUGET_API_KEY }}
        run: |
          set -euo pipefail
          unset VCPKG_FORCE_SYSTEM_BINARIES
          nuget_path="$(vcpkg fetch nuget 2>&1 | grep -E '^/' | tail -n 1)"
          test -n "${nuget_path}"
          test -f "${nuget_path}"
          mono "${nuget_path}" sources remove -name "GitHubPackages" >/dev/null 2>&1 || true
          mono "${nuget_path}" sources add \
            -source "https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json" \
            -storepasswordincleartext \
            -name "GitHubPackages" \
            -username "${{ github.repository_owner }}" \
            -password "${NUGET_AUTH_TOKEN}"
          mono "${nuget_path}" setapikey "${NUGET_AUTH_TOKEN}" \
            -source "https://nuget.pkg.github.com/${{ github.repository_owner }}/index.json"

      - name: Build exact OTClient release
        uses: lukka/run-cmake@5d55ea7949e25f69f0ecb516d8d572297e03a956
        with:
          configurePreset: linux-release
          buildPreset: linux-release
          configurePresetAdditionalArgs: "['-DTOGGLE_BIN_FOLDER=ON', '-DOPTIONS_ENABLE_IPO=OFF']"

      - name: Verify OTClient executable
        run: |
          set -euo pipefail
          OTCLIENT_PATH="$(find build/linux-release/bin -maxdepth 2 -type f -name otclient -perm -111 | head -n 1)"
          test -n "${OTCLIENT_PATH}"
          cp "${OTCLIENT_PATH}" otclient-rehearsal
          chmod +x otclient-rehearsal
          sha256sum otclient-rehearsal | tee otclient-build-sha256.txt

      - name: Upload exact OTClient binary
        id: upload
        uses: actions/upload-artifact@v7
        with:
          name: native-auth-rehearsal-otclient
          path: |
            otclient-rehearsal
            otclient-commit.txt
            otclient-build-sha256.txt
          if-no-files-found: error
          retention-days: 7

  rehearsal:
'''

if text.count('jobs:\n  rehearsal:\n') != 1:
    raise SystemExit('native-auth jobs/rehearsal anchor drifted')
text = text.replace('jobs:\n  rehearsal:\n', build_jobs, 1)

rehearsal_header = '''  rehearsal:
    name: Full ephemeral production-like native-auth cutover
    runs-on: ubuntu-24.04
    timeout-minutes: 120
'''
rehearsal_replacement = '''  rehearsal:
    name: Full ephemeral production-like native-auth cutover
    needs: [build-gateway, build-canary, build-otclient]
    runs-on: ubuntu-24.04
    timeout-minutes: 120
    env:
      SOURCE_BUILD_RUN: ${{ github.run_id }}
      GATEWAY_BUILD_RUN: ${{ github.run_id }}
      CANARY_BUILD_RUN: ${{ github.run_id }}
      GATEWAY_ARTIFACT_ID: ${{ needs.build-gateway.outputs.artifact-id }}
      GATEWAY_ARTIFACT_DIGEST: ${{ needs.build-gateway.outputs.artifact-digest }}
      CANARY_ARTIFACT_ID: ${{ needs.build-canary.outputs.artifact-id }}
      CANARY_ARTIFACT_DIGEST: ${{ needs.build-canary.outputs.artifact-digest }}
      OTCLIENT_ARTIFACT_ID: ${{ needs.build-otclient.outputs.artifact-id }}
      OTCLIENT_ARTIFACT_DIGEST: ${{ needs.build-otclient.outputs.artifact-digest }}
'''
if text.count(rehearsal_header) != 1:
    raise SystemExit('native-auth rehearsal header drifted')
text = text.replace(rehearsal_header, rehearsal_replacement, 1)

download_start = text.index('      - name: Reuse exact Gateway build artifact\n')
verify_start = text.index('      - name: Verify all declared exact revisions and artifact payloads\n', download_start)
downloads = '''      - name: Download current-run Gateway build
        uses: actions/download-artifact@v8
        with:
          name: native-auth-rehearsal-gateway
          path: gateway-artifact

      - name: Download current-run Canary build
        uses: actions/download-artifact@v8
        with:
          name: native-auth-rehearsal-canary
          path: canary-artifact

      - name: Download current-run OTClient build
        uses: actions/download-artifact@v8
        with:
          name: native-auth-rehearsal-otclient
          path: otclient-artifact

'''
text = text[:download_start] + downloads + text[verify_start:]

verify_start = text.index('      - name: Verify all declared exact revisions and artifact payloads\n')
resolve_start = text.index('      - name: Resolve executable artifacts\n', verify_start)
verify = r'''      - name: Verify all declared exact revisions and current-run artifact payloads
        env:
          GITHUB_TOKEN: ${{ github.token }}
        run: |
          set -euo pipefail
          test "$(git -C platform-source rev-parse HEAD)" = "${PLATFORM_REF}"
          test "$(git -C canary-runtime-source rev-parse HEAD)" = "${CANARY_REF}"
          test "$(git -C otclient-source rev-parse HEAD)" = "${OTCLIENT_REF}"
          test "$(git -C canary-harness rev-parse HEAD)" = "${CANARY_HARNESS_REF}"
          test "$(tr -d '\r\n' < gateway-artifact/gateway-commit.txt)" = "${GATEWAY_REF}"
          test "$(tr -d '\r\n' < canary-artifact/canary-commit.txt)" = "${CANARY_REF}"
          test "$(tr -d '\r\n' < otclient-artifact/otclient-commit.txt)" = "${OTCLIENT_REF}"
          (cd gateway-artifact && sha256sum --check gateway-build-sha256.txt)
          (cd canary-artifact && sha256sum --check canary-build-sha256.txt)
          (cd otclient-artifact && sha256sum --check otclient-build-sha256.txt)

          verify_artifact() {
            local id="$1" digest="$2" expected_name="$3"
            local metadata
            metadata="$(curl -fsSL \
              -H "Authorization: Bearer ${GITHUB_TOKEN}" \
              -H "Accept: application/vnd.github+json" \
              -H "X-GitHub-Api-Version: 2022-11-28" \
              "https://api.github.com/repos/${GITHUB_REPOSITORY}/actions/artifacts/${id}")"
            test "$(jq -r '.id' <<<"${metadata}")" = "${id}"
            test "$(jq -r '.name' <<<"${metadata}")" = "${expected_name}"
            test "$(jq -r '.digest' <<<"${metadata}")" = "${digest}"
            test "$(jq -r '.workflow_run.id' <<<"${metadata}")" = "${GITHUB_RUN_ID}"
            test "$(jq -r '.expired' <<<"${metadata}")" = "false"
          }

          verify_artifact "${GATEWAY_ARTIFACT_ID}" "${GATEWAY_ARTIFACT_DIGEST}" native-auth-rehearsal-gateway
          verify_artifact "${CANARY_ARTIFACT_ID}" "${CANARY_ARTIFACT_DIGEST}" native-auth-rehearsal-canary
          verify_artifact "${OTCLIENT_ARTIFACT_ID}" "${OTCLIENT_ARTIFACT_DIGEST}" native-auth-rehearsal-otclient

'''
text = text[:verify_start] + verify + text[resolve_start:]

old_resolve = '''      - name: Resolve executable artifacts
        run: |
          set -euo pipefail
          mv otclient-artifact/otclient otclient-artifact/otclient-rehearsal
          chmod +x gateway-artifact/game-gateway canary-artifact/canary-rehearsal otclient-artifact/otclient-rehearsal
          echo "REHEARSAL_GATEWAY_BIN=${GITHUB_WORKSPACE}/gateway-artifact/game-gateway" >> "${GITHUB_ENV}"
          echo "REHEARSAL_CANARY_BIN=${GITHUB_WORKSPACE}/canary-artifact/canary-rehearsal" >> "${GITHUB_ENV}"
          echo "REHEARSAL_OTCLIENT_BIN=${GITHUB_WORKSPACE}/otclient-artifact/otclient-rehearsal" >> "${GITHUB_ENV}"
'''
new_resolve = '''      - name: Resolve executable artifacts
        run: |
          set -euo pipefail
          chmod +x gateway-artifact/game-gateway canary-artifact/canary-rehearsal otclient-artifact/otclient-rehearsal
          echo "REHEARSAL_GATEWAY_BIN=${GITHUB_WORKSPACE}/gateway-artifact/game-gateway" >> "${GITHUB_ENV}"
          echo "REHEARSAL_CANARY_BIN=${GITHUB_WORKSPACE}/canary-artifact/canary-rehearsal" >> "${GITHUB_ENV}"
          echo "REHEARSAL_OTCLIENT_BIN=${GITHUB_WORKSPACE}/otclient-artifact/otclient-rehearsal" >> "${GITHUB_ENV}"
'''
if text.count(old_resolve) != 1:
    raise SystemExit('native-auth resolve artifact block drifted')
text = text.replace(old_resolve, new_resolve, 1)

static_assertions = {
    'assert revisions["build"]["gateway_source_build_run"] == "30047343772", revisions': 'assert revisions["build"]["gateway_source_build_run"] == "${{ github.run_id }}", revisions',
    'assert revisions["build"]["otclient_source_build_run"] == "30087461815", revisions': 'assert revisions["build"]["otclient_source_build_run"] == "${{ github.run_id }}", revisions',
    'assert digests["canary_source_build_artifact_id"] == 8591665710, digests': 'assert digests["canary_source_build_artifact_id"] == int("${{ needs.build-canary.outputs.artifact-id }}"), digests',
    'assert digests["gateway_source_build_artifact_id"] == 8579664843, digests': 'assert digests["gateway_source_build_artifact_id"] == int("${{ needs.build-gateway.outputs.artifact-id }}"), digests',
    'assert digests["otclient_source_build_artifact_id"] == 8595332324, digests': 'assert digests["otclient_source_build_artifact_id"] == int("${{ needs.build-otclient.outputs.artifact-id }}"), digests',
    'assert digests["gateway_source_build_run"] == 30047343772, digests': 'assert digests["gateway_source_build_run"] == int("${{ github.run_id }}"), digests',
    'assert digests["otclient_source_build_run"] == 30087461815, digests': 'assert digests["otclient_source_build_run"] == int("${{ github.run_id }}"), digests',
}
for old, new in static_assertions.items():
    if text.count(old) != 1:
        raise SystemExit(f'native-auth retained evidence assertion drifted: {old}')
    text = text.replace(old, new, 1)

for marker in [
    '30087461815', '30047343772', '30080248772',
    '8579664843', '8591665710', '8595332324',
    'actions/download-artifact@v4', 'actions/upload-artifact@v4',
]:
    if marker in text:
        raise SystemExit(f'stale native-auth cross-run provenance remains: {marker}')

required = [
    'needs: [build-gateway, build-canary, build-otclient]',
    'uses: actions/setup-go@v7',
    'uses: actions/download-artifact@v8',
    'uses: actions/upload-artifact@v7',
    'artifact-id: ${{ steps.upload.outputs.artifact-id }}',
    'artifact-digest: ${{ steps.upload.outputs.artifact-digest }}',
    'packages: read',
]
missing = [marker for marker in required if marker not in text]
if missing:
    raise SystemExit(f'missing native-auth self-contained invariant(s): {missing}')

path.write_text(text, encoding='utf-8')
print('Native-auth self-contained transform: PASS')
