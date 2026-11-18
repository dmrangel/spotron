@echo off
REM Fase 0: librespot -> pipe -> CamillaDSP -> WASAPI
REM Rodar no cmd.exe (NUNCA no PowerShell 5: ele corrompe pipes binários).
REM Requer librespot.exe e camilladsp.exe no PATH ou nesta pasta.
REM Logs: librespot.log e camilladsp.log nesta pasta (stderr separado do áudio no stdout).

cd /d "%~dp0"

REM --backend pipe sem --device => PCM no stdout. --format S16 = S16LE 44100 Hz 2ch (fixo no librespot 0.8).
REM --initial-volume 100: o volume do celular continua funcionando (softvol), mas começa sem atenuação.
REM -p 1234: WebSocket do CamillaDSP para o ws_test.py.
librespot.exe --name "Spotron Fase0" --backend pipe --format S16 --bitrate 320 --initial-volume 100 2> librespot.log ^
  | camilladsp.exe -p 1234 -v fase0.yml 2> camilladsp.log
