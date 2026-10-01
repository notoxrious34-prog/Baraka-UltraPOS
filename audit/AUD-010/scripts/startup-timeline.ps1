# AUD-009 scripts/startup-timeline.ps1 - launches the exe itself and samples the tree.
# T0 = stopwatch started immediately before process creation. Samples every ~20 ms:
# top-level windows of the process tree (hwnd/class/title/visible/zoomed/iconic/
# rect/DWM frame/DWM cloaked), exe CPU/workingset/threads/IO/handles, child
# processes (name/pid/first-seen/CPU), TCP 9222 (every ~250 ms when port given).
# Usage: startup-timeline.ps1 -Exe <path> -DataDir <dir> -OutPrefix <scratch\runN>
#        [-Port <n>] [-DurationSec <s>] [-SampleMs <ms>]
param([Parameter(Mandatory=$true)][string]$Exe,
      [Parameter(Mandatory=$true)][string]$DataDir,
      [Parameter(Mandatory=$true)][string]$OutPrefix,
      [int]$Port = 0,
      [int]$DurationSec = 45,
      [int]$SampleMs = 20)

$code = @"
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Text;

[StructLayout(LayoutKind.Sequential)] public struct R4 { public int L, T, R, B; }
[StructLayout(LayoutKind.Sequential, CharSet = CharSet.Auto)] public struct PE32 {
  public uint dwSize; public uint cntUsage; public uint th32ProcessID; public IntPtr th32DefaultHeapID;
  public uint th32ModuleID; public uint cntThreads; public uint th32ParentProcessID;
  public int pcPriClassBase; public uint dwFlags;
  [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string szExeFile;
}
[StructLayout(LayoutKind.Sequential)] public struct IOCT { public ulong ReadOp, WrittenOp, OtherOp, ReadBytes, WrittenBytes, OtherBytes; }

public static class ST9 {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc e, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R4 r);
  [DllImport("dwmapi.dll", EntryPoint = "DwmGetWindowAttribute")] public static extern int DwmRect(IntPtr h, int a, out R4 r, int s);
  [DllImport("dwmapi.dll", EntryPoint = "DwmGetWindowAttribute")] public static extern int DwmInt(IntPtr h, int a, out int v, int s);
  [DllImport("kernel32.dll")] public static extern IntPtr CreateToolhelp32Snapshot(uint f, uint pid);
  [DllImport("kernel32.dll")] public static extern bool Process32First(IntPtr h, ref PE32 e);
  [DllImport("kernel32.dll")] public static extern bool Process32Next(IntPtr h, ref PE32 e);
  [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
  [DllImport("kernel32.dll")] public static extern bool GetProcessIoCounters(IntPtr h, out IOCT io);
  [DllImport("kernel32.dll")] public static extern bool GetProcessTimes(IntPtr h, out long c, out long x, out long k, out long u);

  static Dictionary<int,int> tree = new Dictionary<int,int>();
  static List<object[]> wins = new List<object[]>();

  static string J(string s) {
    if (s == null) return "";
    return s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", "\\r").Replace("\n", "\\n").Replace("\t", "\\t");
  }

  public static string Snap(int appPid, bool doTcp, int port) {
    StringBuilder sb = new StringBuilder(8192);
    // process tree via toolhelp
    Dictionary<int,int> ppid = new Dictionary<int,int>();
    Dictionary<int,string> names = new Dictionary<int,string>();
    IntPtr snap = CreateToolhelp32Snapshot(0x2, 0);
    if (snap.ToInt64() != -1) {
      PE32 pe = new PE32(); pe.dwSize = (uint)Marshal.SizeOf(typeof(PE32));
      if (Process32First(snap, ref pe)) {
        do { ppid[(int)pe.th32ProcessID] = (int)pe.th32ParentProcessID; names[(int)pe.th32ProcessID] = pe.szExeFile; }
        while (Process32Next(snap, ref pe));
      }
      CloseHandle(snap);
    }
    HashSet<int> members = new HashSet<int>(); members.Add(appPid);
    bool grew = true; int guard = 0;
    while (grew && guard++ < 40) { grew = false;
      foreach (var kv in ppid) { if (!members.Contains(kv.Key) && members.Contains(kv.Value)) { members.Add(kv.Key); grew = true; } } }
    // per-process CPU + IO
    sb.Append("\"procs\":[");
    bool first = true;
    foreach (int pid in members) {
      string nm = names.ContainsKey(pid) ? names[pid] : "?";
      long cpuMs = -1; ulong rB = 0, wB = 0; long ws = -1; int th = -1; int hd = -1;
      try {
        Process p = Process.GetProcessById(pid); p.Refresh();
        cpuMs = (long)(p.TotalProcessorTime.TotalMilliseconds);
        ws = p.WorkingSet64; th = p.Threads.Count; hd = p.HandleCount;
        IOCT io; if (GetProcessIoCounters(p.Handle, out io)) { rB = io.ReadBytes; wB = io.WrittenBytes; }
      } catch {}
      if (!first) sb.Append(",");
      first = false;
      sb.Append("{\"pid\":").Append(pid).Append(",\"name\":\"").Append(J(nm)).Append("\",\"cpuMs\":").Append(cpuMs)
        .Append(",\"ws\":").Append(ws).Append(",\"threads\":").Append(th).Append(",\"handles\":").Append(hd)
        .Append(",\"ioR\":").Append(rB).Append(",\"ioW\":").Append(wB).Append("}");
    }
    sb.Append("],");
    // top-level windows of members
    tree.Clear(); foreach (int m in members) tree[m] = 1;
    wins.Clear();
    EnumWindows(delegate(IntPtr h, IntPtr l) {
      uint wp; GetWindowThreadProcessId(h, out wp);
      if (tree.ContainsKey((int)wp)) {
        StringBuilder cn = new StringBuilder(128); GetClassName(h, cn, 128);
        StringBuilder tt = new StringBuilder(256); GetWindowText(h, tt, 256);
        R4 r; GetWindowRect(h, out r);
        R4 fr = new R4(); int hr = DwmRect(h, 9, out fr, 16);
        int cloak = 0; DwmInt(h, 14, out cloak, 4);
        wins.Add(new object[] { h.ToInt64(), (int)wp, cn.ToString(), tt.ToString(),
          IsWindowVisible(h), IsZoomed(h), IsIconic(h),
          r.L, r.T, r.R, r.B, hr, fr.L, fr.T, fr.R, fr.B, cloak });
      }
      return true;
    }, IntPtr.Zero);
    sb.Append("\"wins\":[");
    for (int i = 0; i < wins.Count; i++) {
      object[] w = wins[i];
      if (i > 0) sb.Append(",");
      sb.Append("{\"hwnd\":").Append(w[0]).Append(",\"pid\":").Append(w[1])
        .Append(",\"cls\":\"").Append(J((string)w[2])).Append("\",\"title\":\"").Append(J((string)w[3])).Append("\"")
        .Append(",\"vis\":").Append(((bool)w[4]) ? "true" : "false")
        .Append(",\"zoom\":").Append(((bool)w[5]) ? "true" : "false")
        .Append(",\"icon\":").Append(((bool)w[6]) ? "true" : "false")
        .Append(",\"rect\":[").Append(w[7]).Append(",").Append(w[8]).Append(",").Append(w[9]).Append(",").Append(w[10]).Append("]")
        .Append(",\"dwmHr\":").Append(w[11])
        .Append(",\"dwm\":[").Append(w[12]).Append(",").Append(w[13]).Append(",").Append(w[14]).Append(",").Append(w[15]).Append("]")
        .Append(",\"cloak\":").Append(w[16]).Append("}");
    }
    sb.Append("],");
    // TCP check (cadenced by caller)
    bool tcp = false;
    if (doTcp) {
      try {
        TcpClient c = new TcpClient();
        IAsyncResult ar = c.BeginConnect("127.0.0.1", port, null, null);
        tcp = ar.AsyncWaitHandle.WaitOne(60);
        c.Close();
      } catch { tcp = false; }
    }
    sb.Append("\"tcp\":").Append(tcp ? "true" : "false");
    return sb.ToString();
  }
}
"@
Add-Type -TypeDefinition $code | Out-Null

$env:BARAKA_DATA_DIR = $DataDir
if ($Port -gt 0) { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$Port" }
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$utcAnchor = [DateTime]::UtcNow.ToString('o')
$p = Start-Process -FilePath $Exe -PassThru
$env:BARAKA_DATA_DIR = $null
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $null
$t0EpochMs = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $sw.ElapsedMilliseconds
[System.IO.File]::WriteAllText("$OutPrefix-launch.json", (@{ pid = $p.Id; t0EpochMs = $t0EpochMs; utcAnchor = $utcAnchor } | ConvertTo-Json -Compress), (New-Object System.Text.UTF8Encoding($false)))

$samplesPath = "$OutPrefix-samples.jsonl"
$eventsPath = "$OutPrefix-events.jsonl"
$summaryPath = "$OutPrefix-summary.json"
if (Test-Path $samplesPath) { Remove-Item $samplesPath -Force }

$evt = New-Object System.Collections.Generic.List[string]
$lastSig = @{}
$firstSeen = @{}
$tcpOpen = $null
$gapMax = 0; $gapSum = 0; $n = 0; $gaps = New-Object System.Collections.Generic.List[int]
$endMs = $DurationSec * 1000
$lastTcp = -9999
while ($sw.ElapsedMilliseconds -lt $endMs) {
    $loopStart = $sw.ElapsedMilliseconds
    $doTcp = ($Port -gt 0) -and (($loopStart - $lastTcp) -ge 250)
    if ($doTcp) { $lastTcp = $loopStart }
    try { $body = [ST9]::Snap($p.Id, $doTcp, $Port) }
    catch { $body = '"procs":[],"wins":[],"snapErr":true,"tcp":false' }
    $t = $sw.ElapsedMilliseconds
    $line = ('{"t":' + $t + ',' + $body + '}')
    [System.IO.File]::AppendAllText($samplesPath, $line + "`n", (New-Object System.Text.UTF8Encoding($false)))
    # lightweight event detection on raw text signatures per hwnd/title/state
    try {
        $o = $line | ConvertFrom-Json
        foreach ($w in $o.wins) {
            $k = "$($w.hwnd)"
            $sig = "$($w.title)|$($w.vis)|$($w.zoom)|$($w.icon)|$($w.rect -join ',')|$($w.cloak)"
            if (-not $lastSig.ContainsKey($k)) {
                $firstSeen[$k] = $t
                $evt.Add(('{"t":' + $t + ',"ev":"win-created","hwnd":' + $k + ',"title":' + ($w.title | ConvertTo-Json -Compress) + ',"vis":' + ($w.vis.ToString().ToLower()) + ',"zoom":' + ($w.zoom.ToString().ToLower()) + ',"rect":[' + ($w.rect -join ',') + ']}'))
            } elseif ($lastSig[$k] -ne $sig) {
                $evt.Add(('{"t":' + $t + ',"ev":"win-changed","hwnd":' + $k + ',"title":' + ($w.title | ConvertTo-Json -Compress) + ',"vis":' + ($w.vis.ToString().ToLower()) + ',"zoom":' + ($w.zoom.ToString().ToLower()) + ',"rect":[' + ($w.rect -join ',') + ']}'))
            }
            $lastSig[$k] = $sig
        }
        foreach ($pr in $o.procs) {
            $pk = "p$($pr.pid)"
            if (-not $firstSeen.ContainsKey($pk)) {
                $firstSeen[$pk] = $t
                $evt.Add(('{"t":' + $t + ',"ev":"proc-seen","pid":' + $pr.pid + ',"name":' + ($pr.name | ConvertTo-Json -Compress) + '}'))
            }
        }
        if ($Port -gt 0 -and $null -eq $tcpOpen -and $o.tcp -eq $true) { $tcpOpen = $t; $evt.Add(('{"t":' + $t + ',"ev":"tcp-open"}')) }
    } catch {}
    $n++
    $gap = $sw.ElapsedMilliseconds - $loopStart
    if ($gap -gt $gapMax) { $gapMax = $gap }
    $gapSum += $gap; $gaps.Add($gap) | Out-Null
    $sleepLeft = $SampleMs - $gap
    if ($sleepLeft -gt 0) { Start-Sleep -Milliseconds $sleepLeft }
}
$gaps.Sort()
$med = $gaps[[int]($gaps.Count / 2)]
# AUD-010: enrich every sample line with the WebView2 child role (--type=...),
# queried once per distinct child pid from the live command line.
$distinct = @{}
Get-Content -LiteralPath $samplesPath | ForEach-Object {
    try { (ConvertFrom-Json $_).procs | ForEach-Object { $distinct[[int]$_.pid] = 1 } } catch {}
}
$roleOf = @{}
foreach ($rpid in ($distinct.Keys | Where-Object { $_ -ne $p.Id })) {
    $cmd = (Get-CimInstance Win32_Process -Filter "ProcessId=$rpid" -ErrorAction SilentlyContinue).CommandLine
    if ($cmd) {
        if ($cmd -match '--type=([a-z-]+)') { $roleOf["$rpid"] = $Matches[1] }
        else { $roleOf["$rpid"] = 'browser' }
    } else { $roleOf["$rpid"] = 'exited' }
}
$enr = New-Object System.Collections.Generic.List[string]
foreach ($ln in (Get-Content -LiteralPath $samplesPath)) {
    $out = $ln
    foreach ($rpid in $roleOf.Keys) {
        $pat = ('"pid":{0},"name":"([^"]*)"' -f $rpid)
        if ($out -match $pat) {
            $out = $out -replace $pat, ('$0,"role":"' + $roleOf[$rpid] + '"')
        }
    }
    $enr.Add($out)
}
[System.IO.File]::WriteAllText($samplesPath, ($enr -join "`n"), (New-Object System.Text.UTF8Encoding($false)))
[System.IO.File]::WriteAllText("$OutPrefix-roles.json", ($roleOf | ConvertTo-Json -Compress), (New-Object System.Text.UTF8Encoding($false)))
$p.Refresh()
$sum = [ordered]@{ pid = $p.Id; t0EpochMs = $t0EpochMs; utcAnchor = $utcAnchor; exited = $p.HasExited
  samples = $n; gapMaxMs = $gapMax; gapMedianMs = $med; gapMeanMs = [math]::Round($gapSum / [math]::Max($n,1), 1)
  tcpOpenMs = $tcpOpen; durationSec = $DurationSec }
[System.IO.File]::WriteAllText($eventsPath, ($evt -join "`n"), (New-Object System.Text.UTF8Encoding($false)))
$sum | ConvertTo-Json -Compress | Set-Content -LiteralPath $summaryPath -Encoding UTF8
Write-Output (Get-Content -LiteralPath $summaryPath -Raw)