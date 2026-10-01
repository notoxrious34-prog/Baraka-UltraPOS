# AUD-007 meta-list.ps1 — METADATA ONLY listing of a directory.
# Emits relative path, Length, LastWriteTimeUtc ticks per file; NEVER file contents.
# Usage: powershell -NoProfile -File meta-list.ps1 <DirPath>
param([Parameter(Mandatory = $true)][string]$Dir)
if (-not (Test-Path $Dir)) { Write-Output "DIR_MISSING $Dir"; exit 0 }
$root = (Resolve-Path $Dir).Path
$files = Get-ChildItem -LiteralPath $root -Recurse -Force -File | Sort-Object FullName
Write-Output ("ENTRY_COUNT " + $files.Count)
$lines = @()
foreach ($f in $files) {
    $rel = $f.FullName.Substring($root.Length)
    $line = "$rel | $($f.Length) | $($f.LastWriteTimeUtc.Ticks)"
    $lines += $line
    Write-Output $line
}
$joined = [string]::Join("`n", $lines)
$bytes = [System.Text.Encoding]::UTF8.GetBytes($joined)
$sha = [System.Security.Cryptography.SHA256]::Create()
$fp = -join ($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString("x2") })
Write-Output ("FINGERPRINT " + $fp)
