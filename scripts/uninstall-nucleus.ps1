# Removes a Nucleus install created by the Inno Setup installer
# (scripts\nucleus.iss) or the portable ZIP.
#
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall-nucleus.ps1
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall-nucleus.ps1 -RemoveUserData
#
# It removes the launcher install directory and the Start Menu / desktop
# shortcuts. The per-user data (installed apps and settings under
# %APPDATA%\Nucleus) is kept by default; pass -RemoveUserData to delete it too,
# or answer the interactive prompt when the flag is omitted.
#
# The script is idempotent: it prints what it removed and what it skipped, and
# refuses to touch any path outside the ones it knows about. PowerShell 5.1.
param(
    [switch]$RemoveUserData,
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

function Write-Line {
    param([string]$Text)
    if (-not $Quiet) { Write-Host $Text }
}

$removed = New-Object System.Collections.Generic.List[string]
$skipped = New-Object System.Collections.Generic.List[string]

# Deletes a directory only when its leaf name matches $ExpectedLeaf, so a
# mis-resolved environment variable can never cause an unrelated delete.
function Remove-DirSafe {
    param([string]$Path, [string]$ExpectedLeaf)

    if ([string]::IsNullOrWhiteSpace($Path)) { return }
    if (-not (Test-Path -LiteralPath $Path)) {
        $skipped.Add("not found: $Path")
        return
    }
    if ((Split-Path -Leaf $Path) -ne $ExpectedLeaf) {
        $skipped.Add("refusing to delete unexpected path: $Path")
        return
    }
    Remove-Item -LiteralPath $Path -Recurse -Force
    $removed.Add($Path)
}

function Remove-FileSafe {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) { return }
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        $skipped.Add("not found: $Path")
        return
    }
    Remove-Item -LiteralPath $Path -Force
    $removed.Add($Path)
}

# --- guard: warn if Nucleus is still running -------------------------------
$running = @(Get-Process -Name "nucleus-launcher" -ErrorAction SilentlyContinue)
if ($running.Count -gt 0) {
    Write-Warning "Nucleus appears to be running (PID $($running.Id -join ', ')). Close it before removing files."
}

# --- launcher install directory --------------------------------------------
$installDirs = @()
if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
    $installDirs += (Join-Path $env:LOCALAPPDATA "Programs\Nucleus")
}
if (-not [string]::IsNullOrWhiteSpace($env:ProgramFiles)) {
    $installDirs += (Join-Path $env:ProgramFiles "Nucleus")
}
$pf86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
if (-not [string]::IsNullOrWhiteSpace($pf86)) {
    $installDirs += (Join-Path $pf86 "Nucleus")
}

foreach ($dir in $installDirs) {
    Remove-DirSafe -Path $dir -ExpectedLeaf "Nucleus"
}

# --- shortcuts -------------------------------------------------------------
$startMenuRoots = @()
if (-not [string]::IsNullOrWhiteSpace($env:APPDATA)) {
    $startMenuRoots += (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs")
}
if (-not [string]::IsNullOrWhiteSpace($env:ProgramData)) {
    $startMenuRoots += (Join-Path $env:ProgramData "Microsoft\Windows\Start Menu\Programs")
}
foreach ($root in $startMenuRoots) {
    Remove-FileSafe -Path (Join-Path $root "Nucleus.lnk")
    Remove-FileSafe -Path (Join-Path $root "Uninstall Nucleus.lnk")
    Remove-DirSafe  -Path (Join-Path $root "Nucleus") -ExpectedLeaf "Nucleus"
}

$desktop = [Environment]::GetFolderPath("DesktopDirectory")
if ([string]::IsNullOrWhiteSpace($desktop) -and -not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
    $desktop = Join-Path $env:USERPROFILE "Desktop"
}
if (-not [string]::IsNullOrWhiteSpace($desktop)) {
    Remove-FileSafe -Path (Join-Path $desktop "Nucleus.lnk")
}

# --- per-user data (opt-in) ------------------------------------------------
$dataDir = $null
if (-not [string]::IsNullOrWhiteSpace($env:APPDATA)) {
    $dataDir = Join-Path $env:APPDATA "Nucleus"   # where the launcher stores apps + window profiles
}

if (-not [string]::IsNullOrWhiteSpace($dataDir) -and (Test-Path -LiteralPath $dataDir)) {
    if (-not $RemoveUserData) {
        # Prompt only when it is safe to do so; otherwise keep the data.
        if (-not $Quiet -and [Environment]::UserInteractive) {
            $answer = Read-Host "Also delete your Nucleus data and installed apps at $dataDir ? [y/N]"
            if ($answer -match '^[Yy]') { $RemoveUserData = $true }
        } else {
            $skipped.Add("kept user data (pass -RemoveUserData to delete): $dataDir")
        }
    }
    if ($RemoveUserData) {
        Remove-DirSafe -Path $dataDir -ExpectedLeaf "Nucleus"
    }
}

# --- report ----------------------------------------------------------------
Write-Line ""
Write-Line "Nucleus uninstall"
Write-Line "-----------------"
if ($removed.Count -eq 0) {
    Write-Line "Removed nothing (Nucleus does not appear to be installed)."
} else {
    Write-Line "Removed:"
    foreach ($item in $removed) { Write-Line "  $item" }
}
$noteable = @($skipped | Where-Object { $_ -notmatch '^not found:' })
if ($noteable.Count -gt 0) {
    Write-Line "Skipped:"
    foreach ($item in $noteable) { Write-Line "  $item" }
}
Write-Line ""
Write-Line "If Nucleus still appears in Settings > Apps, remove it there as well."
