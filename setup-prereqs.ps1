# Voiceeee - Prerequisites checker / installer
# Run in PowerShell: .\setup-prereqs.ps1
# Detects what is missing and prints winget commands. Does NOT install automatically.

$ErrorActionPreference = 'Stop'

function Test-Cmd($name) {
    return [bool](Get-Command $name -ErrorAction SilentlyContinue)
}

function Section($t) {
    Write-Host ""
    Write-Host "==== $t ====" -ForegroundColor Cyan
}

function Ok($msg)   { Write-Host "  [OK]   $msg" -ForegroundColor Green }
function Miss($msg) { Write-Host "  [MISS] $msg" -ForegroundColor Yellow }
function Bad($msg)  { Write-Host "  [BAD]  $msg" -ForegroundColor Red }

Section "Checks"

$missing = @()

# Node + npm
if (Test-Cmd 'node') { Ok ("node " + (node --version)) } else { Miss "node"; $missing += 'node' }
if (Test-Cmd 'npm')  { Ok ("npm "  + (npm  --version)) } else { Miss "npm";  $missing += 'npm' }

# Rust toolchain
if (Test-Cmd 'rustc') {
    Ok ("rustc " + (rustc --version))
    if (Test-Cmd 'cargo') { Ok ("cargo " + (cargo --version)) }
} else {
    Miss "Rust (rustc/cargo) - needed for Tauri backend"
    $missing += 'rust'
}

# CMake
if (Test-Cmd 'cmake') {
    Ok ((cmake --version | Select-Object -First 1))
} else {
    Miss "CMake - needed for whisper.cpp build"
    $missing += 'cmake'
}

# LLVM / clang
if (Test-Cmd 'clang') {
    Ok ((clang --version | Select-Object -First 1))
    if ($env:LIBCLANG_PATH) {
        Ok "LIBCLANG_PATH = $env:LIBCLANG_PATH"
    } else {
        Bad "LIBCLANG_PATH not set - whisper-rs bindgen will fail"
        $missing += 'libclang_path'
    }
} else {
    Miss "LLVM/clang - needed for whisper-rs bindgen"
    $missing += 'llvm'
}

# CUDA
if (Test-Cmd 'nvcc') {
    $line = (nvcc --version | Select-String 'release')
    if ($line) { Ok $line.ToString().Trim() } else { Ok "nvcc present" }
} else {
    Miss "CUDA Toolkit - for GPU-accelerated whisper.cpp"
    $missing += 'cuda'
}

# nvidia-smi
if (Test-Cmd 'nvidia-smi') {
    Ok "nvidia-smi available"
} else {
    Bad "nvidia-smi missing - NVIDIA driver?"
}

# MSVC
$msvcRoot = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
if (Test-Path $msvcRoot) {
    $ver = (Get-ChildItem $msvcRoot | Select-Object -First 1).Name
    Ok "MSVC C++ Build Tools v$ver installed"
} else {
    Miss "MSVC C++ Build Tools 2022"
    $missing += 'msvc'
}

# WebView2
$webview2 = Test-Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients"
if ($webview2) { Ok "WebView2 runtime detected" } else { Miss "WebView2 runtime"; $missing += 'webview2' }

Section "Status"

if ($missing.Count -eq 0) {
    Write-Host "  All prereqs installed. You can move to Phase 1." -ForegroundColor Green
    exit 0
}

Write-Host "  Missing: $($missing -join ', ')" -ForegroundColor Yellow
Write-Host ""
Write-Host "==== Install commands (run them manually) ====" -ForegroundColor Cyan

if ($missing -contains 'rust') {
    Write-Host ""
    Write-Host "# 1) Rust (rustup)"
    Write-Host "winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements"
    Write-Host "# After install, open a NEW terminal and run:"
    Write-Host "rustup default stable-x86_64-pc-windows-msvc"
    Write-Host "rustup update"
}

if ($missing -contains 'cmake') {
    Write-Host ""
    Write-Host "# 2) CMake"
    Write-Host "winget install --id Kitware.CMake -e --accept-source-agreements --accept-package-agreements"
}

if ($missing -contains 'llvm') {
    Write-Host ""
    Write-Host "# 3) LLVM (clang + libclang for bindgen)"
    Write-Host "winget install --id LLVM.LLVM -e --accept-source-agreements --accept-package-agreements"
    Write-Host "# Then set the env var (run as your user, no admin needed):"
    Write-Host '[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")'
    Write-Host "# Close and reopen PowerShell so it picks up the variable."
}

if (($missing -contains 'libclang_path') -and (-not ($missing -contains 'llvm'))) {
    Write-Host ""
    Write-Host "# 3b) LIBCLANG_PATH missing (LLVM is installed but env var not set)"
    Write-Host '[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")'
}

if ($missing -contains 'cuda') {
    Write-Host ""
    Write-Host "# 4) CUDA Toolkit 12.6 (~3 GB)"
    Write-Host "winget install --id Nvidia.CUDA --version 12.6 -e --accept-source-agreements --accept-package-agreements"
    Write-Host "# Or manually: https://developer.nvidia.com/cuda-12-6-0-download-archive"
}

if ($missing -contains 'msvc') {
    Write-Host ""
    Write-Host "# MSVC Build Tools 2022 (workload 'Desktop development with C++')"
    Write-Host 'winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"'
}

if ($missing -contains 'webview2') {
    Write-Host ""
    Write-Host "# WebView2 Runtime (usually already on Windows 11)"
    Write-Host "winget install --id Microsoft.EdgeWebView2Runtime -e"
}

Write-Host ""
Write-Host "  After installs: close and reopen the terminal, then re-run .\setup-prereqs.ps1" -ForegroundColor Cyan
Write-Host "  to confirm everything is OK." -ForegroundColor Cyan
