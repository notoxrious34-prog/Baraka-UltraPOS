# AUD-008 scripts/maximize-watch.ps1 - self-maximize / icon / rect / foreground watch.
# Polls every $PollMs for $DurationMs; logs every CHANGE of the watched tuple plus initial+final.
# Usage: maximize-watch.ps1 -AppPid <pid> -OutFile <path> [-DurationMs 45000] [-PollMs 100]
param([Parameter(Mandatory=$true)][int]$AppPid,
      [Parameter(Mandatory=$true)][string]$OutFile,
      [long]$T0Epoch = -1,
      [int]$DurationMs = 50000,
      [int]$PollMs = 100)
$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public static class MW {
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
}
"@
Add-Type -TypeDefinition $code | Out-Null
$ctx = [MW]::SetProcessDpiAwarenessContext([IntPtr](-4))

$h = New-Object IntPtr 0
# find the app's window by enumerating processes
$procs = Get-Process -Id $AppPid -ErrorAction Stop
$handle = $procs.MainWindowHandle
# MainWindowHandle may be 0 until window is up; locate via Process.MainWindowHandle refresh
$deadline = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() + 15000
while ($handle -eq 0 -and [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() -lt $deadline) {
    Start-Sleep -Milliseconds 50
    $procs.Refresh(); $handle = $procs.MainWindowHandle
}
if ($handle -eq 0) { '{"error":"no hwnd"}' | Set-Content -LiteralPath $OutFile; Write-Output 'NO_HWND'; exit 2 }
$hwnd = [IntPtr]$handle

$lines = New-Object System.Collections.Generic.List[string]
if ($T0Epoch -le 0) { $t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() } else { $t0 = $T0Epoch }
$last = $null
# children tracking
$childFirstMs = $null; $childCount = 0; $childLastPoll = -999
function Children-Affinity([int]$appPid) {
    $web = Get-CimInstance Win32_Process -Filter "Name like 'msedgewebview2%'" -ErrorAction SilentlyContinue
    $n = 0
    foreach ($w in $web) {
        if ($w.ProcessId -eq 0) { continue }
        # walk ancestors
        $cp = $w.ParentProcessId; $guard = 0; $is = $false
        while ($cp -ne 0 -and $guard -lt 32) {
            if ($cp -eq $appPid) { $is = $true; break }
            $pa = Get-CimInstance Win32_Process -Filter "ProcessId = $cp" -ErrorAction SilentlyContinue
            if ($null -eq $pa) { break }
            $cp = $pa.ParentProcessId; $guard++
        }
        if ($is) { $n++ }
    }
    return $n
}
function Snap([string]$when) {
    $r = New-Object RECT; [MW]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $z = [MW]::IsZoomed($hwnd); $ic = [MW]::IsIconic($hwnd)
    $fg = [MW]::GetForegroundWindow()
    $fgTxt = ""; $sb = New-Object System.Text.StringBuilder 128
    if ($fg -ne [IntPtr]::Zero) { [MW]::GetWindowText($fg, $sb, 128) | Out-Null; $fgTxt = $sb.ToString() }
    $fgPid = 0; [MW]::GetWindowThreadProcessId($fg, [ref]$fgPid) | Out-Null
    return [pscustomobject]@{ t = $when; ms = ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $t0); zoomed = $z; iconic = $ic; rect = "$($r.Left),$($r.Top),$($r.Right),$($r.Bottom)"; fg = $fgTxt; fgIsApp = ($fg -eq $hwnd) }
}
$end = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() + $DurationMs
$count = 0
while ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() -lt $end) {
    $s = Snap 's'
    if ($null -eq $last) { $lines.Add(("INIT " + ($s | ConvertTo-Json -Compress))) }
    else {
        $sig = "$($s.zoomed)|$($s.iconic)|$($s.rect)"
        $lastSig = "$($last.zoomed)|$($last.iconic)|$($last.rect)"
        if ($sig -ne $lastSig -or $s.fgIsApp -ne $last.fgIsApp) { $lines.Add(("CHANGE " + ($s | ConvertTo-Json -Compress))) }
    }
    $last = $s; $count++
    # children first-seen within first 15 s (300 ms cadence)
    $nowMs = ([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $t0)
    if ($nowMs -lt 15000 -and ($nowMs - $childLastPoll) -ge 300) {
        $childLastPoll = $nowMs
        $n = Children-Affinity $AppPid
        if ($n -gt 0 -and $null -eq $childFirstMs) { $childFirstMs = $nowMs }
        if ($n -gt $childCount) { $childCount = $n }
    }
    Start-Sleep -Milliseconds $PollMs
}
$final = Snap 'f'; $lines.Add(("FINAL " + ($final | ConvertTo-Json -Compress)))
$res = [ordered]@{ samples = $count; durationMs = $DurationMs; hwnd = $hwnd; childFirstMs = $childFirstMs; childCountMax15s = $childCount; lines = $lines }
$res | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $OutFile -Encoding UTF8
Write-Output ("DONE samples=$count")