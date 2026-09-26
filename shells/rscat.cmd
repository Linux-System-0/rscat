@echo off
REM rscat: rainbow cat — https://github.com/anomalyco/rscat
REM Added by `rscat --init cmd`. Run once to persist PATH for future cmd.exe:
REM   rscat --init cmd > %TEMP%\rscat-init.cmd & %TEMP%\rscat-init.cmd
set "RSCAT_BIN=%USERPROFILE%\.local\bin"
echo %PATH% | findstr /I /C:"%RSCAT_BIN%" >nul || setx PATH "%RSCAT_BIN%;%PATH%" >nul
