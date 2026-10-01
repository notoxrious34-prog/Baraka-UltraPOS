# AUD-008 scripts/dwm-frame.ps1 - supplementary: DWM visible frame + client origin (one JSON line).
# Usage: dwm-frame.ps1 -Hwnd <h>
param([Parameter(Mandatory=$true)][long]$Hwnd)
$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT2 { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct PT2 { public int X, Y; }
public static class DF {
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT2 r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT2 r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref PT2 p);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT2 r, int size);
}
"@
Add-Type -TypeDefinition $code | Out-Null
[DF]::SetProcessDpiAwarenessContext([IntPtr](-4)) | Out-Null
$h = [IntPtr]$Hwnd
$dpi = [DF]::GetDpiForWindow($h); $f = $dpi / 96.0
$fr = New-Object RECT2; $hr = [DF]::DwmGetWindowAttribute($h, 9, [ref]$fr, 16)
$co = New-Object PT2; $co.X = 0; $co.Y = 0; [DF]::ClientToScreen($h, [ref]$co) | Out-Null
$o = [ordered]@{ dpi = $dpi; dwmHr = $hr
  dwmFramePhysical = @($fr.Left, $fr.Top, $fr.Right, $fr.Bottom)
  dwmFrameLogical = @([math]::Round($fr.Left / $f, 1), [math]::Round($fr.Top / $f, 1), [math]::Round($fr.Right / $f, 1), [math]::Round($fr.Bottom / $f, 1))
  clientOriginPhysical = @($co.X, $co.Y); clientOriginLogical = @([math]::Round($co.X / $f, 1), [math]::Round($co.Y / $f, 1))
  iconic = [DF]::IsIconic($h) }
Write-Output ($o | ConvertTo-Json -Compress)