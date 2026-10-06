param(
    [string]$Repository = "burncloud/burncloud",
    [string]$BaseBranch = "main",
    [int]$MaxPullRequests = 100
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$ArgumentList,
        [string]$WorkingDirectory = (Get-Location).Path
    )

    & $FilePath @ArgumentList
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed ($LASTEXITCODE): $FilePath $($ArgumentList -join ' ')"
    }
}

function Set-CommitStatus {
    param(
        [Parameter(Mandatory = $true)][string]$Sha,
        [Parameter(Mandatory = $true)][ValidateSet("pending", "success", "failure", "error")][string]$State,
        [Parameter(Mandatory = $true)][string]$Description,
        [Parameter(Mandatory = $true)][string]$TargetUrl
    )

    try {
        & gh api --method POST "repos/$Repository/statuses/$Sha" `
            -f "state=$State" `
            -f "context=burncloud/code-test" `
            -f "description=$Description" `
            -f "target_url=$TargetUrl" | Out-Null
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "Could not publish GitHub commit status for $Sha."
        }
    }
    catch {
        Write-Warning "Could not publish GitHub commit status for $Sha`: $($_.Exception.Message)"
    }
}

$repoRoot = (& git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($repoRoot)) {
    throw "Run this script from inside a burncloud Git checkout."
}

$gitDirRaw = (& git -C $repoRoot rev-parse --git-dir).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Unable to locate the repository .git directory."
}
$gitDir = if ([System.IO.Path]::IsPathRooted($gitDirRaw)) {
    $gitDirRaw
} else {
    [System.IO.Path]::GetFullPath((Join-Path $repoRoot $gitDirRaw))
}

$stateDir = Join-Path $gitDir "burncloud/pr-code-test"
$worktreeRoot = Join-Path $stateDir "worktrees"
$logsDir = Join-Path $stateDir "logs"
$statePath = Join-Path $stateDir "state.json"
$lockPath = Join-Path $stateDir "runner.lock"
New-Item -ItemType Directory -Force -Path $worktreeRoot, $logsDir | Out-Null

$lockStream = $null
try {
    try {
        $lockStream = [System.IO.File]::Open($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    }
    catch [System.IO.IOException] {
        Write-Host "Another PR code-test runner is already active; exiting."
        exit 0
    }

    foreach ($tool in @("git", "gh", "cargo")) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "Required command '$tool' is not available in PATH."
        }
    }

    Invoke-Checked -FilePath "gh" -ArgumentList @("auth", "status") -WorkingDirectory $repoRoot
    Invoke-Checked -FilePath "git" -ArgumentList @("fetch", "--prune", "origin", $BaseBranch) -WorkingDirectory $repoRoot

    $state = @{}
    if (Test-Path $statePath) {
        $raw = Get-Content -Raw -Path $statePath
        if (-not [string]::IsNullOrWhiteSpace($raw)) {
            $saved = $raw | ConvertFrom-Json -AsHashtable
            if ($saved) {
                $state = $saved
            }
        }
    }

    $prJson = & gh pr list --repo $Repository --state open --limit $MaxPullRequests --json number,headRefOid,url,title,isDraft
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to list open pull requests from $Repository."
    }
    $pullRequests = @($prJson | ConvertFrom-Json)

    foreach ($pr in $pullRequests) {
        $prNumber = [int]$pr.number
        $sha = [string]$pr.headRefOid
        $url = [string]$pr.url

        if ([string]::IsNullOrWhiteSpace($sha)) {
            Write-Warning "PR #$prNumber has no head SHA; skipping."
            continue
        }

        if ($state.ContainsKey($sha) -and $state[$sha].status -in @("success", "failure")) {
            Write-Host "PR #$prNumber $sha already tested: $($state[$sha].status)."
            continue
        }

        $shortSha = $sha.Substring(0, [Math]::Min(12, $sha.Length))
        $worktree = Join-Path $worktreeRoot "pr-$prNumber-$shortSha"
        $logPath = Join-Path $logsDir "pr-$prNumber-$shortSha.log"

        Write-Host "Testing PR #$prNumber at $sha"
        Set-CommitStatus -Sha $sha -State "pending" -Description "cargo run -- code test is running" -TargetUrl $url

        $record = @{
            pr = $prNumber
            sha = $sha
            status = "running"
            started_at = (Get-Date).ToUniversalTime().ToString("o")
            log = $logPath
        }
        $state[$sha] = $record
        $state | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 -Path $statePath

        $result = "error"
        try {
            Invoke-Checked -FilePath "git" -ArgumentList @("fetch", "origin", "+refs/pull/$prNumber/head:refs/remotes/origin/pr/$prNumber") -WorkingDirectory $repoRoot

            if (Test-Path $worktree) {
                Invoke-Checked -FilePath "git" -ArgumentList @("worktree", "remove", "--force", $worktree) -WorkingDirectory $repoRoot
            }
            Invoke-Checked -FilePath "git" -ArgumentList @("worktree", "add", "--detach", $worktree, $sha) -WorkingDirectory $repoRoot

            Push-Location $worktree
            try {
                & cargo run -- code test --base "origin/$BaseBranch" 2>&1 | Tee-Object -FilePath $logPath
                $exitCode = $LASTEXITCODE
            }
            finally {
                Pop-Location
            }

            if ($exitCode -eq 0) {
                $result = "success"
                Set-CommitStatus -Sha $sha -State "success" -Description "cargo run -- code test passed" -TargetUrl $url
                Write-Host "PR #$prNumber passed code test."
            }
            else {
                $result = "failure"
                Set-CommitStatus -Sha $sha -State "failure" -Description "cargo run -- code test failed" -TargetUrl $url
                Write-Error "PR #$prNumber failed code test. Log: $logPath" -ErrorAction Continue
            }
        }
        catch {
            $result = "error"
            Set-CommitStatus -Sha $sha -State "error" -Description "PR code-test runner failed" -TargetUrl $url
            "Runner error: $($_.Exception.Message)" | Add-Content -Encoding UTF8 -Path $logPath
            Write-Error "PR #$prNumber runner error: $($_.Exception.Message)" -ErrorAction Continue
        }
        finally {
            if (Test-Path $worktree) {
                try {
                    Invoke-Checked -FilePath "git" -ArgumentList @("worktree", "remove", "--force", $worktree) -WorkingDirectory $repoRoot
                }
                catch {
                    Write-Warning "Could not remove worktree $worktree`: $($_.Exception.Message)"
                }
            }
            & git -C $repoRoot worktree prune | Out-Null

            $record.status = $result
            $record.finished_at = (Get-Date).ToUniversalTime().ToString("o")
            $state[$sha] = $record
            $state | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 -Path $statePath
        }
    }
}
finally {
    if ($lockStream) {
        $lockStream.Dispose()
    }
}
