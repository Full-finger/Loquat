@echo off
chcp 65001 >nul

:: Loquat Framework - Rebuild Script
:: Cleans and rebuilds the project

echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║                Loquat Framework - Rebuild                   ║
echo ╚════════════════════════════════════════════════════════════╝
echo.

echo [INFO] Cleaning build artifacts...
cargo clean
if %errorlevel% neq 0 (
    echo [ERROR] Failed to clean build artifacts
    pause
    exit /b 1
)

echo [INFO] Building release version...
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Build failed
    pause
    exit /b 1
)

echo.
echo [SUCCESS] Rebuild completed successfully!
echo.
