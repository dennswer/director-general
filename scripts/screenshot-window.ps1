param(
    [Parameter(Mandatory=$true)] [string]$ProcessName,
    [Parameter(Mandatory=$true)] [string]$Out
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
public struct RECT { public int Left, Top, Right, Bottom; }
"@

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) {
    Write-Error "No window found for $ProcessName"
    exit 1
}

$h = $proc.MainWindowHandle

# Restore if minimized
if ([Win32]::IsIconic($h)) {
    [Win32]::ShowWindow($h, 9) | Out-Null # SW_RESTORE
    Start-Sleep -Milliseconds 300
}

# Bring to foreground
[Win32]::SetForegroundWindow($h) | Out-Null
Start-Sleep -Milliseconds 300

$r = New-Object RECT
if (-not [Win32]::GetWindowRect($h, [ref]$r)) {
    Write-Error "GetWindowRect failed"
    exit 1
}

$w = $r.Right - $r.Left
$ht = $r.Bottom - $r.Top
Write-Host "Window rect: $($r.Left),$($r.Top) - $w x $ht"

$bmp = New-Object System.Drawing.Bitmap($w, $ht)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
$bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose()
$bmp.Dispose()
Write-Host "Saved $Out"
