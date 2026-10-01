# AUD-008 scripts/drag-probe.ps1 - real mouse drag with neighborhood grab-point search.
# Gates: G1 normal 1280x640 client; G2 foreground; G3 WindowFromPoint root==hwnd;
#        G4 hit-test 17 (BR) / 13 (TL) found within +-12px, ON-SCREEN point required;
#        G5 <=3s total, button always released, cursor restored.
# Usage: drag-probe.ps1 -Hwnd <h> -Corner BR|TL -ToW <clientLogicalW> -ToH <clientLogicalH>
param([Parameter(Mandatory=$true)][long]$Hwnd,
      [Parameter(Mandatory=$true)][string]$Corner,
      [Parameter(Mandatory=$true)][int]$ToW,
      [Parameter(Mandatory=$true)][int]$ToH)
$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT4 { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct PT4 { public int X, Y; }
public static class DG {
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT4 r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT4 r);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(PT4 p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out PT4 p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint d, uint dx, uint dy, uint dw, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr h, ref MON4 m);
  [StructLayout(LayoutKind.Sequential)] public struct MON4 { public int cbSize; public RECT4 rcMonitor; public RECT4 rcWork; public int dwFlags; }
}
"@
Add-Type -TypeDefinition $code | Out-Null
[DG]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$h = [IntPtr]$Hwnd
$fail = { param($g, $d) Write-Output (@{ gateFail = $g; detail = $d } | ConvertTo-Json -Compress); exit 3 }
$dpi = [DG]::GetDpiForWindow($h); $f = $dpi / 96.0
# G1: normal state, client 1280x640
if ([DG]::IsZoomed($h) -or [DG]::IsIconic($h)) { &$fail 'G1' 'zoomed-or-iconic' }
$c = New-Object RECT4; [DG]::GetClientRect($h, [ref]$c) | Out-Null
$cw = [math]::Round(($c.Right - $c.Left) / $f, 1); $ch = [math]::Round(($c.Bottom - $c.Top) / $f, 1)
if ([math]::Abs($cw - 1280) -gt 1 -or [math]::Abs($ch - 640) -gt 1) { &$fail 'G1' ("client=${cw}x${ch}") }
# G2: foreground
if (-not [DG]::SetForegroundWindow($h)) { &$fail 'G2' 'SetForegroundWindow-false' }
Start-Sleep -Milliseconds 200
if ([DG]::GetForegroundWindow() -ne $h) { &$fail 'G2' 'not-foreground' }
$r = New-Object RECT4; [DG]::GetWindowRect($h, [ref]$r) | Out-Null
$mon = [DG]::MonitorFromWindow($h, 2); $mi = New-Object DG+MON4; $mi.cbSize = 40; [DG]::GetMonitorInfo($mon, [ref]$mi) | Out-Null
$mL = $mi.rcMonitor.Left; $mR = $mi.rcMonitor.Right; $mT = $mi.rcMonitor.Top; $mB = $mi.rcMonitor.Bottom
$want = if ($Corner -eq 'BR') { 17 } else { 13 }
# G3+G4: neighborhood search +-12px for on-screen, uncovered, correct-hit point
$grab = $null
for ($rad = 0; $rad -le 12 -and $null -eq $grab; $rad++) {
  for ($dx = -$rad; $dx -le $rad; $dx++) {
    for ($dy = -$rad; $dy -le $rad; $dy++) {
      if ([math]::Max([math]::Abs($dx), [math]::Abs($dy)) -ne $rad) { continue }
      if ($Corner -eq 'BR') { $px = $r.Right - 1 + $dx; $py = $r.Bottom - 1 + $dy }
      else { $px = $r.Left + 1 + $dx; $py = $r.Top + 1 + $dy }
      if ($px -lt $mL -or $px -ge $mR -or $py -lt $mT -or $py -ge $mB) { continue }  # must be on-screen
      $pt = New-Object PT4; $pt.X = $px; $pt.Y = $py
      $wfp = [DG]::WindowFromPoint($pt)
      if ($wfp -eq [IntPtr]::Zero -or [DG]::GetAncestor($wfp, 2) -ne $h) { continue }  # G3
      $lp = (($py -shl 16) -bor ($px -band 0xFFFF))
      $ht = ([DG]::SendMessage($h, 0x0084, [IntPtr]::Zero, [IntPtr]$lp)).ToInt32()
      if ($ht -eq $want) { $grab = @($px, $py); break }
    }
    if ($null -ne $grab) { break }
  }
}
if ($null -eq $grab) { &$fail 'G3G4' ("no on-screen uncovered ht=$want point in +-12px") }
# target point: outer delta for requested client size
$dW = ($r.Right - $r.Left) - ($c.Right - $c.Left); $dH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
$cur0 = New-Object PT4; [DG]::GetCursorPos([ref]$cur0) | Out-Null
$t0ms = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
try {
  if ($Corner -eq 'BR') {
    # move amount: newOuter - oldOuter applied at grab corner
    $tpx = $grab[0] + (([int][math]::Round($ToW * $f) + $dW) - ($r.Right - $r.Left))
    $tpy = $grab[1] + (([int][math]::Round($ToH * $f) + $dH) - ($r.Bottom - $r.Top))
  } else {
    $tpx = $grab[0] - (([int][math]::Round($ToW * $f) + $dW) - ($r.Right - $r.Left))
    $tpy = $grab[1] - (([int][math]::Round($ToH * $f) + $dH) - ($r.Bottom - $r.Top))
  }
  [DG]::SetCursorPos($grab[0], $grab[1]) | Out-Null
  Start-Sleep -Milliseconds 100
  [DG]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero) | Out-Null
  Start-Sleep -Milliseconds 120
  $steps = 18
  for ($i = 1; $i -le $steps; $i++) {
    $x = $grab[0] + [int][math]::Round(($tpx - $grab[0]) * $i / $steps)
    $y = $grab[1] + [int][math]::Round(($tpy - $grab[1]) * $i / $steps)
    [DG]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 10
  }
  Start-Sleep -Milliseconds 500
} finally {
  [DG]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero) | Out-Null
  [DG]::SetCursorPos($cur0.X, $cur0.Y) | Out-Null
}
$dur = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() - $t0ms
$r2 = New-Object RECT4; [DG]::GetWindowRect($h, [ref]$r2) | Out-Null
$c2 = New-Object RECT4; [DG]::GetClientRect($h, [ref]$c2) | Out-Null
$o = [ordered]@{ action = 'drag2'; corner = $Corner; ht = $want; grabPhysical = $grab
  targetClientLogical = @($ToW, $ToH)
  afterOuterPhysical = @($r2.Left, $r2.Top, $r2.Right, $r2.Bottom)
  afterClientLogical = @([math]::Round(($c2.Right - $c2.Left) / $f, 1), [math]::Round(($c2.Bottom - $c2.Top) / $f, 1))
  durationMs = $dur; withinBudget = ($dur -le 3000) }
Write-Output ($o | ConvertTo-Json -Compress)