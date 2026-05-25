@echo off
REM Voiceeee - Rust build wrapper
REM Loads MSVC BuildTools env, sets CMake generator to Ninja, then runs cargo command.
REM Usage: build-rust.cmd check        (or: build, run, etc.)

REM Add tool paths that may not be on the inherited PATH (newly installed today)
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "PATH=C:\Program Files\CMake\bin;%PATH%"
set "PATH=C:\Program Files\LLVM\bin;%PATH%"
set "PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6\bin;%PATH%"
set "PATH=C:\Users\Dennswer\AppData\Local\Microsoft\WinGet\Packages\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe;%PATH%"
REM vswhere is needed by vcvars64.bat to find SDK paths
set "PATH=C:\Program Files (x86)\Microsoft Visual Studio\Installer;%PATH%"

call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if errorlevel 1 (
  echo [ERROR] vcvars64.bat failed
  exit /b 1
)

set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
set "CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.6"
set "CMAKE_GENERATOR=Ninja"

echo --- Env ---
where cargo
where cmake
where clang
where nvcc
where ninja
where cl
echo LIBCLANG_PATH=%LIBCLANG_PATH%
echo CUDA_PATH=%CUDA_PATH%
echo CMAKE_GENERATOR=%CMAKE_GENERATOR%
echo --- Running cargo %* in src-tauri ---

pushd "%~dp0src-tauri"
cargo %*
set "RC=%errorlevel%"
popd
exit /b %RC%
