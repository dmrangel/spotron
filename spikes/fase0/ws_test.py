"""Fase 0: altera parâmetros do CamillaDSP em tempo real via WebSocket.

Uso (com o run.cmd rodando e música tocando):
    pip install websockets
    python ws_test.py

Comandos conferidos em src/socketserver.rs do CamillaDSP v4.1.3:
    {"SetConfigValue": ["<json pointer>", valor]}
    {"GetConfigValue": "<json pointer>"}
"""

import asyncio
import json
import time

import websockets

URL = "ws://127.0.0.1:1234"


async def cmd(ws, payload):
    t0 = time.perf_counter()
    await ws.send(json.dumps(payload))
    reply = await ws.recv()
    dt = (time.perf_counter() - t0) * 1000
    print(f"{dt:6.1f} ms  {json.dumps(payload)}  ->  {reply}")
    return json.loads(reply)


async def main():
    async with websockets.connect(URL) as ws:
        await cmd(ws, "GetState")
        await cmd(ws, "GetCaptureRate")

        print("\n1) Liga/desliga o passa-baixa trocando a frequência (500 Hz <-> 20 kHz).")
        for freq in (500, 20000, 500, 20000):
            await cmd(ws, {"SetConfigValue": ["/filters/lowpass_test/parameters/freq", freq]})
            await asyncio.sleep(3)

        print("\n2) Varre o ganho do peaking em 1 kHz (+12 dB e volta). Deve soar sem cortes.")
        for gain in (0, 3, 6, 9, 12, 6, 0):
            await cmd(ws, {"SetConfigValue": ["/filters/peq_test/parameters/gain", gain]})
            await asyncio.sleep(1)

        print("\nDiagnóstico:")
        await cmd(ws, "GetClippedSamples")
        await cmd(ws, "GetProcessingLoad")
        await cmd(ws, "GetState")


if __name__ == "__main__":
    asyncio.run(main())
