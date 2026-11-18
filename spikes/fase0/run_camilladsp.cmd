@echo off
REM Fase 0 / Spike B: CamillaDSP capturando do VB-CABLE e tocando nos fones.
REM Requer camilladsp.exe (4.1.3) nesta pasta ou no PATH e o VB-CABLE instalado.
REM -p 1234: WebSocket para o ws_test.py. Log em camilladsp.log nesta pasta.

cd /d "%~dp0"
camilladsp.exe -p 1234 -v vbcable.yml 2> camilladsp.log
