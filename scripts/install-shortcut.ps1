# Creates the "Nucleus" desktop shortcut (+ an icon), so the nucleus
# launches like any other Windows app.
#
#   powershell -ExecutionPolicy Bypass -File scripts\install-shortcut.ps1
#
# The shortcut points at the native Rust launcher
# (launcher-rs\target\release\nucleus-launcher.exe) and falls back to the
# legacy TypeScript entry (scripts\nucleus.ps1) when it has not been built.
param(
    [string]$Name = "Nucleus"
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $repo "assets"
$exe = Join-Path $repo "launcher-rs\target\release\nucleus-launcher.exe"
$entry = Join-Path $repo "scripts\nucleus.ps1"
$iconPath = Join-Path $assets "nucleus.ico"

if (-not (Test-Path -LiteralPath $exe) -and -not (Test-Path -LiteralPath $entry)) {
    throw "Neither the Rust launcher ($exe) nor the legacy entry ($entry) exists."
}

# --- icon ------------------------------------------------------------------
# Builds a multi-size .ico from the brand logo (docs/brand/logo.png) so the
# launcher carries the on-brand INK-tile mark in the taskbar and shortcuts.
function New-NucleusIcon {
    param([string]$Path)

    Add-Type -AssemblyName System.Drawing
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null

    $logo = Join-Path $repo "docs\brand\logo.png"
    if (-not (Test-Path -LiteralPath $logo)) {
        throw "Missing brand logo for icon generation: $logo"
    }

    $src = [System.Drawing.Image]::FromFile($logo)

    # Frames are stored as PNG inside the ICO (supported since Vista).
    $sizes = 256, 64, 48, 32, 16
    $pngs = @()
    foreach ($s in $sizes) {
        $bmp = New-Object System.Drawing.Bitmap($s, $s)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.DrawImage($src, 0, 0, $s, $s)
        $g.Dispose()
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $pngs += , $ms.ToArray()
        $ms.Dispose()
        $bmp.Dispose()
    }
    $src.Dispose()

    $count = $pngs.Count
    $offset = 6 + 16 * $count
    $out = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($out)

    # ICO header: reserved=0, type=1 (icon), count.
    $bw.Write([UInt16]0)
    $bw.Write([UInt16]1)
    $bw.Write([UInt16]$count)

    for ($i = 0; $i -lt $count; $i++) {
        $s = $sizes[$i]
        $dim = if ($s -ge 256) { 0 } else { $s }
        $bw.Write([Byte]$dim)     # width  (0 means 256)
        $bw.Write([Byte]$dim)     # height
        $bw.Write([Byte]0)        # color count
        $bw.Write([Byte]0)        # reserved
        $bw.Write([UInt16]1)      # color planes
        $bw.Write([UInt16]32)     # bits per pixel
        $bw.Write([UInt32]$pngs[$i].Length)
        $bw.Write([UInt32]$offset)
        $offset += $pngs[$i].Length
    }
    foreach ($frame in $pngs) { $bw.Write($frame) }
    $bw.Flush()
    [System.IO.File]::WriteAllBytes($Path, $out.ToArray())
    $bw.Dispose()
    $out.Dispose()
}

try {
    New-NucleusIcon -Path $iconPath
    Write-Host "Icon written: $iconPath"
} catch {
    Write-Warning "Could not generate icon ($($_.Exception.Message)); the shortcut will use a default icon."
    $iconPath = $null
}

# --- shortcut --------------------------------------------------------------
$desktop = [Environment]::GetFolderPath("DesktopDirectory")
if ([string]::IsNullOrWhiteSpace($desktop) -or -not (Test-Path -LiteralPath $desktop)) {
    $desktop = Join-Path $env:USERPROFILE "Desktop"
}

$linkPath = Join-Path $desktop "$Name.lnk"

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($linkPath)
if (Test-Path -LiteralPath $exe) {
    $shortcut.TargetPath = $exe
    $shortcut.Arguments = ""
} else {
    Write-Warning "Rust launcher not built; falling back to scripts\nucleus.ps1. Build it with: cargo build --release --manifest-path launcher-rs\Cargo.toml"
    $shortcut.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
    $shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$entry`""
}
# The working directory stays the repo root: that is where the launcher looks
# for `apps\` to build its catalogue.
$shortcut.WorkingDirectory = $repo
$shortcut.Description = "Nucleus - launcher for the nucleus apps"
if ($iconPath -and (Test-Path -LiteralPath $iconPath)) {
    $shortcut.IconLocation = "$iconPath,0"
}
$shortcut.Save()

Write-Host ""
Write-Host "Shortcut created:"
Write-Host "  $linkPath"
Write-Host ""
Write-Host "Double-click it to open Nucleus."
