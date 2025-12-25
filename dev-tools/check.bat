@echo off
chcp 65001 >nul

:: Loquat Framework - Check Script
:: Runs cargo check, clippy, and tests

echo.
echo ╔════════════════════════════════════════════════════════════╗
echo ║                Loquat Framework - Check                    ║
echo ╚════════════════════════════════════════════════════════════╝
echo.

echo [INFO] Running cargo check...
cargo check
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Cargo check failed
    pause
    exit /b 1
)
echo [SUCCESS] Cargo check passed
echo.

echo [INFO] Running clippy...
cargo clippy -- -D warnings
if %errorlevel% neq 0 (
    echo.
    echo [WARN] Clippy found issues (treated as errors)
    pause
    exit /b 1
)
echo [SUCCESS] Clippy checks passed
echo.

echo [INFO] Running tests...
cargo test
if %errorlevel% neq 0 (
    echo.
    echo [ERROR] Tests failed
    pause
    exit /b 1
)
echo [SUCCESS] All tests passed
echo.

echo [SUCCESS] All checks completed successfully!
echo.
