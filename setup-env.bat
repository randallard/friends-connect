@echo off
REM This script sets up the Visual Studio build environment for Rust projects that need MSVC tools

REM Find the VS installation path
if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" (
    echo "Setting up Visual Studio 2022 Community environment..."
    call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" x64
) else if exist "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" (
    echo "Setting up Visual Studio 2022 Community environment (x86 path)..."
    call "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvarsall.bat" x64
) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvarsall.bat" (
    echo "Setting up Visual Studio 2022 Enterprise environment..."
    call "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvarsall.bat" x64
) else if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat" (
    echo "Setting up Visual Studio 2022 Professional environment..."
    call "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvarsall.bat" x64
) else (
    echo Visual Studio environment setup script not found.
    echo Please ensure Visual Studio 2022 with C++ tools is installed.
    exit /b 1
)

REM Explicitly set INCLUDE path to include Windows SDK
set INCLUDE=%INCLUDE%;%WindowsSdkDir%\Include\%WindowsSDKVersion%\um;%WindowsSdkDir%\Include\%WindowsSDKVersion%\shared

echo VS Environment setup complete
echo Running cargo build
cargo build

REM If you want to just set environment variables without running cargo build
REM Remove the line above and run your own cargo commands after running this script