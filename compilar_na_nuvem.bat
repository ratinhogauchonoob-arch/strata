@echo off
chcp 65001 >nul
setlocal EnableDelayedExpansion
title Strata - compilar na nuvem (repo PRIVADO de builds)
cd /d "%~dp0"
echo ============================================================
echo  Compila o Strata nos PCs do GitHub, no repo PRIVADO strata-builds.
echo  So voce ve. O repo publico "strata" NAO e tocado.
echo  Cada build fica guardado em strata-builds - Releases (build-1, build-2...).
echo  Leva uns 10-20 min. No fim aparece a pasta Strata-pronto.
echo ============================================================
set "WG=%LOCALAPPDATA%\Microsoft\WindowsApps\winget.exe"
if not exist "%WG%" set "WG=winget"
set "PATH=%PATH%;%ProgramFiles%\Git\cmd;%ProgramFiles%\GitHub CLI"
where git >nul 2>nul || "%WG%" install -e --id Git.Git --accept-source-agreements --accept-package-agreements
where gh  >nul 2>nul || "%WG%" install -e --id GitHub.cli --accept-source-agreements --accept-package-agreements
where git >nul 2>nul || (echo [ERRO] Git nao instalado. Feche e abra de novo. & pause & exit /b 1)
where gh  >nul 2>nul || (echo [ERRO] GitHub CLI nao instalado. Feche e abra de novo. & pause & exit /b 1)

gh auth status >nul 2>nul || (
  echo O navegador vai abrir. O codigo aparece AQUI nesta janela.
  gh auth login --hostname github.com --git-protocol https --web || goto :erro
)
gh auth setup-git >nul 2>nul
for /f "usebackq delims=" %%U in (`gh api user -q .login`) do set "GHUSER=%%U"
if not defined GHUSER goto :erro
set "REPO=!GHUSER!/strata-builds"
echo Conta: !GHUSER!

gh repo view "!REPO!" >nul 2>nul || (
  echo Criando repositorio PRIVADO !REPO! so para os builds...
  gh repo create "!REPO!" --private --description "Strata - builds de teste (privado)" >nul || goto :erro
)

if not exist ".git" (
  git init -q -b main || goto :erro
  git remote add origin "https://github.com/!GHUSER!/strata.git"
)
git remote get-url builds >nul 2>nul && git remote set-url builds "https://github.com/!REPO!.git"
git remote get-url builds >nul 2>nul || git remote add builds "https://github.com/!REPO!.git"
git config user.name "!GHUSER!"
git config user.email "!GHUSER!@users.noreply.github.com"
git add -A
git commit -q -m "Strata build %date% %time%" 2>nul
rem Envia so para o repo privado de builds (o publico fica igual ate voce usar atualizar_github.bat)
git push builds HEAD:main --force || goto :erro

echo Esperando o GitHub comecar...
timeout /t 15 >nul
set "RUN="
for /f "usebackq delims=" %%R in (`gh run list -R "!REPO!" -w build.yml -L 1 --json databaseId -q ".[0].databaseId"`) do set "RUN=%%R"
if not defined RUN (
  gh workflow run build.yml -R "!REPO!" --ref main
  timeout /t 15 >nul
  for /f "usebackq delims=" %%R in (`gh run list -R "!REPO!" -w build.yml -L 1 --json databaseId -q ".[0].databaseId"`) do set "RUN=%%R"
)
if not defined RUN (echo [ERRO] O build nao comecou. Veja https://github.com/!REPO!/actions & pause & exit /b 1)

echo Compilando: https://github.com/!REPO!/actions/runs/!RUN!
echo Pode minimizar. Nao feche.
gh run watch !RUN! -R "!REPO!" --interval 30 --exit-status
if errorlevel 1 (
  echo.
  echo [ERRO] O build falhou. Abra o link acima, clique no passo vermelho e mande print pro Claude.
  pause & exit /b 1
)

if exist "%~dp0Strata-pronto" rmdir /s /q "%~dp0Strata-pronto"
gh run download !RUN! -R "!REPO!" -n Strata-Windows -D "%~dp0Strata-pronto" || goto :erro
echo.
echo ============================================================
echo  PRONTO!  Strata-pronto\Strata\Strata.exe  (versao portatil)
echo  ou o instalador Strata_*_x64-setup.exe na mesma pasta.
echo  Historico de builds: https://github.com/!REPO!/releases
echo ============================================================
start "" explorer "%~dp0Strata-pronto"
pause
exit /b 0
:erro
echo.
echo [ERRO] Algo falhou acima. Mande o texto pro Claude.
pause
exit /b 1
