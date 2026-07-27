@echo off
set "BIN=%~dp0bin\"
set "LISTS=%~dp0lists\"

start "zapret: %~n0" /min "%BIN%winws.exe" --wf-tcp=443 --filter-tcp=443 --ipset="%LISTS%ipset-telegram-web-runtime.txt" --dup=1 --dup-cutoff=n1