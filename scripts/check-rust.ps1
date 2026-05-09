$ErrorActionPreference = "Stop"

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$vsDevCmd = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if (-not (Test-Path $vsDevCmd)) {
    Write-Error "Visual Studio Build Tools were not found. Install Microsoft.VisualStudio.2022.BuildTools with the VCTools workload."
}

Push-Location (Join-Path $PSScriptRoot "..\src-tauri")
try {
    & cmd.exe /d /s /c "`"$vsDevCmd`" -arch=x64 && set `"PATH=%USERPROFILE%\.cargo\bin;%PATH%`" && cargo test"
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}
