param(
    [string]$Repository = "burncloud/burncloud",
    [string]$BaseBranch = "main",
    [int]$IntervalMinutes = 1,
    [string]$TaskName = "BurnCloud PR Code Test"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

if ($IntervalMinutes -lt 1) {
    throw "IntervalMinutes must be at least 1."
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this script from inside a burncloud Git checkout."
}

$runner = Join-Path $repoRoot ".github/scripts/pr-code-test.ps1"
if (-not (Test-Path $runner)) {
    throw "Runner script not found: $runner"
}

foreach ($tool in @("git", "gh", "cargo", "rustup")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Required command '$tool' is not available in PATH."
    }
}

& gh auth status
if ($LASTEXITCODE -ne 0) {
    throw "GitHub CLI is not authenticated. Run: gh auth login"
}

& rustup component add rustfmt clippy
if ($LASTEXITCODE -ne 0) {
    throw "Unable to install/verify rustfmt and Clippy."
}

& cargo deny --version *> $null
if ($LASTEXITCODE -ne 0) {
    Write-Host "cargo-deny is missing; installing it..."
    & cargo install --locked cargo-deny
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to install cargo-deny."
    }
}

$powerShell = (Get-Command powershell.exe -ErrorAction Stop).Source
$arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$runner`" -Repository `"$Repository`" -BaseBranch `"$BaseBranch`""
$action = New-ScheduledTaskAction -Execute $powerShell -Argument $arguments -WorkingDirectory $repoRoot
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date).AddMinutes(1) -RepetitionInterval (New-TimeSpan -Minutes $IntervalMinutes)
$settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 6)

$existing = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($existing) {
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
}

Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Settings $settings `
    -Description "Poll $Repository pull requests and run cargo run -- code test on each new PR head SHA." | Out-Null

Write-Host "Installed scheduled task: $TaskName"
Write-Host "Interval: every $IntervalMinutes minute(s)"
Write-Host "Repository: $Repository"
Write-Host "Working directory: $repoRoot"
Write-Host "Runner: $runner"
Write-Host ""
Write-Host "Run once now with:"
Write-Host "  powershell -ExecutionPolicy Bypass -File `"$runner`""
Write-Host ""
Write-Host "Inspect task with:"
Write-Host "  Get-ScheduledTask -TaskName `"$TaskName`""
