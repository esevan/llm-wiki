param(
    [switch]$Install
)

$ErrorActionPreference = "Stop"
$RepositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepositoryRoot

if ($env:OS -ne "Windows_NT") {
    throw "This packaging script must run on Windows."
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$Command,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )

    & $Command @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Command failed with exit code $LASTEXITCODE."
    }
}

foreach ($Command in @("git", "node", "npm", "rustup", "cargo")) {
    if (-not (Get-Command $Command -ErrorAction SilentlyContinue)) {
        throw "Required command is unavailable: $Command"
    }
}

$PythonFiles = @(git ls-files "*.py" "*.pyi")
if ($LASTEXITCODE -ne 0) {
    throw "Could not inspect tracked source files."
}
if ($PythonFiles.Count -ne 0) {
    throw "Python files remain in the native-only source tree."
}

$Toolchain = "stable-x86_64-pc-windows-msvc"
Invoke-Checked -Command "rustup" -Arguments @(
    "toolchain", "install", $Toolchain, "--profile", "minimal",
    "--component", "rustfmt", "--component", "clippy"
)
$env:RUSTUP_TOOLCHAIN = $Toolchain

Invoke-Checked -Command "npm" -Arguments @("ci")
Invoke-Checked -Command "npm" -Arguments @("run", "prepare:embedding")
Invoke-Checked -Command "npm" -Arguments @("run", "lint")
Invoke-Checked -Command "npm" -Arguments @("run", "typecheck")
Invoke-Checked -Command "npm" -Arguments @("test")
Invoke-Checked -Command "npm" -Arguments @("run", "build")
Invoke-Checked -Command "cargo" -Arguments @(
    "fmt", "--manifest-path", "src-tauri/Cargo.toml", "--check"
)
Invoke-Checked -Command "cargo" -Arguments @(
    "clippy", "--manifest-path", "src-tauri/Cargo.toml", "--all-targets",
    "--", "-D", "warnings"
)
Invoke-Checked -Command "cargo" -Arguments @(
    "test", "--manifest-path", "src-tauri/Cargo.toml"
)
Invoke-Checked -Command "npm" -Arguments @("run", "tauri:build")

$BundleRoot = Join-Path $RepositoryRoot "src-tauri\target\release\bundle"
$Installers = @(
    Get-ChildItem $BundleRoot -Recurse -File |
        Where-Object { $_.Extension -in @(".msi", ".exe") }
)
if ($Installers.Count -eq 0) {
    throw "Tauri completed without producing an MSI or NSIS installer."
}

Write-Host "Windows packages:"
$Installers | ForEach-Object { Write-Host " - $($_.FullName)" }

if ($Install) {
    $Msi = $Installers | Where-Object Extension -eq ".msi" | Select-Object -First 1
    if ($Msi) {
        $Process = Start-Process msiexec.exe -ArgumentList "/i", "`"$($Msi.FullName)`"" -Wait -PassThru
        $SuccessfulExitCodes = @(0, 3010)
    } else {
        $Process = Start-Process $Installers[0].FullName -Wait -PassThru
        $SuccessfulExitCodes = @(0)
    }
    if ($Process.ExitCode -notin $SuccessfulExitCodes) {
        throw "Installer exited with code $($Process.ExitCode)."
    }
    if ($Process.ExitCode -eq 3010) {
        Write-Host "LLM Wiki installation completed; Windows requested a restart."
    } else {
        Write-Host "LLM Wiki installation completed."
    }
}
