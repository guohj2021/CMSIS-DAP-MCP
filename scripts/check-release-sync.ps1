# Pre-release sync check (lightweight Git Flow).
#
# Verifies, before tagging a release:
#   1. the working tree is clean,
#   2. local develop matches origin/develop,
#   3. origin/main == origin/develop (release requirement),
#   4. no stale merged remote branches remain (they should be deleted).
#
# Usage:  powershell -File scripts/check-release-sync.ps1
$ErrorActionPreference = "Stop"

git fetch origin --prune
if ($LASTEXITCODE -ne 0) { throw "git fetch failed" }

function Sha([string]$ref) { return (git rev-parse $ref).Trim() }

Write-Host "== 1. working tree ==" -ForegroundColor Cyan
$st = git status --porcelain
if ($st) {
    Write-Host "WARN: working tree is not clean:" -ForegroundColor Yellow
    $st | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
} else {
    Write-Host "OK: working tree clean" -ForegroundColor Green
}

Write-Host "`n== 2. local develop vs origin/develop ==" -ForegroundColor Cyan
$devLocal = Sha "develop"
$devRemote = Sha "origin/develop"
Write-Host "develop      = $devLocal"
Write-Host "origin/develop = $devRemote"
if ($devLocal -ne $devRemote) {
    Write-Host "WARN: local develop differs from origin/develop; pull first." -ForegroundColor Yellow
} else {
    Write-Host "OK: local develop is in sync" -ForegroundColor Green
}

Write-Host "`n== 3. main and develop content in sync (release requirement) ==" -ForegroundColor Cyan
$mainTree = (git rev-parse "origin/main^{tree}").Trim()
$devTree  = (git rev-parse "origin/develop^{tree}").Trim()
Write-Host "origin/main tree    = $mainTree"
Write-Host "origin/develop tree = $devTree"
if ($mainTree -ne $devTree) {
    Write-Host "FAIL: main and develop do not have identical content." -ForegroundColor Red
    Write-Host "      Merge develop into main (PR develop -> main) before tagging a release." -ForegroundColor Red
    exit 1
}
Write-Host "OK: main and develop are in sync (identical content)" -ForegroundColor Green

Write-Host "`n== 4. stale merged branches ==" -ForegroundColor Cyan
$merged = git branch -r --merged origin/develop | ForEach-Object { $_.Trim() } |
    Where-Object { $_ -notmatch '^origin/(develop|main|HEAD)$' -and $_ -notmatch '->' }
if ($merged) {
    Write-Host "Merged remote branches that should be deleted:" -ForegroundColor Yellow
    $merged | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
    Write-Host "Delete them with:  git push origin --delete <branch>" -ForegroundColor DarkYellow
} else {
    Write-Host "OK: no stale merged branches" -ForegroundColor Green
}

Write-Host "`nRelease sync check passed. Safe to bump the version and tag vX.Y.Z." -ForegroundColor Green
