@echo off
chcp 65001 >nul

:: Loquat Framework - Clean Script
:: Cleans build artifacts and temporary files

echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║                Loquat Framework - Clean                     ║
echo ╚════════════════════════════════════════════════════════════╝
echo.

set CLEAN_ALL=0

:: Parse arguments
if "%~1"=="--all" set CLEAN_ALL=1

echo [INFO] Cleaning Cargo build artifacts...
cargo clean
if %errorlevel% neq 0 (
    echo [ERROR] Failed to clean Cargo artifacts
    pause
    exit /b 1
)

if %CLEAN_ALL%==1 (
    echo [INFO] Cleaning logs directory...
    if exist "logs" (
        del /Q logs\*.* 2>nul
        echo [INFO] Logs cleaned
    )

    echo [INFO] Cleaning target directory...
    if exist "target" (
        rmdir /S /Q target 2>nul
        echo [INFO] Target directory removed
    )

    echo [INFO] Cleaning temporary files...
    if exist "*.log" del /Q *.log 2>nul
    if exist "*.tmp" del /Q *.tmp 2>nul
    echo [INFO] Temporary files cleaned
)

echo.
echo [SUCCESS] Clean completed successfully!
echo.
