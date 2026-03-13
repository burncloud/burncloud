#!/usr/bin/env pwsh
#Requires -Version 7
<#
.SYNOPSIS
    Windows CI 全自动调试脚本 - 监控 GitHub Actions 并自动本地复现

.DESCRIPTION
    1. 推送代码到 GitHub
    2. 监控 CI 运行状态
    3. 失败时自动获取日志
    4. 在本地复现 Windows 测试环境
    5. 分析错误并提供修复建议

.EXAMPLE
    ./scripts/ci-debug-windows.ps1
    ./scripts/ci-debug-windows.ps1 -SkipPush  # 不推送，只监控
#>

param(
    [switch]$SkipPush,
    [string]$Branch = (git branch --show-current),
    [switch]$Verbose
)

$ErrorActionPreference = "Continue"
$Host.UI.RawUI.WindowTitle = "BurnCloud CI Debugger"

# ============================================================
# 配置
# ============================================================
$Config = @{
    SoftwareId     = "openclaw"
    MaxWaitMinutes = 30
    PollInterval   = 10  # 秒
    LogFile        = "$PSScriptRoot/../ci-debug.log"
}

# 颜色输出
function Write-Step { param($msg) Write-Host "`n🔧 $msg" -ForegroundColor Cyan }
function Write-Success { param($msg) Write-Host "✅ $msg" -ForegroundColor Green }
function Write-Fail { param($msg) Write-Host "❌ $msg" -ForegroundColor Red }
function Write-Info { param($msg) Write-Host "   $msg" -ForegroundColor Gray }
function Write-Highlight { param($msg) Write-Host "💡 $msg" -ForegroundColor Yellow }

# ============================================================
# Step 1: 推送代码
# ============================================================
function Push-Code {
    if ($SkipPush) {
        Write-Info "跳过推送，监控现有运行..."
        return
    }

    Write-Step "推送代码到 GitHub (分支: $Branch)"

    # 检查是否有未提交的更改
    $status = git status --porcelain
    if ($status) {
        Write-Fail "有未提交的更改，请先提交"
        Write-Host $status
        exit 1
    }

    git push origin $Branch 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "推送失败"
        exit 1
    }
    Write-Success "代码已推送"
}

# ============================================================
# Step 2: 等待 CI 开始
# ============================================================
function Wait-CIStart {
    Write-Step "等待 CI 开始运行..."

    $startTime = Get-Date
    $timeout = $Config.MaxWaitMinutes * 60

    while ($true) {
        $runs = gh run list --branch $Branch --limit 3 --json databaseId,status,conclusion,displayTitle,createdAt 2>$null | ConvertFrom-Json

        if ($runs -and $runs.Count -gt 0) {
            $latestRun = $runs[0]

            if ($latestRun.status -eq "in_progress" -or $latestRun.status -eq "queued") {
                Write-Success "CI 已启动: Run ID = $($latestRun.databaseId)"
                return $latestRun.databaseId
            }
        }

        $elapsed = (Get-Date) - $startTime
        if ($elapsed.TotalSeconds -gt $timeout) {
            Write-Fail "等待超时，CI 未启动"
            exit 1
        }

        Write-Info "等待中... ($([int]$elapsed.TotalSeconds)s)"
        Start-Sleep -Seconds $Config.PollInterval
    }
}

# ============================================================
# Step 3: 监控 CI 运行
# ============================================================
function Watch-CIRun {
    param($RunId)

    Write-Step "监控 CI 运行 (Run ID: $RunId)..."
    Write-Info "在线查看: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/actions/runs/$RunId"

    # 使用 gh run watch 实时显示
    gh run watch $RunId --exit-status 2>&1

    return $LASTEXITCODE
}

# ============================================================
# Step 4: 获取失败日志
# ============================================================
function Get-FailureLogs {
    param($RunId)

    Write-Step "获取失败日志..."

    # 获取失败的 job
    $jobs = gh run view $RunId --json jobs 2>$null | ConvertFrom-Json
    $failedJobs = $jobs.jobs | Where-Object { $_.conclusion -eq "failure" }

    $allLogs = @()

    foreach ($job in $failedJobs) {
        Write-Info "Job: $($job.name)"

        # 获取该 job 的日志
        $logFile = "$env:TEMP\ci-log-$($job.databaseId).txt"
        gh run view $RunId --log-failed --job $job.databaseId 2>$null | Out-File -FilePath $logFile -Encoding utf8

        $logContent = Get-Content $logFile -Raw
        $allLogs += @{
            JobName = $job.name
            Log     = $logContent
            File    = $logFile
        }

        Write-Info "  日志已保存: $logFile"
    }

    return $allLogs
}

# ============================================================
# Step 5: 分析错误
# ============================================================
function Analyze-Errors {
    param($Logs)

    Write-Step "分析错误原因..."

    $errorPatterns = @(
        @{ Pattern = "error\[E\d+\]"; Name = "编译错误"; Type = "Compile" },
        @{ Pattern = "panic!|thread.*panicked"; Name = "运行时崩溃"; Type = "Runtime" },
        @{ Pattern = "git.*not found|'git' is not recognized"; Name = "Git 未安装"; Type = "Dependency" },
        @{ Pattern = "node.*not found|'node' is not recognized"; Name = "Node.js 未安装"; Type = "Dependency" },
        @{ Pattern = "npm.*not found|'npm' is not recognized"; Name = "npm 未安装"; Type = "Dependency" },
        @{ Pattern = "permission denied|Access is denied"; Name = "权限问题"; Type = "Permission" },
        @{ Pattern = "network|timeout|ETIMEDOUT"; Name = "网络问题"; Type = "Network" },
        @{ Pattern = "checksum mismatch|sha256"; Name = "文件校验失败"; Type = "Bundle" },
        @{ Pattern = "file not found|cannot find"; Name = "文件缺失"; Type = "Bundle" }
    )

    $foundErrors = @()

    foreach ($logEntry in $Logs) {
        Write-Info "`n📋 Job: $($logEntry.JobName)"
        $logLines = $logEntry.Log -split "`n"

        foreach ($pattern in $errorPatterns) {
            $matches = $logLines | Select-String -Pattern $pattern.Pattern -AllMatches
            if ($matches) {
                foreach ($match in $matches | Select-Object -First 3) {
                    Write-Host "   🎯 $($pattern.Name): " -NoNewline
                    Write-Host $match.Line.Trim() -ForegroundColor Red
                    $foundErrors += @{
                        Type = $pattern.Type
                        Message = $match.Line.Trim()
                        Job = $logEntry.JobName
                    }
                }
            }
        }
    }

    return $foundErrors
}

# ============================================================
# Step 6: 本地复现
# ============================================================
function Invoke-LocalReproduce {
    Write-Step "本地复现 Windows 测试环境..."

    $projectRoot = Split-Path $PSScriptRoot -Parent
    Set-Location $projectRoot

    # 1. 构建 CLI
    Write-Info "构建 burncloud CLI..."
    cargo build --release --bin burncloud 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "构建失败"
        cargo build --release --bin burncloud
        return $false
    }
    Write-Success "CLI 构建完成"

    # 2. 创建 Bundle
    Write-Info "创建 bundle..."
    $bundleDir = Join-Path $env:TEMP "burncloud-test-bundles"
    & "./target/release/burncloud.exe" bundle create $Config.SoftwareId -o $bundleDir 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Fail "Bundle 创建失败"
        return $false
    }
    Write-Success "Bundle 创建完成"

    # 3. 清理环境变量（模拟新机器）
    Write-Info "清理环境变量（模拟新机器）..."

    # 保存当前 PATH
    $originalPath = $env:PATH

    # 移除常见的预装工具路径
    $pathsToRemove = @(
        "*\nodejs\*",
        "*\npm\*",
        "*\Git\*",
        "*\fnm\*",
        "*\.fnm\*"
    )

    $cleanPath = ($env:PATH -split ";" | Where-Object {
        $p = $_
        -not ($pathsToRemove | Where-Object { $p -like $_ })
    }) -join ";"

    # 4. 在干净环境中测试安装
    Write-Info "`n开始测试安装..."
    Write-Info "=" * 60

    $env:RUST_LOG = "debug"
    $env:PATH = $cleanPath

    # 先检查当前环境
    Write-Info "预装检查:"
    @("git", "node", "npm") | ForEach-Object {
        $cmd = $_
        try {
            $result = Get-Command $cmd -ErrorAction SilentlyContinue
            if ($result) {
                Write-Host "   ⚠️  $cmd 已安装: $($result.Source)" -ForegroundColor Yellow
            } else {
                Write-Host "   ✓  $cmd 未安装（符合预期）" -ForegroundColor Green
            }
        } catch {
            Write-Host "   ✓  $cmd 未安装（符合预期）" -ForegroundColor Green
        }
    }

    Write-Info "`n执行安装命令..."
    $installResult = & "./target/release/burncloud.exe" install $Config.SoftwareId --bundle "$bundleDir/$($Config.SoftwareId)-bundle" --auto-deps 2>&1

    # 恢复 PATH
    $env:PATH = $originalPath

    Write-Info "=" * 60
    Write-Info $installResult

    if ($LASTEXITCODE -eq 0) {
        Write-Success "本地复现测试通过！"
        return $true
    } else {
        Write-Fail "本地复现测试失败"
        return $false
    }
}

# ============================================================
# Step 7: 生成修复建议
# ============================================================
function Generate-FixSuggestions {
    param($Errors)

    Write-Step "生成修复建议..."

    $suggestions = @{
        "Compile"    = "检查代码语法，运行 'cargo clippy' 获取详细错误"
        "Runtime"    = "添加更多错误处理，检查空指针和边界条件"
        "Dependency" = "确保 bundle 包含所有依赖，或检查 --auto-deps 实现"
        "Permission" = "检查文件写入权限，可能需要管理员权限"
        "Network"    = "添加重试逻辑，或检查代理配置"
        "Bundle"     = "重新创建 bundle，确保文件完整性"
    }

    $uniqueTypes = $Errors | Select-Object -ExpandProperty Type -Unique

    foreach ($type in $uniqueTypes) {
        if ($suggestions[$type]) {
            Write-Highlight "[$type] $($suggestions[$type])"
        }
    }

    # 生成快速修复命令
    Write-Info "`n快捷修复命令:"
    Write-Host "  # 本地完整测试" -ForegroundColor Gray
    Write-Host "  ./scripts/ci-debug-windows.ps1 -SkipPush" -ForegroundColor White
    Write-Host ""
    Write-Host "  # 仅构建测试" -ForegroundColor Gray
    Write-Host "  cargo build --release --bin burncloud" -ForegroundColor White
    Write-Host ""
    Write-Host "  # 查看 CI 日志" -ForegroundColor Gray
    Write-Host "  gh run view --log-failed" -ForegroundColor White
}

# ============================================================
# 主流程
# ============================================================
function Main {
    $totalStart = Get-Date

    Write-Host @"

  ╔═══════════════════════════════════════════════════════════╗
  ║        BurnCloud Windows CI 全自动调试器                  ║
  ╚═══════════════════════════════════════════════════════════╝

"@ -ForegroundColor Cyan

    # 1. 推送代码
    Push-Code

    # 2. 等待 CI
    $runId = Wait-CIStart

    # 3. 监控运行
    $exitCode = Watch-CIRun -RunId $runId

    if ($exitCode -eq 0) {
        Write-Success "`n🎉 CI 测试通过！"
        return
    }

    # 4. CI 失败，开始调试
    Write-Fail "`nCI 测试失败，开始分析..."

    # 5. 获取日志
    $logs = Get-FailureLogs -RunId $runId

    # 6. 分析错误
    $errors = Analyze-Errors -Logs $logs

    # 7. 本地复现
    $localSuccess = Invoke-LocalReproduce

    # 8. 生成建议
    Generate-FixSuggestions -Errors $errors

    # 总结
    $totalTime = ((Get-Date) - $totalStart).TotalMinutes
    Write-Host "`n" + "=" * 60
    Write-Host "📊 调试完成 (耗时: $([int]$totalTime) 分钟)"
    Write-Host "=" * 60

    if (-not $localSuccess) {
        Write-Host "`n💡 提示: 修复后运行此脚本重新测试" -ForegroundColor Yellow
    }
}

# 运行
Main
