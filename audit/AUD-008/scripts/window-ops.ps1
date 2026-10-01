# AUD-008 scripts/window-ops.ps1 - Win32 window ops for min-size probes and measurement.
# Usage:
#   window-ops.ps1 minmax -Hwnd <h>                     Win32 MINMAXINFO (message-level truth)  [JSON]
#   window-ops.ps1 info -Hwnd <h>                       DPI, rects, monitor, title, zoomed       [JSON]
#   window-ops.ps1 restore -Hwnd <h>                    SW_RESTORE
#   window-ops.ps1 setclient -Hwnd <h> -ClientW <w> -ClientH <h>   SetWindowPos target client logical
#   window-ops.ps1 moveclient -Hwnd <h> -ClientW <w> -ClientH <h>  MoveWindow  target client logical
#   window-ops.ps1 cdpsize -Hwnd <h>                    return client logical size                 [JSON]
#   window-ops.ps1 drag -Hwnd <h> -Corner BR|TL -ToW <cw> -ToH <ch>  real mouse drag resize         [JSON]
#   window-ops.ps1 close -Hwnd <h>                     post WM_CLOSE
param([Parameter(Mandatory=$true)][string]$Action,
      [long]$Hwnd = 0,
      [int]$ClientW = 0,
      [int]$ClientH = 0,
      [string]$Corner = 'BR',
      [int]$DwellMs = 500)

$code = @"
using System;
using System.Runtime.InteropServices;
public struct RECT { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct POINT2 { public int X, Y; }
[StructLayout(LayoutKind.Sequential)] public struct MINMAXINFO { public POINT2 ptReserved; public POINT2 ptMaxSize; public POINT2 ptMaxPosition; public POINT2 ptMinTrackSize; public POINT2 ptMaxTrackSize; }
public static class WO {
  [DllImport("user32.dll")] public static extern IntPtr SetProcessDpiAwarenessContext(IntPtr v);
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, System.Text.StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr h, int x, int y, int cx, int cy, bool repaint);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetFocus(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT2 p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT2 p);
  [DllImport("user32.dll")] public static extern void mouse_event(uint d, uint dx, uint dy, uint dw, UIntPtr extra);
  [DllImport("user32.dll")] public static extern IntPtr MonitorFromWindow(IntPtr h, uint f);
  [DllImport("user32.dll")] public static extern bool GetMonitorInfo(IntPtr h, ref MONITORINFO m);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int attr, out RECT r, int size);
  [StructLayout(LayoutKind.Sequential)] public struct MONITORINFO { public int cbSize; public RECT rcMonitor; public RECT rcWork; public int dwFlags; }
  public static int ClientOriginX(IntPtr h) { POINT2 p = new POINT2 { X = 0, Y = 0 }; ClientToScreen(h, ref p); return p.X; }
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT2 p);
}
"@
Add-Type -TypeDefinition $code | Out-Null
$ctx = [WO]::SetProcessDpiAwarenessContext([IntPtr](-4))

function ClientLogical([IntPtr]$h) {
  $dpi = [WO]::GetDpiForWindow($h)
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $f = $dpi / 96.0
  return ,@([math]::Round(($c.Right - $c.Left) / $f, 1), [math]::Round(($c.Bottom - $c.Top) / $f, 1), $dpi)
}

if ($Action -eq 'minmax') {
  $h = [IntPtr]$Hwnd
  $mm = New-Object MINMAXINFO
  $mm.ptReserved = New-Object POINT2; $mm.ptMaxSize = New-Object POINT2; $mm.ptMaxPosition = New-Object POINT2; $mm.ptMinTrackSize = New-Object POINT2; $mm.ptMaxTrackSize = New-Object POINT2
  foreach ($f in 'ptReserved','ptMaxSize','ptMaxPosition','ptMinTrackSize','ptMaxTrackSize') {
    $p = $mm.$f; $p.X = -1; $p.Y = -1; $mm.$f = $p
  }
  $ptr = [System.Runtime.InteropServices.Marshal]::AllocHGlobal([System.Runtime.InteropServices.Marshal]::SizeOf($mm))
  [System.Runtime.InteropServices.Marshal]::StructureToPtr($mm, $ptr, $false) | Out-Null
  [WO]::SendMessage($h, 0x0024, [IntPtr]::Zero, $ptr) | Out-Null
  $read = [System.Runtime.InteropServices.Marshal]::PtrToStructure($ptr, [type][MINMAXINFO])
  [System.Runtime.InteropServices.Marshal]::FreeHGlobal($ptr)
  $dpi = [WO]::GetDpiForWindow($h)
  $f = $dpi / 96.0
  $r = New-Object RECT; [WO]::GetWindowRect($h, [ref]$r) | Out-Null
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $ncdW = ($r.Right - $r.Left) - ($c.Right - $c.Left)
  $ncdH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
  $mtsx = $read.ptMinTrackSize.X; $mtsy = $read.ptMinTrackSize.Y
  $o = [ordered]@{
    dpi = $dpi; ncdPhysical = @($ncdW, $ncdH)
    ptReserved = @($read.ptReserved.X, $read.ptReserved.Y)
    ptMaxSize = @($read.ptMaxSize.X, $read.ptMaxSize.Y)
    ptMaxPosition = @($read.ptMaxPosition.X, $read.ptMaxPosition.Y)
    ptMinTrackSizePhysical = @($mtsx, $mtsy)
    ptMinTrackSizeLogical = @([math]::Round($mtsx / $f, 1), [math]::Round($mtsy / $f, 1))
    ptMinTrackSizeClientLogical = @([math]::Round(($mtsx - $ncdW) / $f, 1), [math]::Round(($mtsy - $ncdH) / $f, 1))
    ptMaxTrackSizePhysical = @($read.ptMaxTrackSize.X, $read.ptMaxTrackSize.Y)
    sentinelsUnchanged = ($mtsx -eq -1 -and $mtsy -eq -1)
  }
  Write-Output ($o | ConvertTo-Json -Compress)
  exit 0
}
if ($Action -eq 'info') {
  $h = [IntPtr]$Hwnd
  $dpi = [WO]::GetDpiForWindow($h); $f = $dpi / 96.0
  $r = New-Object RECT; [WO]::GetWindowRect($h, [ref]$r) | Out-Null
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $sb = New-Object System.Text.StringBuilder 256; [WO]::GetWindowText($h, $sb, 256) | Out-Null
  $mon = [WO]::MonitorFromWindow($h, 2); $mi = New-Object WO+MONITORINFO; $mi.cbSize = 40; [WO]::GetMonitorInfo($mon, [ref]$mi) | Out-Null
  $ox = [int]($r.Right - $r.Left); $oy = [int]($r.Bottom - $r.Top); $cx = [int]($c.Right - $c.Left); $cy = [int]($c.Bottom - $c.Top)
  $o = [ordered]@{ dpi = $dpi; hwnd = "$h"; zoomed = [WO]::IsZoomed($h); title = $sb.ToString()
    outerPhysical = @($r.Left, $r.Top, $r.Right, $r.Bottom); clientPhysical = @($cx, $cy)
    outerLogical = @([math]::Round($ox / $f, 1), [math]::Round($oy / $f, 1))
    clientLogical = @([math]::Round($cx / $f, 1), [math]::Round($cy / $f, 1))
    workPhysical = @($mi.rcWork.Left, $mi.rcWork.Top, $mi.rcWork.Right, $mi.rcWork.Bottom)
    monitorPhysical = @($mi.rcMonitor.Left, $mi.rcMonitor.Top, $mi.rcMonitor.Right, $mi.rcMonitor.Bottom) }
  Write-Output ($o | ConvertTo-Json -Compress)
  exit 0
}
if ($Action -eq 'restore') { [WO]::ShowWindow([IntPtr]$Hwnd, 9) | Out-Null; Start-Sleep -Milliseconds $DwellMs; exit 0 }
if ($Action -eq 'setclient') {
  $h = [IntPtr]$Hwnd; $dpi = [WO]::GetDpiForWindow($h); $f = $dpi / 96.0
  $r = New-Object RECT; [WO]::GetWindowRect($h, [ref]$r) | Out-Null
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $dW = ($r.Right - $r.Left) - ($c.Right - $c.Left); $dH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
  $nw = [int][math]::Round($ClientW * $f) + $dW; $nh = [int][math]::Round($ClientH * $f) + $dH
  [WO]::SetWindowPos($h, [IntPtr]::Zero, $r.Left, $r.Top, $nw, $nh, 0x0004 -bor 0x0010) | Out-Null
  Start-Sleep -Milliseconds $DwellMs
  $cl = ClientLogical $h
  Write-Output (@{ action = 'setclient'; requested = @($ClientW, $ClientH); afterClientLogical = $cl } | ConvertTo-Json -Compress)
  exit 0
}
if ($Action -eq 'moveclient') {
  $h = [IntPtr]$Hwnd; $dpi = [WO]::GetDpiForWindow($h); $f = $dpi / 96.0
  $r = New-Object RECT; [WO]::GetWindowRect($h, [ref]$r) | Out-Null
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $dW = ($r.Right - $r.Left) - ($c.Right - $c.Left); $dH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
  $nw = [int][math]::Round($ClientW * $f) + $dW; $nh = [int][math]::Round($ClientH * $f) + $dH
  [WO]::MoveWindow($h, $r.Left, $r.Top, $nw, $nh, $true) | Out-Null
  Start-Sleep -Milliseconds $DwellMs
  $cl = ClientLogical $h
  Write-Output (@{ action = 'moveclient'; requested = @($ClientW, $ClientH); afterClientLogical = $cl } | ConvertTo-Json -Compress)
  exit 0
}
if ($Action -eq 'cdpsize') { # returns the current client logical as JSON only
  $cl = ClientLogical ([IntPtr]$Hwnd); Write-Output (@{ clientLogical = @($cl[0], $cl[1]); dpi = $cl[2] } | ConvertTo-Json -Compress); exit 0 }
if ($Action -eq 'close') { [WO]::PostMessage([IntPtr]$Hwnd, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null; exit 0 }

if ($Action -eq 'drag') {
  $h = [IntPtr]$Hwnd
  if (-not [WO]::SetForegroundWindow($h)) { Write-Output '{"skip":"SetForegroundWindow failed"}'; exit 3 }
  Start-Sleep -Milliseconds 200
  if ([WO]::GetForegroundWindow() -ne $h) { Write-Output '{"skip":"not foreground"}'; exit 3 }
  $r = New-Object RECT; [WO]::GetWindowRect($h, [ref]$r) | Out-Null
  $c = New-Object RECT; [WO]::GetClientRect($h, [ref]$c) | Out-Null
  $dpi = [WO]::GetDpiForWindow($h); $f = $dpi / 96.0
  $dW = ($r.Right - $r.Left) - ($c.Right - $c.Left); $dH = ($r.Bottom - $r.Top) - ($c.Bottom - $c.Top)
  $cur = New-Object POINT2; [WO]::GetCursorPos([ref]$cur) | Out-Null
  $grab = New-Object POINT2
  $ht = 0; $targetDX = 0; $targetDY = 0
  if ($Corner -eq 'BR') {
    $cand = @( @(($r.Right - 3), ($r.Bottom - 3)), @(($r.Right - 1), ($r.Bottom - 1)), @(($r.Right + 2), ($r.Bottom + 2)) )
    foreach ($pp in $cand) { $grab.X = $pp[0]; $grab.Y = $pp[1]; $wm = [WO]::WindowFromPoint($grab); if ([WO]::GetAncestor($wm, 2) -eq $h) { $ht = ([WO]::SendMessage($h, 0x0084, [IntPtr]::Zero, [IntPtr]$(($grab.Y -shl 16) -bor ($grab.X -band 0xFFFF)))).ToInt32(); if ($ht -eq 17) { break } } }
    if ($ht -ne 17) { Write-Output '{"skip":"no HTBOTTOMRIGHT"}'; exit 3 }
    $targetDX = -([math]::Round($ClientW * $f) + $dW - ($r.Right - $r.Left)); $targetDY = -([math]::Round($ClientH * $f) + $dH - ($r.Bottom - $r.Top))
    $tpx = $grab.X + $targetDX; $tpy = $grab.Y + $targetDY
  } else {
    $cand = @( @(($r.Left + 3), ($r.Top + 3)), @(($r.Left + 1), ($r.Top + 1)), @(($r.Left - 2), ($r.Top - 2)) )
    foreach ($pp in $cand) { $grab.X = $pp[0]; $grab.Y = $pp[1]; $wm = [WO]::WindowFromPoint($grab); if ([WO]::GetAncestor($wm, 2) -eq $h) { $ht = ([WO]::SendMessage($h, 0x0084, [IntPtr]::Zero, [IntPtr]$(($grab.Y -shl 16) -bor ($grab.X -band 0xFFFF)))).ToInt32(); if ($ht -eq 13) { break } } }
    if ($ht -ne 13) { Write-Output '{"skip":"no HTTOPLEFT"}'; exit 3 }
    $tpx = $grab.X - ([math]::Round($ClientW * $f) + $dW - ($r.Right - $r.Left)); $tpy = $grab.Y - ([math]::Round($ClientH * $f) + $dH - ($r.Bottom - $r.Top))
  }
  [WO]::SetCursorPos($grab.X, $grab.Y) | Out-Null
  Start-Sleep -Milliseconds 100
  [WO]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # LEFTDOWN
  Start-Sleep -Milliseconds 120
  $steps = 20
  for ($i = 1; $i -le $steps; $i++) {
    $x = $grab.X + [int][math]::Round(($tpx - $grab.X) * $i / $steps)
    $y = $grab.Y + [int][math]::Round(($tpy - $grab.Y) * $i / $steps)
    [WO]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 15
  }
  [WO]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # LEFTUP
  Start-Sleep -Milliseconds $DwellMs
  [WO]::SetCursorPos($cur.X, $cur.Y) | Out-Null
  $cl = ClientLogical $h
  Write-Output (@{ action = 'drag'; corner = $Corner; ht = $ht; targetClientLogical = @($ClientW, $ClientH); afterClientLogical = $cl } | ConvertTo-Json -Compress)
  exit 0
}
Write-Output '{"error":"unknown action"}'; exit 1