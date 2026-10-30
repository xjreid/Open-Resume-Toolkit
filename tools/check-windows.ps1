#Requires -Version 7.0
# Checks native adapters and optionally desktop source without linking ORT.
# This script never packages, installs, or launches ORT.
[CmdletBinding()]
param(
    [switch]$CheckOnly,
    [switch]$NativeVault,
    [switch]$NativeContainment,
    [switch]$DesktopSource
)
$ErrorActionPreference = 'Stop'
if (-not $IsWindows -or [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64') {
    throw 'These checks require Windows x64.'
}
$ortCheckRoot = Split-Path -Parent $PSScriptRoot
$ortPreviousVaultOptIn = $env:ORT_RUN_OS_VAULT_TESTS
$ortPreviousParserOptIn = $env:ORT_RUN_WINDOWS_PARSER_TESTS
Push-Location -LiteralPath $ortCheckRoot
try {
    $ortLocalTools = Join-Path $ortCheckRoot 'target/windows-toolchain'
    $ortLocalNode = Join-Path $ortLocalTools 'node-v24.16.0-win-x64'
    if (Test-Path -LiteralPath (Join-Path $ortLocalNode 'node.exe')) {
        $env:PATH = $ortLocalNode + ';' + (Join-Path $ortLocalTools 'js-bin') + ';' + $env:PATH
    }
    if (Test-Path -LiteralPath (Join-Path $ortLocalTools 'cargo/bin/cargo.exe')) {
        $env:CARGO_HOME = Join-Path $ortLocalTools 'cargo'
        $env:RUSTUP_HOME = Join-Path $ortLocalTools 'rustup'
        $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-msvc'
        $env:PATH = (Join-Path $ortLocalTools 'cargo/bin') + ';' + $env:PATH
    }
    $ortPortablePerl = Join-Path $ortLocalTools 'strawberry-perl/perl/bin'
    if (Test-Path -LiteralPath (Join-Path $ortPortablePerl 'perl.exe')) {
        $env:PATH = $ortPortablePerl + ';' + $env:PATH
    }
    $ortVsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (-not (Test-Path -LiteralPath $ortVsWhere)) {
        throw 'Microsoft C++ Build Tools are required; see DEVELOPMENT.md.'
    }
    $ortVsInstall = & $ortVsWhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($LASTEXITCODE -ne 0 -or -not $ortVsInstall) { throw 'MSVC x64 tools were not found.' }
    Import-Module (Join-Path $ortVsInstall 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
    Enter-VsDevShell -VsInstallPath $ortVsInstall -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
    $ortRustVersion = & rustc --version
    if ($LASTEXITCODE -ne 0 -or $ortRustVersion -notmatch '^rustc 1\.98\.0 ') {
        throw 'Rust 1.98.0 is required.'
    }
    $ortPerlPlatform = & perl -e 'print $^O'
    if ($LASTEXITCODE -ne 0 -or $ortPerlPlatform -ne 'MSWin32') {
        throw 'Native Windows Perl is required by vendored OpenSSL; Git/MSYS Perl is insufficient.'
    }
    $ortLibraries = @('-p', 'ort-windows', '-p', 'ort-vault', '-p', 'ort-platform', '-p', 'ort-storage', '-p', 'ort-ipc', '-p', 'ort-native-host')
    & cargo fmt --all --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed.' }
    & cargo clippy --locked @ortLibraries --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Windows library Clippy check failed.' }
    if ($CheckOnly) {
        & cargo check --locked @ortLibraries
    } else {
        # test-support is shared by fixture consumers. Do not let that feature
        # implicitly opt the selected ort-windows package into credential-using
        # parser integration tests; those require -NativeContainment below.
        & cargo test --locked -p ort-vault -p ort-platform -p ort-storage -p ort-ipc -p ort-native-host
        if ($LASTEXITCODE -ne 0) { throw 'Windows library verification failed.' }
        & cargo test --locked -p ort-windows --lib
    }
    if ($LASTEXITCODE -ne 0) { throw 'Windows library verification failed.' }
    & cargo clippy --locked -p ort-windows -p ort-ipc -p ort-native-host -p ort-parser-helper --all-targets --features ort-windows/native-probes,ort-native-host/dev-browser-bridge -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Windows development bridge/helper source checks failed.' }
    if ($DesktopSource) {
        & cargo clippy --locked -p ort-desktop --all-targets --features dev-browser-bridge -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw 'Windows desktop source/test-target check failed.' }
    }
    if ($NativeVault) {
        $env:ORT_RUN_OS_VAULT_TESTS = '1'
        & cargo test --locked -p ort-vault -- --ignored
        if ($LASTEXITCODE -ne 0) { throw 'Synthetic native credential verification failed.' }
    }
    if ($NativeContainment) {
        $env:ORT_RUN_WINDOWS_PARSER_TESTS = '1'
        & cargo test --locked -p ort-windows --features native-probes --test parser -- --ignored
        if ($LASTEXITCODE -ne 0) { throw 'Disposable parser containment qualification failed.' }
        & cargo test --locked -p ort-ipc --features dev-browser-bridge
        if ($LASTEXITCODE -ne 0) { throw 'Disposable private bridge qualification failed.' }
    }
} finally {
    if ($null -eq $ortPreviousParserOptIn) {
        Remove-Item Env:ORT_RUN_WINDOWS_PARSER_TESTS -ErrorAction SilentlyContinue
    } else {
        $env:ORT_RUN_WINDOWS_PARSER_TESTS = $ortPreviousParserOptIn
    }
    if ($null -eq $ortPreviousVaultOptIn) {
        Remove-Item Env:ORT_RUN_OS_VAULT_TESTS -ErrorAction SilentlyContinue
    } else {
        $env:ORT_RUN_OS_VAULT_TESTS = $ortPreviousVaultOptIn
    }
    Pop-Location
}
