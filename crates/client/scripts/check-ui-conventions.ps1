# Fail when console UI uses raw <button class="btn ..."> instead of BCButton.
# Guest / client-api crates are excluded (see docs/ui/components.md).

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

$ScanDirs = @(
    "crates/client-shared/src",
    "crates/client-access/src",
    "crates/client-connect/src",
    "crates/client-log/src",
    "crates/client-models/src",
    "crates/client-monitor/src",
    "crates/client-playground/src",
    "crates/client-settings/src",
    "crates/client-users/src",
    "src"
)

$violations = 0

function Test-Pattern {
    param(
        [string]$Label,
        [string]$Pattern
    )
    $hits = @()
    foreach ($dir in $ScanDirs) {
        if (-not (Test-Path $dir)) { continue }
        $files = Get-ChildItem -Path $dir -Filter "*.rs" -Recurse -File
        foreach ($file in $files) {
            $content = Get-Content -Raw -Path $file.FullName
            if ($content -match $Pattern) {
                $hits += "${file}: matched"
            }
        }
    }
    if ($hits.Count -gt 0) {
        Write-Host "::error::$Label"
        $hits | ForEach-Object { Write-Host $_ }
        $script:violations += $hits.Count
    }
}

Test-Pattern -Label "Raw button with btn-* class - use BCButton" `
    -Pattern 'button\s*\{[^}]*class:\s*"btn-(primary|secondary|danger|ghost|black)"'
Test-Pattern -Label "BCButton duplicates variant in class prop" `
    -Pattern 'BCButton[^}]*class:\s*"(btn-primary|btn-secondary|btn-danger|btn-ghost|btn-black)"'

if ($violations -gt 0) {
    Write-Host "Found $violations UI convention violation(s). See docs/ui/components.md"
    exit 1
}

Write-Host "Console UI button conventions OK"
