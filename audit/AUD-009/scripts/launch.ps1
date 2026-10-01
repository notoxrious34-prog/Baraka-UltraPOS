# AUD-008 scripts/launch.ps1 - start app isolated; record T0 plus milestones (a) handle, (b) TCP.
# Env vars live only in this process session; unset right after Start-Process.
# Returns as soon as both (a) and (b) are captured so the bash orchestrator can
# launch the CDP harness at ~T0 (before first paint). Children first-seen/count
# are captured by maximize-watch.ps1.
# Usage: launch.ps1 -Exe <path> -DataDir <dir> -Port <n> -RunIndex <i> [-OutDir <dir>]
param([Parameter(Mandatory=$true)][string]$Exe,
      [Parameter(Mandatory=$true)][string]$DataDir,
      [Parameter(Mandatory=$true)][int]$Port,
      [Parameter(Mandatory=$true)][int]$RunIndex,
      [string]$OutDir = 'D:\Dev\_scratch\AUD-008')

$env:BARAKA_DATA_DIR = $DataDir
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$p = Start-Process -FilePath $Exe -PassThru
$env:BARAKA_DATA_DIR = $null
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $null
$t0EpochMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $sw.ElapsedMilliseconds

$msA = $null; $msB = $null
$maxGap = 0; $lastTick = $sw.ElapsedMilliseconds
$deadlineMs = [int]($sw.ElapsedMilliseconds) + 60000
while ($sw.ElapsedMilliseconds -lt $deadlineMs) {
    $now = $sw.ElapsedMilliseconds
    $gap = $now - $lastTick; if ($gap -gt $maxGap) { $maxGap = $gap }; $lastTick = $now
    if ($null -eq $msA) { $p.Refresh(); if ($p.MainWindowHandle -ne 0) { $msA = $sw.ElapsedMilliseconds } }
    if ($null -eq $msB) {
        try { $c = New-Object System.Net.Sockets.TcpClient; $c.Connect('127.0.0.1', $Port); $c.Close(); $msB = $sw.ElapsedMilliseconds } catch {}
    }
    if ($null -ne $msA -and $null -ne $msB) { break }
    Start-Sleep -Milliseconds 20
}
$p.Refresh()
$o = [ordered]@{
    runIndex = $RunIndex; pid = $p.Id; t0EpochMs = $t0EpochMs; exited = $p.HasExited
    exitCode = $(if ($p.HasExited) { $p.ExitCode } else { $null })
    msHandle = $msA; msTcp = $msB; maxLoopGapMs = $maxGap
}
$launchJson = Join-Path $OutDir ("run{0}-launch.json" -f $RunIndex)
$o | ConvertTo-Json -Compress | Set-Content -LiteralPath $launchJson -Encoding UTF8
Write-Output (Get-Content -LiteralPath $launchJson -Raw)