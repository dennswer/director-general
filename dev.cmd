@echo off
REM Voiceeee - tauri dev wrapper. Same env setup as build-rust.cmd, then `npm run tauri dev`.

set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "PATH=C:\Program Files\CMake\bin;%PATH%"
set "PATH=C:\Program Files\LLVM\bin;%PATH%"
set "PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6\bin;%PATH%"
set "PATH=C:\Users\Dennswer\AppData\Local\Microsoft\WinGet\Packages\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe;%PATH%"
set "PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%"

call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 (
  echo [ERROR] vcvars64.bat failed
  exit /b 1
)

set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6"
set "CMAKE_GENERATOR=Ninja"

pushd "%~dp0"
echo [voiceeee] starting tauri dev...
npm run tauri dev
popd
