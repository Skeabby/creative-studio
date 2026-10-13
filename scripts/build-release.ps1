# Builds the launcher binary that is safe to publish.
#
#   powershell -ExecutionPolicy Bypass -File scripts\build-release.ps1
#
# Rust bakes `file!()` and panic locations into the artifact, so a plain
# `cargo build --release` ships the build machine's absolute paths inside the
# download - that is how `C:\Users\<account>` ended up in the v0.3.0 release
# asset. This script remaps the prefix out, relies on `strip = true` from the
# Cargo release profile, and then *verifies* the result by scanning the binary
# for the account name. CI builds its own artifacts; this is the path used for
# GitHub Releases and for local testing of a releasable build.

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $repo "launcher-rs\Cargo.toml"
$exe = Join-Path $repo "launcher-rs\target\release\nucleus-launcher.exe"

# The prefix is read from the environment, never hardcoded: it must not become
# personal data in the repository either.
$prefix = $env:USERPROFILE
if ([string]::IsNullOrWhiteSpace($prefix)) { throw "USERPROFILE is not set" }
$remap = "--remap-path-prefix=$prefix=."
if ([string]::IsNullOrWhiteSpace($env:RUSTFLAGS)) {
    $env:RUSTFLAGS = $remap
} else {
    $env:RUSTFLAGS = "$env:RUSTFLAGS $remap"
}

Write-Host "Building with remapped paths: $prefix -> ."
cargo build --release --manifest-path $manifest
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

if (-not (Test-Path -LiteralPath $exe)) { throw "the build produced no $exe" }

# --- verification ----------------------------------------------------------
# The released binary must not contain the build machine's account name, in any
# of the forms we have seen leak (full path, cargo home, project path).
$raw = [System.IO.File]::ReadAllBytes($exe)
$text = [System.Text.Encoding]::ASCII.GetString($raw)
$needles = @($prefix, $env:USERNAME) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique

$leaks = @()
foreach ($needle in $needles) {
    $hits = ([regex]::Matches($text, [regex]::Escape($needle), 'IgnoreCase')).Count
    if ($hits -gt 0) { $leaks += "$needle ($hits occurrences)" }
}
if ($leaks.Count -gt 0) {
    throw "the binary still embeds build-machine paths: $($leaks -join ', ')"
}

$item = Get-Item -LiteralPath $exe
Write-Host ""
Write-Host ("Built   {0}" -f $item.FullName)
Write-Host ("Size    {0:N0} bytes" -f $item.Length)
Write-Host ("Checked no {0} in the binary" -f ($needles -join ' / '))
