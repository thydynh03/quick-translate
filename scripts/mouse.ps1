# Injects mouse input for the manual end-to-end checks.
#   mouse.ps1 click 500 300
#   mouse.ps1 double 500 300
#   mouse.ps1 drag 500 300 800 300
param([string]$Action, [int]$X1, [int]$Y1, [int]$X2 = 0, [int]$Y2 = 0)

Add-Type @"
using System.Runtime.InteropServices;
public class QtMouse {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, System.UIntPtr extra);
}
"@
[QtMouse]::SetProcessDPIAware() | Out-Null
$down = 0x0002; $up = 0x0004

function Press { [QtMouse]::mouse_event($down, 0, 0, 0, [UIntPtr]::Zero) }
function Release { [QtMouse]::mouse_event($up, 0, 0, 0, [UIntPtr]::Zero) }

[QtMouse]::SetCursorPos($X1, $Y1) | Out-Null
Start-Sleep -Milliseconds 150
switch ($Action) {
    'move' { }
    'click' { Press; Start-Sleep -Milliseconds 40; Release }
    'double' { Press; Release; Start-Sleep -Milliseconds 90; Press; Release }
    'drag' {
        Press
        $steps = 12
        for ($i = 1; $i -le $steps; $i++) {
            [QtMouse]::SetCursorPos($X1 + ($X2 - $X1) * $i / $steps, $Y1 + ($Y2 - $Y1) * $i / $steps) | Out-Null
            Start-Sleep -Milliseconds 20
        }
        Release
    }
}
