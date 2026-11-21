@echo off
chcp 65001 >nul
setlocal EnableDelayedExpansion
title Strata - publicar uma versao (release no GitHub)
cd /d "%~dp0"
set "PATH=%PATH%;%ProgramFiles%\Git\cmd;%ProgramFiles%\GitHub CLI"
for /f "usebackq delims=" %%V in (`powershell -NoProfile -Command "(Get-Content src-tauri\tauri.conf.json | ConvertFrom-Json).version"`) do set "VER=%%V"
set "TAG=v!VER!-beta"
echo Versao: !TAG!
echo Isso cria a release publica com instalador, portatil e checksums.
echo O repositorio precisa ser PUBLICO para o site conseguir baixar.
set /p OK=Continuar? (s/n): 
if /i not "!OK!"=="s" exit /b 0
git add -A
git commit -q -m "Release !TAG!" 2>nul
git push origin main || goto :erro
git tag -f "!TAG!" || goto :erro
git push origin "!TAG!" --force || goto :erro
for /f "usebackq delims=" %%U in (`gh api user -q .login`) do set "GHUSER=%%U"
echo.
echo Pronto. O GitHub esta compilando e vai publicar em:
echo   https://github.com/!GHUSER!/strata/releases
echo Leva uns 15 minutos.
pause
exit /b 0
:erro
echo [ERRO] Algo falhou acima.
pause
exit /b 1
