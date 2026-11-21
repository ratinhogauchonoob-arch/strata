@echo off
chcp 65001 >nul
setlocal EnableDelayedExpansion
title Strata - atualizar o repositorio no GitHub
cd /d "%~dp0"
set "PATH=%PATH%;%ProgramFiles%\Git\cmd;%ProgramFiles%\GitHub CLI"
where gh >nul 2>nul || (echo [ERRO] GitHub CLI nao encontrado. Rode compilar_na_nuvem.bat uma vez. & pause & exit /b 1)
gh auth status >nul 2>nul || gh auth login --hostname github.com --git-protocol https --web
for /f "usebackq delims=" %%U in (`gh api user -q .login`) do set "GHUSER=%%U"
set "REPO=!GHUSER!/strata"
set /p SITE=<site-url.txt

if not exist ".git" (
  git init -q -b main
  git remote add origin "https://github.com/!REPO!.git"
)
git config user.name "!GHUSER!"
git config user.email "!GHUSER!@users.noreply.github.com"
git add -A
git commit -q -m "Repository: README, license, templates, docs" 2>nul
git push -u origin main || goto :erro

echo Configurando descricao, site e topicos...
gh repo edit "!REPO!" --description "A calm, open-source game launcher for the KytyPS5 engine on Windows. Auto-tuned per PC, memory limit, per-game settings, developer mode." --homepage "!SITE!" --add-topic launcher --add-topic tauri --add-topic rust --add-topic emulator-frontend --add-topic windows --add-topic kytyps5 --enable-issues --enable-wiki=false >nul || echo [AVISO] Nao consegui mudar a descricao.

for /f "usebackq delims=" %%V in (`gh repo view "!REPO!" --json visibility -q .visibility`) do set "VIS=%%V"
echo Visibilidade atual: !VIS!
if /i "!VIS!"=="PRIVATE" (
  echo.
  echo Para o site mostrar o botao de download, o repo precisa ser PUBLICO.
  echo Publico = qualquer um ve o codigo. Isso tambem cumpre a GPL.
  set /p PUB=Deixar PUBLICO agora? s/n: 
  if /i "!PUB!"=="s" (
    gh repo edit "!REPO!" --visibility public --accept-visibility-change-consequences && echo Agora e publico.
  )
)
echo.
echo Pronto: https://github.com/!REPO!
pause
exit /b 0
:erro
echo [ERRO] Algo falhou acima.
pause
exit /b 1
