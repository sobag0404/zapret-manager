@echo off
set "BIN=%~dp0bin\"
set "LISTS=%~dp0lists\"

start "zapret: %~n0" /min "%BIN%winws.exe" --wf-tcp=443 --filter-tcp=443 --ipset="%LISTS%ipset-telegram-web-runtime.txt" --dpi-desync=syndata,fake,fakedsplit --dpi-desync-repeats=6 --dpi-desync-fooling=ts --dpi-desync-fakedsplit-pattern=0x00 --dpi-desync-fake-tls="%BIN%tls_clienthello_www_google_com.bin"
