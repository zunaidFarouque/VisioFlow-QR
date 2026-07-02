# Generates hidden .vbs launchers beside visioflow.exe (for Scoop/release zip).
# Usage:
#   .\scripts\generate-launchers.ps1 -OutDir .\dist\visioflow-win-x64

param(
    [Parameter(Mandatory)]
    [string]$OutDir
)

$ErrorActionPreference = "Stop"

$launcherDir = Join-Path $OutDir "launchers"
New-Item -ItemType Directory -Path $launcherDir -Force | Out-Null

$entries = @(
    @{ Name = "camera-auto"; Args = "capture --source webcam" },
    @{ Name = "camera-copy"; Args = "capture --source webcam --trigger copy" },
    @{ Name = "snip-auto"; Args = "capture --source snip" },
    @{ Name = "snip-copy"; Args = "capture --source snip --trigger copy" }
)

foreach ($entry in $entries) {
    $launcherPath = Join-Path $launcherDir "$($entry.Name).vbs"
    $argsLiteral = $entry.Args -replace '"', '""'
    $body = @"
Option Explicit
Dim fso, shell, launcherDir, appDir, exe, command, i, arg
Set fso = CreateObject("Scripting.FileSystemObject")
Set shell = CreateObject("WScript.Shell")
launcherDir = fso.GetParentFolderName(WScript.ScriptFullName)
appDir = fso.GetParentFolderName(launcherDir)
exe = fso.BuildPath(appDir, "visioflow.exe")
command = """" & exe & """ $argsLiteral"
For i = 0 To WScript.Arguments.Count - 1
    arg = Replace(WScript.Arguments(i), """", """""")
    command = command & " """ & arg & """"
Next
shell.Run command, 0, False
"@
    Set-Content -Path $launcherPath -Value $body -Encoding ASCII
}

Write-Host "Generated launchers in: $launcherDir"
