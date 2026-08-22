param(
    [ValidateSet('smoke','critical','full','account-lifecycle','portability','responsive','resilience','accessibility','coverage-strict')]
    [string]$Profile = 'smoke'
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
$composeFile = Join-Path $PSScriptRoot 'compose.yml'
$projectName = 'oteryn-portal-e2e-1219'

Push-Location $repoRoot
try {
    $sha = (git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $sha -notmatch '^[0-9a-f]{40}$') {
        throw 'Could not resolve exact repository SHA.'
    }

    $env:OTERYN_E2E_SHA = $sha
    $env:PORTAL_E2E_PROFILE = $Profile
    $env:COMPOSE_PROJECT_NAME = $projectName

    Write-Host "Oteryn Portal Docker E2E: profile=$Profile sha=$sha"
    docker compose -f $composeFile up --build --abort-on-container-exit --exit-code-from portal-e2e portal-e2e
    if ($LASTEXITCODE -ne 0) {
        throw "Portal Docker E2E profile '$Profile' failed."
    }
}
finally {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'SilentlyContinue'
    & docker compose -f $composeFile down --volumes --remove-orphans *> $null
    $ErrorActionPreference = $previousErrorActionPreference
    Remove-Item Env:OTERYN_E2E_SHA -ErrorAction SilentlyContinue
    Remove-Item Env:PORTAL_E2E_PROFILE -ErrorAction SilentlyContinue
    Remove-Item Env:COMPOSE_PROJECT_NAME -ErrorAction SilentlyContinue
    Pop-Location
}
