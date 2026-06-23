@echo off
if defined VSCMD_VER goto :eof

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" (
  for /f "usebackq delims=" %%I in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSINSTALL=%%I"
)

if defined VSINSTALL (
  if exist "%VSINSTALL%\VC\Auxiliary\Build\vcvars64.bat" (
    call "%VSINSTALL%\VC\Auxiliary\Build\vcvars64.bat" >nul
    goto :eof
  )
)

for %%D in (
  "C:\Program Files\Microsoft Visual Studio\18\BuildTools"
  "C:\Program Files\Microsoft Visual Studio\18\Community"
  "C:\Program Files\Microsoft Visual Studio\2022\BuildTools"
  "C:\Program Files\Microsoft Visual Studio\2022\Community"
) do (
  if exist "%%~D\VC\Auxiliary\Build\vcvars64.bat" (
    call "%%~D\VC\Auxiliary\Build\vcvars64.bat" >nul
    goto :eof
  )
)

echo Could not find Visual Studio vcvars64.bat. Cargo may fail to link.
