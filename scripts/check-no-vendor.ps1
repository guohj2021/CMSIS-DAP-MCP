# Content-policy scan for terms the repository must not contain.
#
# The needles are assembled from fragments so this file is excluded from its
# own scan.
#
# Checks plaintext, base64, UTF-16LE and hex forms over tracked and
# untracked-but-unignored files, and over all git history when -History is
# passed.
#
# Usage:
#   powershell -File scripts/check-no-vendor.ps1            # working tree
#   powershell -File scripts/check-no-vendor.ps1 -History   # + all git history
#   powershell -File scripts/check-no-vendor.ps1 -Path <f>  # specific file(s)

[CmdletBinding()]
param(
  # Scan these files instead of the repository file list (also scans ignored
  # files, which is useful when verifying a local artifact).
  [string[]]$Path,
  # Also scan every commit message, tag message and git object.
  [switch]$History
)

$ErrorActionPreference = 'Stop'

$needles = @(
  ('ALB{0}' -f '32'),
  ('ALBS{0}' -f 'EMI'),
  ('Bai{0}' -f 'he'),
  ('XV-{0}' -f 'Link')
)

$hexNeedles = @{}
foreach ($n in $needles) {
  $hexNeedles[$n] = (([System.Text.Encoding]::UTF8.GetBytes($n)) |
    ForEach-Object { $_.ToString('x2') }) -join ''
}

# Latin-1 maps every byte to exactly one char, so regex/IndexOf hits stay
# byte-accurate and binary files can be scanned without decode errors.
$Latin1 = [System.Text.Encoding]::GetEncoding(28591)
$Base64Run = [regex]'[A-Za-z0-9+/]{20,}={0,2}'

function Test-Content([byte[]]$Bytes) {
  $hits = New-Object System.Collections.Generic.List[string]
  if ($Bytes.Length -eq 0) { return $hits }

  $text = $Latin1.GetString($Bytes)
  # Both UTF-16LE byte phases: a wide-encoded needle at an odd byte offset is
  # misaligned when the whole buffer is decoded from offset 0.
  $utf16 = New-Object System.Collections.Generic.List[string]
  $utf16.Add([System.Text.Encoding]::Unicode.GetString($Bytes))
  if ($Bytes.Length -gt 1) {
    $utf16.Add([System.Text.Encoding]::Unicode.GetString($Bytes, 1, $Bytes.Length - 1))
  }

  foreach ($n in $needles) {
    if ($text.IndexOf($n, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
      $hits.Add("plaintext '$n'")
    }
    foreach ($wide in $utf16) {
      if ($wide.IndexOf($n, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
        $hits.Add("utf16le '$n'")
        break
      }
    }
    if ($text.IndexOf($hexNeedles[$n], [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
      $hits.Add("hex '$n'")
    }
  }

  # Decode every base64-looking run and rescan the bytes it yields; this catches
  # the needle at any byte offset, which a search for base64(needle) would miss.
  foreach ($m in $Base64Run.Matches($text)) {
    $run = $m.Value
    try {
      $decoded = [Convert]::FromBase64String($run + ('=' * ((4 - $run.Length % 4) % 4)))
    } catch {
      continue
    }
    $decodedText = $Latin1.GetString($decoded)
    foreach ($n in $needles) {
      if ($decodedText.IndexOf($n, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
        $hits.Add("base64 '$n'")
      }
    }
  }

  return $hits
}

function Get-ScanFiles {
  if ($Path) {
    return $Path | ForEach-Object { (Resolve-Path -LiteralPath $_).Path }
  }
  Push-Location $script:Root
  try {
    $files = & git ls-files --cached --others --exclude-standard
    if ($LASTEXITCODE -ne 0) { throw 'git ls-files failed' }
  } finally {
    Pop-Location
  }
  return $files | Where-Object { $_ -and $_ -ne 'scripts/check-no-vendor.ps1' }
}

$script:Root = Split-Path -Parent $PSScriptRoot
$findings = New-Object System.Collections.Generic.List[string]
$scanned = 0

foreach ($rel in (Get-ScanFiles)) {
  $full = if ([System.IO.Path]::IsPathRooted($rel)) { $rel } else { Join-Path $script:Root $rel }
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
  $scanned++
  foreach ($hit in (Test-Content ([System.IO.File]::ReadAllBytes($full)))) {
    $findings.Add("$rel : $hit")
  }
}

if ($History) {
  Push-Location $script:Root
  try {
    $messages = & git log --all --format='%H%n%B' 2>$null
    $messages += & git tag -l --format='%(refname) %(contents)' 2>$null
    $objectDump = [System.IO.Path]::GetTempFileName()
    try {
      # Start-Process redirects raw bytes; PowerShell's `>` would transcode the
      # object dump (UTF-16LE under Windows PowerShell) and corrupt the scan.
      $proc = Start-Process -FilePath 'git' `
        -ArgumentList @('cat-file', '--batch-all-objects', '--batch') `
        -RedirectStandardOutput $objectDump -NoNewWindow -Wait -PassThru
      if ($proc.ExitCode -ne 0) { throw "git cat-file failed ($($proc.ExitCode))" }
      foreach ($hit in (Test-Content ([System.IO.File]::ReadAllBytes($objectDump)))) {
        $findings.Add("history/objects : $hit")
      }
    } finally {
      Remove-Item -LiteralPath $objectDump -Force -ErrorAction SilentlyContinue
    }
    foreach ($hit in (Test-Content ([System.Text.Encoding]::UTF8.GetBytes(($messages -join "`n"))))) {
      $findings.Add("history/messages : $hit")
    }
  } finally {
    Pop-Location
  }
}

if ($findings.Count -gt 0) {
  [Console]::Error.WriteLine('vendor-specific content found ({0} hit(s)):' -f $findings.Count)
  foreach ($finding in $findings) { [Console]::Error.WriteLine('  ' + $finding) }
  exit 1
}
Write-Output "no vendor-specific content ($scanned file(s) scanned$(if ($History) { ', history included' }))"
