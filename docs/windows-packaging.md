# Windows packaging and installation

**English** | [한국어](windows-packaging.ko.md)

This guide lets a Windows build agent produce and optionally install the native LLM Wiki package.
The repository contains no Python runtime or browser backend. Packaging builds the React UI, Rust
application, bundled fonts, and pinned multilingual ONNX model into Tauri installers.

## Supported build host

- Windows 10 version 1803 or later, or Windows 11, on x64
- Git for Windows
- Node.js 22 LTS with npm
- Rustup using the MSVC toolchain
- Visual Studio 2022 Build Tools with **Desktop development with C++**, MSVC v143, and a Windows
  10 or 11 SDK
- Microsoft Edge WebView2 Runtime; current Windows installations normally include it
- Internet access during the build to install dependencies and restore the checksum-pinned model

Run `winver`, `node --version`, `npm --version`, `rustup --version`, and
`where.exe cl` before packaging. If `cl` is missing, install the Visual Studio workload and start a
new PowerShell session.

## One-command agent workflow

From a clean checkout of `main`, use 64-bit PowerShell:

```powershell
git clone <repository-url> llm-wiki
Set-Location llm-wiki
git checkout main
powershell -ExecutionPolicy Bypass -File .\scripts\package_windows.ps1
```

The script verifies that no Python source remains, installs the stable MSVC Rust toolchain, restores
locked Node dependencies, verifies the embedding model, runs lint/typecheck/React/runtime/Rust
tests, builds the UI, and creates the Tauri bundles.

Expected output locations:

```text
src-tauri\target\release\bundle\msi\*.msi
src-tauri\target\release\bundle\nsis\*.exe
```

To install immediately after a successful build, open PowerShell with permission to install local
applications and run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\package_windows.ps1 -Install
```

The script prefers MSI and otherwise starts the NSIS installer. It waits for completion, fails on
an unsuccessful exit code, and reports MSI code 3010 as a successful install that needs a restart.

## Post-install verification

1. Launch **LLM Wiki** from the Start menu.
2. Confirm that Workbench opens without a terminal, Python process, or localhost service.
3. Create a Capture, close the app, relaunch it, and confirm that the Capture remains.
4. Put a Markdown file in `%USERPROFILE%\Documents\LLM Wiki Vault`, select **Vault Search**, enable
   semantic search, and confirm that the note is returned after background indexing.
5. Open AI Setup and confirm that an API key is reported only as configured/not configured. The key
   is stored through Windows Credential Manager and is never returned to React.

`http://ipc.localhost` in the Tauri content-security policy is WebView2's virtual IPC origin; it is
not a listening TCP socket. Network connections occur only when the user configures an external AI
provider.

## Signing and distribution

Unsigned MSI/NSIS packages are suitable for controlled internal testing but can trigger Windows
SmartScreen. Public distribution requires an organization-owned code-signing certificate and CI
secret configuration. Do not put a PFX file or password in this repository. Signing is intentionally
outside the local packaging script so a build agent cannot silently access release credentials.

If packaging fails, retain the complete PowerShell log and the output of `rustc -vV`, `cargo -vV`,
`node --version`, and `where.exe cl`.
