@echo off
echo Building BurnCloud with optimized UI...
cd /d "%~dp0"
cargo build --release
if %ERRORLEVEL% equ 0 (
    echo.
    echo ✅ Build successful! Starting BurnCloud...
    echo.
    target\release\burncloud.exe
) else (
    echo.
    echo ❌ Build failed. Please check the error messages above.
    pause
)