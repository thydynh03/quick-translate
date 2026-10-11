# Saves a screenshot of a screen region to a PNG. Used by the manual end-to-end checks.
param([int]$X = 0, [int]$Y = 0, [int]$W = 1920, [int]$H = 1080, [string]$Out = "$env:TEMP\qt-shot.png")

Add-Type -AssemblyName System.Drawing
Add-Type @"
using System.Runtime.InteropServices;
public class QtDpi { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[QtDpi]::SetProcessDPIAware() | Out-Null

$bitmap = New-Object System.Drawing.Bitmap $W, $H
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($X, $Y, 0, 0, $bitmap.Size)
$bitmap.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
$Out
