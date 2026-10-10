# Creative Studio - desktop entry point.
# Runs the launcher (which opens its own window), and shuts the studio down
# again when that window is closed.
$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$cli = Join-Path $repo "packages\launcher\dist\cli.js"

if (-not (Test-Path -LiteralPath $cli)) {
    Write-Error "Launcher not built. Run 'npm install; npm run build' in $repo first."
    exit 1
}

Set-Location -LiteralPath $repo
& node $cli up
