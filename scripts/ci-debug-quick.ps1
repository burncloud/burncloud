#!/usr/bin/env pwsh
# ci-debug-quick.ps1 - 快速版 CI 调试脚本
# 用法: ./scripts/ci-debug-quick.ps1

param([switch]$SkipPush)

Write-Host "🚀 BurnCloud CI 快速调试" -ForegroundColor Cyan

# 1. 推送
if (-not $SkipPush) {
    Write-Host "📦 推送代码..." -NoNewline
    git push 2>$null
    if ($LASTEXITCODE -eq 0) { Write-Host " ✓" -ForegroundColor Green }
}

# 2. 监控 CI
Write-Host "👀 监控 CI..." -ForegroundColor Yellow
gh run watch --exit-status

# 3. 结果
if ($LASTEXITCODE -eq 0) {
    Write-Host "`n✅ CI 通过！" -ForegroundColor Green
    exit 0
}

# 4. 失败 - 获取日志
Write-Host "`n❌ CI 失败，获取日志..." -ForegroundColor Red
gh run view --log-failed

Write-Host "`n💡 本地复现命令:" -ForegroundColor Yellow
Write-Host "   ./scripts/ci-debug-windows.ps1 -SkipPush" -ForegroundColor White
