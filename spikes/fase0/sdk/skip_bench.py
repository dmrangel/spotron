"""Fase 2 / diagnóstico: quais flags do Edge fazem o servidor de licença do Spotify responder 403?

Para cada conjunto de flags: abre o player.py (perfil do spike), transfere a reprodução, aperta
"próxima" N vezes e conta licenças 200/403 (via DevTools) e playback_error (log da página).

    ..\\.venv\\Scripts\\python.exe skip_bench.py [config ...]
"""

import asyncio
import json
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import websockets

import player
import ram_bench as rb

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "api"))
import api  # noqa: E402

OPT = player.OPTIMIZE_FLAGS
GPU = ["--disable-gpu", "--disable-gpu-compositing", "--disable-software-rasterizer"]
LEAN = ["--disable-extensions", "--disable-sync", "--disable-default-apps", "--disable-breakpad", "--no-pings",
        "--disable-features=Translate,msEdgeShopping,msWalletCheckout,msEdgeSidebarV2,msEdgeCollections,msEdgeFeaturesSearchIndexer"]
NOISO = ["--disable-site-isolation-trials", "--disable-features=IsolateOrigins,site-per-process"]
OFF = ["--window-position=-32000,-32000", "--window-size=480,360", "--disable-backgrounding-occluded-windows",
       "--disable-renderer-backgrounding", "--disable-background-timer-throttling", "--disable-features=CalculateNativeWinOcclusion"]

CONFIGS = {
    "nenhuma": [],
    "gpu": GPU,
    "lean": LEAN,
    "noiso": NOISO,
    "off": OFF,
    "app": OPT + OFF,
}

NEXTS = 6


async def count_licenses(seconds, dev):
    page = next(t for t in json.loads(urllib.request.urlopen("http://127.0.0.1:9222/json/list").read()) if t["type"] == "page")
    ok = bad = 0
    async with websockets.connect(page["webSocketDebuggerUrl"], max_size=None) as ws:
        await ws.send(json.dumps({"id": 1, "method": "Network.enable"}))
        t = api.ensure_token(min_left=600)
        print("transfer:", api.call(t, "PUT", "/me/player", body={"device_ids": [dev], "play": True})[0], flush=True)
        end = time.time() + seconds
        next_at = time.time() + 8
        nexts = 0
        while time.time() < end:
            if nexts < NEXTS and time.time() > next_at:
                api.call(t, "POST", "/me/player/next", [("device_id", dev)])
                nexts += 1
                next_at = time.time() + 5
            try:
                m = json.loads(await asyncio.wait_for(ws.recv(), 0.5))
            except asyncio.TimeoutError:
                continue
            if m.get("method") == "Network.responseReceived" and "widevine-license" in m["params"]["response"]["url"]:
                if m["params"]["response"]["status"] == 200:
                    ok += 1
                else:
                    bad += 1
    return ok, bad


def run(name, flags):
    flags = rb.merge_features(" ".join(flags))
    print(f"\n=== {name}: {flags or '(sem flags)'}", flush=True)
    pos = rb.log_size()
    proc = subprocess.Popen([rb.PY, "-u", "player.py", "--debug", "--extra", flags or "--no-first-run"], cwd=HERE,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        dev = None
        for _ in range(60):
            import re
            m = re.search(r"ready: device_id=(\w+)", rb.log_since(pos))
            if m:
                dev = m.group(1)
                break
            time.sleep(1)
        if not dev:
            return {"config": name, "erro": "sem ready"}
        ok, bad = asyncio.run(count_licenses(8 + NEXTS * 5 + 6, dev))
        errs = rb.log_since(pos).count("playback_error")
        return {"config": name, "licenca_200": ok, "licenca_403": bad, "playback_error": errs}
    finally:
        asyncio.run(rb.cdp_close())
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.terminate()
        time.sleep(3)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    res = [run(n, CONFIGS[n]) for n in (sys.argv[1:] or CONFIGS)]
    print("\nconfig      200  403  playback_error")
    for r in res:
        print(f"{r['config']:<10}{r.get('licenca_200', '-'):>5}{r.get('licenca_403', '-'):>5}{r.get('playback_error', '-'):>8}  {r.get('erro', '')}")
