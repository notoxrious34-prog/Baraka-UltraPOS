# AUD-007 scripts/launch.ps1 — start the app isolated, print PID + T0 + milestone (a).
# Env vars exist ONLY in this process session; unset right after Start-Process.
# Usage: launch.ps1 -Exe <path> -DataDir <dir> -Port <n>
param([Parameter(Mandatory = $true)][string]$Exe,
      [Parameter(Mandatory = $true)][string]$DataDir,
      [Parameter(Mandatory = $true)][int]$Port)
$env:BARAKA_DATA_DIR = $DataDir
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
$p = Start-Process -FilePath $Exe -PassThru
$env:BARAKA_DATA_DIR = $null
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $null
$msA = $null
$deadline = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() + 60000
while ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() -lt $deadline) {
    $p.Refresh()
    if ($p.HasExited) { break }
    if ($p.MainWindowHandle -ne 0) { $msA = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $t0; break }
    Start-Sleep -Milliseconds 50
}
$p.Refresh()
Write-Output (@{ pid = $p.Id; t0 = $t0; exited = $p.HasExited; exitCode = $(if ($p.HasExited) { $p.ExitCode } else { $null }); msHandle = $msA } | ConvertTo-Json -Compress)
