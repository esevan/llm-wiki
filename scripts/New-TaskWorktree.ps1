param(
    [Parameter(Mandatory = $true)][ValidatePattern("^[a-z0-9][a-z0-9-]*$")][string]$TaskName,
    [Parameter(Mandatory = $true)][string]$BranchName,
    [string]$StartPoint = "main"
)

$ErrorActionPreference = "Stop"
$StartCommit = (& git rev-parse --verify "${StartPoint}^{commit}").Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Could not resolve the requested start point: $StartPoint"
}
$CommonGitDir = (& git rev-parse --path-format=absolute --git-common-dir).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Could not locate the shared Git directory."
}
$RepositoryRoot = Split-Path -Parent $CommonGitDir
$WorktreePath = Join-Path $RepositoryRoot ".worktrees\$TaskName"

if (Test-Path $WorktreePath) {
    throw "Worktree path already exists: $WorktreePath"
}

$RequiredCaches = @(
    "node_modules",
    "src-tauri\target",
    "src-tauri\resources\embedding-model",
    "dist"
)
foreach ($RelativePath in $RequiredCaches) {
    $CachePath = Join-Path $RepositoryRoot $RelativePath
    if (-not (Test-Path $CachePath -PathType Container)) {
        throw "Shared cache is not primed: $CachePath"
    }
}

$ModelSource = Join-Path $RepositoryRoot "src-tauri\resources\embedding-model"
& node (Join-Path $PSScriptRoot "prepare_embedding_model.mjs") `
    --check-only --model-dir $ModelSource
if ($LASTEXITCODE -ne 0) {
    throw "Bundled embedding-model assets are missing or invalid."
}

& git -C $RepositoryRoot worktree add $WorktreePath -b $BranchName $StartCommit
if ($LASTEXITCODE -ne 0) {
    throw "Git could not create the task worktree."
}

New-Item -ItemType Junction -Path (Join-Path $WorktreePath "node_modules") `
    -Target (Join-Path $RepositoryRoot "node_modules") | Out-Null
New-Item -ItemType Junction -Path (Join-Path $WorktreePath "src-tauri\target") `
    -Target (Join-Path $RepositoryRoot "src-tauri\target") | Out-Null
Copy-Item -Recurse (Join-Path $RepositoryRoot "dist") (Join-Path $WorktreePath "dist")

$ModelTarget = Join-Path $WorktreePath "src-tauri\resources\embedding-model"
Get-ChildItem $ModelSource -File |
    Where-Object Name -NotIn @("manifest.json", "NOTICE.md") |
    ForEach-Object {
        New-Item -ItemType HardLink -Path (Join-Path $ModelTarget $_.Name) `
            -Target $_.FullName | Out-Null
    }

Write-Host "Created cached worktree: $WorktreePath"
Write-Host "Shared node_modules, Cargo target, model assets, and frontend build are ready."
