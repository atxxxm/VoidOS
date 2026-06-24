#Requires -Version 5.1
param(
    [switch]$BuildOnly,  # Only build, skip QEMU
    [switch]$RunOnly     # Only run QEMU, skip build
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path

function Require($cmd, $hint) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        Write-Error "$cmd not found. $hint"
        exit 1
    }
}

# ── Build ────────────────────────────────────────────────────────────────────

if (-not $RunOnly) {
    Require "cargo" "Install Rust: https://rustup.rs"

    # Ensure the Linux musl cross-compilation target is installed
    $installed = rustup target list --installed 2>&1
    if ($installed -notmatch "x86_64-unknown-linux-musl") {
        Write-Host "Installing x86_64-unknown-linux-musl target..."
        rustup target add x86_64-unknown-linux-musl
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    # Ensure cargo-zigbuild is installed (provides musl cross-linker on Windows)
    if (-not (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue)) {
        Write-Host "Installing cargo-zigbuild..."
        cargo install cargo-zigbuild
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    # Zig must be installed separately: winget install zig.zig
    if (-not (Get-Command zig -ErrorAction SilentlyContinue)) {
        Write-Error "zig not found.`nInstall it with:  winget install zig.zig`nThen restart the terminal and re-run build.ps1"
        exit 1
    }

    Write-Host "Building VoidOS..."
    Push-Location "$Root\void-build"
    try {
        cargo run
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }

    Write-Host "Build complete!"
}

# ── Run ──────────────────────────────────────────────────────────────────────

if (-not $BuildOnly) {
    # Try PATH first, then common QEMU install locations on Windows
    $QemuCmd = Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue
    $Qemu = if ($QemuCmd) { $QemuCmd.Source } else { $null }
    if (-not $Qemu) {
        $candidates = @(
            "C:\Program Files\qemu\qemu-system-x86_64.exe",
            "C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
            "C:\msys64\usr\bin\qemu-system-x86_64.exe",
            "C:\msys64\mingw64\bin\qemu-system-x86_64.exe",
            "C:\msys64\ucrt64\bin\qemu-system-x86_64.exe",
            "C:\msys64\clang64\bin\qemu-system-x86_64.exe"
        )
        $Qemu = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    }
    if (-not $Qemu) {
        Write-Error "qemu-system-x86_64 not found in PATH or default install locations.`nInstall QEMU: https://www.qemu.org/download/#windows"
        exit 1
    }

    $Kernel    = "$Root\os\kernel\bzImage"
    $Initramfs = "$Root\os\initramfs.cpio"

    if (-not (Test-Path $Kernel)) {
        Write-Error "Kernel not found: $Kernel (run without -RunOnly first)"
        exit 1
    }
    if (-not (Test-Path $Initramfs)) {
        Write-Error "Initramfs not found: $Initramfs (run without -RunOnly first)"
        exit 1
    }

    # QEMU (MSYS2 build) can't handle non-ASCII paths — copy to a temp dir first
    $TmpDir = "$env:TEMP\voidOS"
    New-Item -ItemType Directory -Force $TmpDir | Out-Null
    Copy-Item $Kernel    "$TmpDir\bzImage"
    Copy-Item $Initramfs "$TmpDir\initramfs.cpio"

    Write-Host "Starting VoidOS in QEMU... (exit: Ctrl+A then X)"
    & $Qemu `
        -m 1024 `
        -kernel "$TmpDir\bzImage" `
        -initrd "$TmpDir\initramfs.cpio" `
        -append "console=tty0 console=ttyS0,115200 init=/init" `
        -nographic
}
