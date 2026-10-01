# AUD-007 scripts/window-ops.ps1 — Win32 measurement / resize / close for the app window only.
# Usage:
#   window-ops.ps1 info -AppPid <pid>          -> JSON with DPI, rects, monitor work area, title
#   window-ops.ps1 resize -Hwnd <h> -ClientW <w> -ClientH <h> [-DwellMs 500] -> JSON before/after
#   window-ops.ps1 close -Hwnd <h>            -> posts WM_CLOSE
param(
    [Parameter(Mandatory = $true)][string]$Action,
    [int]$AppPid = 0,
    [long]$Hwnd = 0,
    [int]$ClientW = 0,
    [int]$ClientH = 0,
    [int]$DwellMs = 500
)

$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
public struct POINT { public int X, Y; }
[StructLayout(LayoutKind.Sequential)] public struct MONITORINFO {
    public int cbSize; public RECT rcMonitor; public RECT rcWork; public int dwFlags;
}
public static class Win {
    [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
    [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l);
    public delegate bool EnumWindowsProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint f);
    [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr h, ref MONITORINFO m);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out RECT r, int s);
    public static bool ClientOrigin(IntPtr h, out int x, out int y) {
        POINT p = new POINT { X = 0, Y = 0 };
        bool ok = ClientToScreen(h, ref p);
        x = p.X; y = p.Y; return ok;
    }
}
"@
Add-Type -TypeDefinition $code | Out-Null

# -4 = DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
$ctx = [Win]::SetProcessDpiAwarenessContext([IntPtr](-4))
$aware = ($ctx -ne [IntPtr]::Zero)

# Simpler: enumerate via Get-Process MainWindowHandle (already DPI-aware for this process context is irrelevant to the handle).
if ($Action -eq 'info') {
    $p = Get-Process -Id $AppPid -ErrorAction Stop
    $h = $p.MainWindowHandle
    if ($h -eq 0) { Write-Output '{"error":"no MainWindowHandle yet"}'; exit 2 }
    $dpi = [Win]::GetDpiForWindow($h)
    $r = New-Object RECT; [Win]::GetWindowRect($h, [ref]$r) | Out-Null
    $c = New-Object RECT; [Win]::GetClientRect($h, [ref]$c) | Out-Null
    $ox = 0; $oy = 0; [Win]::ClientOrigin($h, [ref]$ox, [ref]$oy) | Out-Null
    $fr = New-Object RECT; $hr = [Win]::DwmGetWindowAttribute($h, 9, [ref]$fr, 16)
    $mon = [Win]::MonitorFromWindow($h, 2)
    $mi = New-Object MONITORINFO; $mi.cbSize = 40; [Win]::GetMonitorInfo($mon, [ref]$mi) | Out-Null
    $sb = New-Object System.Text.StringBuilder 512
    [Win]::GetWindowText($h, $sb, 512) | Out-Null
    $zoomed = [Win]::IsZoomed($h)
    $o = [ordered]@{
        dpiAwareContext = $aware; hwnd = "$h"; dpi = $dpi; isZoomed = $zoomed
        title = $sb.ToString()
        outer = @($r.Left, $r.Top, $r.Right, $r.Bottom)
        clientSize = @(($c.Right - $c.Left), ($c.Bottom - $c.Top))
        clientOrigin = @($ox, $oy)
        frame = @($fr.Left, $fr.Top, $fr.Right, $fr.Bottom)
        monitor = @($mi.rcMonitor.Left, $mi.rcMonitor.Top, $mi.rcMonitor.Right, $mi.rcMonitor.Bottom)
        work = @($mi.rcWork.Left, $mi.rcWork.Top, $mi.rcWork.Right, $mi.rcWork.Bottom)
    }
    Write-Output ($o | ConvertTo-Json -Compress)
    exit 0
}

if ($Action -eq 'resize') {
    $h = [IntPtr]$Hwnd
    $dpi = [Win]::GetDpiForWindow($h)
    $f = $dpi / 96.0
    $r = New-Object RECT; [Win]::GetWindowRect($h, [ref]$r) | Out-Null
    $c = New-Object RECT; [Win]::GetClientRect($h, [ref]$c) | Out-Null
    $before = [ordered]@{ outer = @(($r.Right - $r.Left), ($r.Bottom - $r.Top)); client = @(($c.Right - $c.Left), ($c.Bottom - $c.Top)) }
    $dW = ($r.Right - $r.Left) - ($c.Right - $c.Left)
    $dH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
    $newW = [int][Math]::Round($ClientW * $f) + $dW
    $newH = [int][Math]::Round($ClientH * $f) + $dH
    [Win]::SetWindowPos($h, [IntPtr]::Zero, $r.Left, $r.Top, $newW, $newH, 0x0004 -bor 0x0010) | Out-Null
    Start-Sleep -Milliseconds $DwellMs
    $r2 = New-Object RECT; [Win]::GetWindowRect($h, [ref]$r2) | Out-Null
    $c2 = New-Object RECT; [Win]::GetClientRect($h, [ref]$c2) | Out-Null
    $o = [ordered]@{
        dpi = $dpi; requestedClientLogical = @($ClientW, $ClientH)
        before = $before
        after = [ordered]@{ outer = @(($r2.Right - $r2.Left), ($r2.Bottom - $r2.Top)); client = @(($c2.Right - $c2.Left), ($c2.Bottom - $c2.Top)) }
    }
    Write-Output ($o | ConvertTo-Json -Compress -Depth 4)
    exit 0
}

if ($Action -eq 'close') {
    [Win]::PostMessage([IntPtr]$Hwnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Write-Output '{"posted":true}'
    exit 0
}

if ($Action -eq 'restore') {
    # 9 = SW_RESTORE: leave maximized state, keep restore bounds
    [Win]::ShowWindow([IntPtr]$Hwnd, 9) | Out-Null
    Start-Sleep -Milliseconds $DwellMs
    Write-Output '{"restored":true}'
    exit 0
}

Write-Output '{"error":"unknown action"}'; exit 1
