# Builds assets\nucleus-shell.exe (WinForms + WebView2 native window) with the
# .NET Framework compiler - no Visual Nucleus, no Rust needed.
#
# The WebView2 interop assemblies must already be in assets\webview2\
# (download/refresh them with scripts\fetch-webview2.ps1).
$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$csc = Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319\csc.exe"
$webview = Join-Path $repo "assets\webview2"

if (-not (Test-Path -LiteralPath $csc)) { throw "csc.exe not found at $csc" }
if (-not (Test-Path -LiteralPath (Join-Path $webview "Microsoft.Web.WebView2.WinForms.dll"))) {
    throw "WebView2 assemblies missing in $webview - run scripts\fetch-webview2.ps1 first."
}

New-Item -ItemType Directory -Force -Path (Join-Path $repo "assets") | Out-Null

& $csc /nologo /target:winexe /out:"$repo\assets\nucleus-shell.exe" `
    "/r:System.Windows.Forms.dll" "/r:System.Drawing.dll" `
    "/r:$webview\Microsoft.Web.WebView2.Core.dll" `
    "/r:$webview\Microsoft.Web.WebView2.WinForms.dll" `
    "$repo\shell\nucleus-shell.cs"

if ($LASTEXITCODE -ne 0) { throw "csc failed with exit code $LASTEXITCODE" }

# The WebView2 managed assemblies and native loader must sit next to the exe.
Copy-Item -LiteralPath (Join-Path $webview "Microsoft.Web.WebView2.Core.dll") `
    -Destination (Join-Path $repo "assets\Microsoft.Web.WebView2.Core.dll") -Force
Copy-Item -LiteralPath (Join-Path $webview "Microsoft.Web.WebView2.WinForms.dll") `
    -Destination (Join-Path $repo "assets\Microsoft.Web.WebView2.WinForms.dll") -Force
Copy-Item -LiteralPath (Join-Path $webview "WebView2Loader.dll") `
    -Destination (Join-Path $repo "assets\WebView2Loader.dll") -Force

"Built: $repo\assets\nucleus-shell.exe"