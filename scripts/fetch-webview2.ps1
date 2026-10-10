# Downloads the WebView2 WinForms interop assemblies from nuget.org into
# assets\webview2 (used to compile studio-shell.exe).
$ErrorActionPreference = "Stop"

$repo = Split-Path -Parent $PSScriptRoot
$dir = Join-Path $repo "assets\webview2"
New-Item -ItemType Directory -Force -Path $dir | Out-Null

$index = Invoke-RestMethod https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/index.json
$version = ($index.versions | Where-Object { $_ -notmatch "-" } | Select-Object -Last 1)
"latest Microsoft.Web.WebView2: $version"

$pkg = Join-Path $env:TEMP "microsoft.web.webview2.$version.nupkg"
Invoke-WebRequest "https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/$version/microsoft.web.webview2.$version.nupkg" -OutFile $pkg

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($pkg)
try {
    $zip.Entries | Where-Object {
        $_.FullName -match "lib/net462/(Microsoft.Web.WebView2.(Core|WinForms).dll)$|runtimes/win-x64/native/WebView2Loader.dll$"
    } | ForEach-Object {
        $target = Join-Path $dir $_.Name
        [System.IO.Compression.ZipFileExtensions]::ExtractToFile($_, $target, $true)
        "  extracted: $($_.Name)"
    }
} finally {
    $zip.Dispose()
}
Remove-Item -LiteralPath $pkg -Force
"ready in $dir"