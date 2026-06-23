@echo off
if "%~1"=="status_zapret" (
    chcp 437 > nul
    netsh interface tcp show global | findstr /i "timestamps" | findstr /i "enabled" > nul || netsh interface tcp set global timestamps=enabled > nul 2>&1
    exit /b
)
if "%~1"=="check_updates" exit /b
if "%~1"=="load_game_filter" (
    set "GameFilterTCP="
    set "GameFilterUDP="
    exit /b
)
if "%~1"=="load_user_lists" (
    if not exist "%~dp0lists\ipset-exclude-user.txt" echo 203.0.113.113/32>"%~dp0lists\ipset-exclude-user.txt"
    if not exist "%~dp0lists\list-general-user.txt" echo # Never leave this file empty>"%~dp0lists\list-general-user.txt"
    if not exist "%~dp0lists\list-exclude-user.txt" echo domain.example.abc>"%~dp0lists\list-exclude-user.txt"
    exit /b
)
exit /b
