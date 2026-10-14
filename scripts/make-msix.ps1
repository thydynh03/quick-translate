# Packs the Windows build as an MSIX for the Microsoft Store.
# The package is not signed: the Store signs it after upload.
#
#   pwsh scripts/make-msix.ps1 -Version 0.1.22 -IdentityName 12345Name.Swipelate `
#     -Publisher "CN=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX" -PublisherDisplayName "thydynh03"
#
# The three identity values are on Partner Center > the product > Product identity.
param(
    [Parameter(Mandatory)] [string] $Version,
    [Parameter(Mandatory)] [string] $IdentityName,
    [Parameter(Mandatory)] [string] $Publisher,
    [Parameter(Mandatory)] [string] $PublisherDisplayName,
    [string] $Exe = "src-tauri/target/release/swipelate.exe",
    [string] $Out = "Swipelate-Store-x64.msix"
)
$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent

# The Store wants four numbers with the last one 0.
$parts = $Version.TrimStart("v").Split("-")[0].Split(".")
if ($parts.Count -ne 3) { throw "Version must look like 1.2.3, got $Version" }
$packageVersion = "$($parts -join '.').0"

$layout = Join-Path ([System.IO.Path]::GetTempPath()) "swipelate-msix"
Remove-Item $layout -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force "$layout/Assets" | Out-Null

Copy-Item (Join-Path $root $Exe) "$layout/swipelate.exe"
$icons = Join-Path $root "src-tauri/icons"
foreach ($name in "StoreLogo.png", "Square44x44Logo.png", "Square150x150Logo.png") {
    Copy-Item "$icons/$name" "$layout/Assets/$name"
}

function Escape([string] $text) { [System.Security.SecurityElement]::Escape($text) }
$manifest = Get-Content (Join-Path $root "src-tauri/windows/msix/AppxManifest.xml") -Raw -Encoding utf8
$manifest = $manifest.Replace("{{IDENTITY_NAME}}", (Escape $IdentityName)).
    Replace("{{PUBLISHER}}", (Escape $Publisher)).
    Replace("{{PUBLISHER_DISPLAY_NAME}}", (Escape $PublisherDisplayName)).
    Replace("{{VERSION}}", $packageVersion)
[System.IO.File]::WriteAllText("$layout/AppxManifest.xml", $manifest, (New-Object System.Text.UTF8Encoding $false))

$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
    Sort-Object { [version]$_.Directory.Parent.Name } | Select-Object -Last 1
if (-not $makeappx) { throw "makeappx.exe not found: install the Windows SDK" }

$folder = Split-Path $Out -Parent
if ($folder) { New-Item -ItemType Directory -Force $folder | Out-Null }
& $makeappx.FullName pack /o /d $layout /p $Out
if ($LASTEXITCODE -ne 0) { throw "makeappx failed ($LASTEXITCODE)" }
Write-Host "Packed $Out ($packageVersion)"
