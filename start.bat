@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion

:: Loquat Framework - One-Click Startup Script for Windows
:: Usage: start.bat [environment] [options]
::   environment: dev (default), test, prod
::   options: --rebuild

echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║                    Loquat Framework                        ║
echo ║             One-Click Startup System                       ║
echo ╚════════════════════════════════════════════════════════════╝
echo.

:: Parse arguments
set ENV=dev
set REBUILD=0

:parse_args
if "%~1"=="" goto end_parse_args
if "%~1"=="dev" (
    set ENV=dev
    shift
    goto parse_args
)
if "%~1"=="test" (
    set ENV=test
    shift
    goto parse_args
)
if "%~1"=="prod" (
    set ENV=prod
    shift
    goto parse_args
)
if "%~1"=="--rebuild" (
    set REBUILD=1
    shift
    goto parse_args
)
if "%~1"=="--env" (
    shift
    if not "%~1"=="" (
        set ENV=%~1
        shift
    )
    goto parse_args
)
shift
goto parse_args

:end_parse_args

echo Environment: %ENV%
echo.

:: Check Rust installation
where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust is not installed or not in PATH
    echo Please install Rust from https://rustup.rs/
    pause
    exit /b 1
)

echo [INFO] Rust environment detected
rustc --version
echo.

:: Check Cargo.toml
if not exist "Cargo.toml" (
    echo [ERROR] Cargo.toml not found in current directory
    echo Please run this script from the project root directory
    pause
    exit /b 1
)

echo [INFO] Cargo.toml found
echo.

:: Check config directory
if not exist "config" (
    echo [WARN] Config directory not found
    echo Creating config directory...
    mkdir config
    echo [INFO] Config directory created
    echo.

    if not exist "config\default.toml" (
        echo [WARN] Configuration files not found
        echo Please ensure configuration files exist in config/
        echo.
    )
)

:: Rebuild if requested
if %REBUILD%==1 (
    echo [INFO] Rebuilding project...
    echo.
    cargo clean
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to clean build artifacts
        pause
        exit /b 1
    )
    echo.
    cargo build --release
    if %errorlevel% neq 0 (
        echo [ERROR] Build failed
        pause
        exit /b 1
    )
    echo [INFO] Rebuild complete
    echo.
) else (
    :: Build or check if binary exists
    if not exist "target\release\loquat.exe" (
        echo [INFO] Release binary not found, building...
        echo.
        cargo build --release
        if %errorlevel% neq 0 (
            echo [ERROR] Build failed
            pause
            exit /b 1
        )
        echo [INFO] Build complete
        echo.
    ) else (
        echo [INFO] Using existing release binary
        echo.
    )
)

:: Create logs directory if needed
if not exist "logs" (
    echo [INFO] Creating logs directory...
    mkdir logs
)

:: Create plugins directory if needed
if not exist "plugins" (
    echo [INFO] Creating plugins directory...
    mkdir plugins
)

:: Create adapters directory if needed
if not exist "adapters" (
    echo [INFO] Creating adapters directory...
    mkdir adapters
)

echo [INFO] Starting Loquat Framework...
echo ════════════════════════════════════════════════════════════
echo.

:: Run the application
if %ENV%==dev (
    target\release\loquat.exe %ENV%
) else (
    target\release\loquat.exe %ENV%
)

if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Application exited with error code %errorlevel%
    echo.
    pause
    exit /b %errorlevel%
)

echo.
echo [INFO] Application shut down successfully
pause
