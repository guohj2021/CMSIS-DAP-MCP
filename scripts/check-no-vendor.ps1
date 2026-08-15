$root = Split-Path -Parent $PSScriptRoot
$pattern = 'ALB{0}|ALBS{1}' -f '32', 'EMI'
$hits = rg -i $pattern $root --glob '!target/**' --glob '!.git/**' --glob '!.superpowers/**' --glob '!scripts/check-no-vendor.ps1' 2>$null
if ($LASTEXITCODE -eq 0) {
  Write-Error "vendor-specific content found:`n$hits"
  exit 1
}
Write-Output "no vendor-specific content"