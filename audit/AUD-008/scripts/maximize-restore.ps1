# AUD-008 scripts/maximize-restore.ps1 - SW_MAXIMIZE / SW_RESTORE with rect + DWM frame readback.
# Usage: maximize-restore.ps1 -Hwnd <h> -Mode max|restore
param([Parameter(Mandatory=$true)][long]$Hwnd,
      [Parameter(Mandatory=$true)][string]$Mode)
$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT3 { public int Left, Top, Right, Bottom; }
public static class MR {
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT3 r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT3 r);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr h, ref MON3 m);
  [StructLayout(LayoutKind.Sequential)] public struct MON3 { public int cbSize; public RECT3 rcMonitor; public RECT3 rcWork; public int dwFlags; }
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT3 r, int size);
}
"@
Add-Type -TypeDefinition $code | Out-Null
[MR]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$h = [IntPtr]$Hwnd
$cmd = if ($Mode -eq 'max') { 3 } else { 9 }
[MR]::ShowWindow($h, $cmd) | Out-Null
Start-Sleep -Milliseconds 700
$dpi = [MR]::GetDpiForWindow($h); $f = $dpi / 96.0
$r = New-Object RECT3; [MR]::GetWindowRect($h, [ref]$r) | Out-Null
$c = New-Object RECT3; [MR]::GetClientRect($h, [ref]$c) | Out-Null
$fr = New-Object RECT3; $hr = [MR]::DwmGetWindowAttribute($h, 9, [ref]$fr, 16)
$mon = [MR]::MonitorFromWindow($h, 2); $mi = New-Object MR+MON3; $mi.cbSize = 40; [MR]::GetMonitorInfo($mon, [ref]$mi) | Out-Null
$o = [ordered]@{ mode = $Mode; dpi = $dpi; zoomed = [MR]::IsZoomed($h); dwmHr = $hr
  outerPhysical = @($r.Left, $r.Top, $r.Right, $r.Bottom)
  clientPhysical = @(([int]($c.Right - $c.Left)), ([int]($c.Bottom - $c.Top)))
  clientLogical = @([math]::Round(($c.Right - $c.Left) / $f, 1), [math]::Round(($c.Bottom - $c.Top) / $f, 1))
  dwmFramePhysical = @($fr.Left, $fr.Top, $fr.Right, $fr.Bottom)
  workPhysical = @($mi.rcWork.Left, $mi.rcWork.Top, $mi.rcWork.Right, $mi.rcWork.Bottom) }
Write-Output ($o | ConvertTo-Json -Compress)