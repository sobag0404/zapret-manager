@echo off
set "BIN=%~dp0bin\"
set "LISTS=%~dp0lists\"

start "zapret: %~n0" /min "%BIN%winws.exe" --debug=@"%LISTS%..\winws-debug.log" --wf-tcp=443 --filter-tcp=443 --ipset="%LISTS%ipset-telegram-web-runtime.txt" --wssize=1:6
