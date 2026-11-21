@echo off
chcp 65001 >nul
setlocal EnableDelayedExpansion
title Strata - trocar o endereco do site no GitHub
cd /d "%~dp0"
set /p OLD=<site-url.txt
echo Endereco atual: !OLD!
set /p NEW=Novo endereco do site (ex: https://strata.seudominio.com): 
if "!NEW!"=="" exit /b 0
if "!NEW:~-1!"=="/" set "NEW=!NEW:~0,-1!"
powershell -NoProfile -Command ^
  "$old='%OLD%'; $new='%NEW%'; $oldHost=([uri]$old).Host; $newHost=([uri]$new).Host;" ^
  "foreach($f in 'README.md','CONTRIBUTING.md','.github\ISSUE_TEMPLATE\config.yml'){" ^
  "  $t=[IO.File]::ReadAllText($f);" ^
  "  $t=$t.Replace($old,$new).Replace('>'+$oldHost+'<','>'+$newHost+'<').Replace('['+$oldHost+'/guide]','['+$newHost+'/guide]');" ^
  "  $t=$t -replace ' <i>\(home test server[^<]*</i>','';" ^
  "  [IO.File]::WriteAllText($f,$t,(New-Object Text.UTF8Encoding $false)) }"
if errorlevel 1 (echo [ERRO] Nao consegui trocar. & pause & exit /b 1)
>site-url.txt echo !NEW!
echo Pronto: README, CONTRIBUTING e modelos de issue agora apontam para !NEW!
echo Rode atualizar_github.bat para enviar.
pause
