# Creates the "Nucleus" desktop shortcut (+ an icon), so the nucleus
# launches like any other Windows app.
#
#   powershell -ExecutionPolicy Bypass -File scripts\install-shortcut.ps1
param(
    [string]$Name = "Nucleus"
)

$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$assets = Join-Path $repo "assets"
$entry = Join-Path $repo "scripts\nucleus.ps1"
$iconPath = Join-Path $assets "nucleus.ico"

if (-not (Test-Path -LiteralPath $entry)) {
    throw "Missing entry script: $entry"
}

# --- icon ------------------------------------------------------------------
function New-NucleusIcon {
    param([string]$Path)

    Add-Type -AssemblyName System.Drawing
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Path) | Out-Null

    $size = 256
    $bmp = New-Object System.Drawing.Bitmap($size, $size)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)

    $rect = New-Object System.Drawing.Rectangle(8, 8, ($size - 16), ($size - 16))
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $rect,
        [System.Drawing.Color]::FromArgb(255, 122, 162, 255),
        [System.Drawing.Color]::FromArgb(255, 63, 99, 255),
        45.0)
    $rounded = New-Object System.Drawing.Drawing2D.GraphicsPath
    $r = 58
    $rounded.AddArc(8, 8, $r, $r, 180, 90)
    $rounded.AddArc(($size - 8 - $r), 8, $r, $r, 270, 90)
    $rounded.AddArc(($size - 8 - $r), ($size - 8 - $r), $r, $r, 0, 90)
    $rounded.AddArc(8, ($size - 8 - $r), $r, $r, 90, 90)
    $rounded.CloseFigure()
    $g.FillPath($brush, $rounded)

    $white = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
    $faint = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(150, 255, 255, 255))
    $ghost = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(90, 255, 255, 255))

    $g.FillRectangle($white, 66, 66, 56, 56)
    $g.FillRectangle($faint, 138, 66, 56, 56)
    $g.FillRectangle($faint, 66, 138, 56, 56)
    $g.FillRectangle($ghost, 138, 138, 56, 56)

    $g.Dispose()

    $handle = $bmp.GetHicon()
    $icon = [System.Drawing.Icon]::FromHandle($handle)
    $stream = [System.IO.File]::Create($Path)
    $icon.Save($stream)
    $stream.Close()
    $icon.Dispose()
    $bmp.Dispose()
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
$shortcut.TargetPath = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
$shortcut.Arguments = "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$entry`""
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
