"""Fase 1 / otimização: mede a RAM do player (Edge --app) com combinações de flags.

Para cada configuração: abre o player.py com as flags, espera o "ready", transfere a reprodução
para ele pela Web API, confere pela Web API se a música está mesmo avançando (licença OK e áudio
saindo), mede a memória privada dos processos do perfil e fecha o Edge via DevTools.

    ..\\.venv\\Scripts\\python.exe ram_bench.py              (todas as configurações)
    ..\\.venv\\Scripts\\python.exe ram_bench.py base gpu     (só algumas)

Toca música de verdade na conta (uns 50 s por configuração).
"""

import asyncio
import json
import re
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

import websockets

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "api"))
import api  # noqa: E402

PY = str(HERE.parent / ".venv" / "Scripts" / "python.exe")
LOG = HERE / "spike_a.log"
PROFILE_MARK = r"spotron-spike\edge"

NO_GPU = "--disable-gpu --disable-gpu-compositing --disable-software-rasterizer"
LEAN = ("--disable-extensions --disable-sync --disable-default-apps --disable-breakpad "
        "--no-pings --disable-features=Translate,msEdgeShopping,msWalletCheckout,"
        "msEdgeSidebarV2,msEdgeCollections,msEdgeFeaturesSearchIndexer")
ONE_RENDERER = "--renderer-process-limit=1"
NO_ISOLATION = "--disable-site-isolation-trials --disable-features=IsolateOrigins,site-per-process"
SMALL_HEAP = "--js-flags=--max-old-space-size=96"

CONFIGS = {
    "base": "",
    "gpu": NO_GPU,
    "gpu+lean": f"{NO_GPU} {LEAN}",
    "gpu+lean+1rend": f"{NO_GPU} {LEAN} {ONE_RENDERER}",
    "gpu+lean+noiso": f"{NO_GPU} {LEAN} {NO_ISOLATION}",
    "tudo": f"{NO_GPU} {LEAN} {ONE_RENDERER} {NO_ISOLATION} {SMALL_HEAP}",
}

PS_MEM = r"""
$p = Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -like '*MARK*' }
$out = foreach ($x in $p) {
  $g = Get-Process -Id $x.ProcessId -ErrorAction SilentlyContinue
  if (-not $g) { continue }
  $t = if ($x.CommandLine -match '--type=(\S+)') { $Matches[1] } else { 'browser' }
  $u = if ($x.CommandLine -match '--utility-sub-type=(\S+)') { $Matches[1] } else { '' }
  [pscustomobject]@{ type = $t; sub = $u; priv = $g.PrivateMemorySize64; ws = $g.WorkingSet64 }
}
$out | ConvertTo-Json -Compress
""".replace("MARK", PROFILE_MARK)


def mem():
    raw = subprocess.run(["powershell", "-NoProfile", "-Command", PS_MEM],
                         capture_output=True, text=True).stdout.strip()
    rows = json.loads(raw) if raw else []
    return rows if isinstance(rows, list) else [rows]


def log_size():
    return LOG.stat().st_size if LOG.exists() else 0


def log_since(pos):
    with open(LOG, "rb") as f:
        f.seek(pos)
        return f.read().decode("utf-8", errors="replace")


async def cdp_close():
    try:
        v = json.loads(urllib.request.urlopen("http://127.0.0.1:9222/json/version", timeout=3).read())
        async with websockets.connect(v["webSocketDebuggerUrl"]) as ws:
            await ws.send(json.dumps({"id": 1, "method": "Browser.close"}))
            await asyncio.wait_for(ws.recv(), 5)
    except Exception:
        pass


def merge_features(flags):
    """O Chromium só respeita o último --disable-features: junta todos num só."""
    parts, feats = [], []
    for f in flags.split():
        if f.startswith("--disable-features="):
            feats += f.split("=", 1)[1].split(",")
        else:
            parts.append(f)
    if feats:
        parts.append("--disable-features=" + ",".join(dict.fromkeys(feats)))
    return " ".join(parts)


def run(name, flags):
    flags = merge_features(flags)
    print(f"\n=== {name}: {flags or '(sem flags)'}", flush=True)
    pos = log_size()
    proc = subprocess.Popen([PY, "-u", "player.py", "--debug", "--extra", flags], cwd=HERE,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    result = {"config": name, "flags": flags}
    try:
        dev = None
        for _ in range(60):
            m = re.search(r"ready: device_id=(\w+)", log_since(pos))
            if m:
                dev = m.group(1)
                break
            time.sleep(1)
        if not dev:
            result["erro"] = "player não ficou ready em 60 s"
            return result
        token = api.ensure_token(min_left=600)
        st, _, _ = api.call(token, "PUT", "/me/player", body={"device_ids": [dev], "play": True})
        result["transfer"] = st
        time.sleep(15)
        _, p1, _ = api.call(token, "GET", "/me/player")
        time.sleep(25)
        _, p2, _ = api.call(token, "GET", "/me/player")
        ok_dev = bool(p2) and p2.get("device", {}).get("id") == dev
        adv = (p2 or {}).get("progress_ms", 0) - (p1 or {}).get("progress_ms", 0)
        result["tocando"] = bool(ok_dev and (p2 or {}).get("is_playing") and adv > 15000)
        result["avanço_s"] = round(adv / 1000, 1)
        rows = mem()
        result["processos"] = len(rows)
        result["privada_MB"] = round(sum(r["priv"] for r in rows) / 2**20)
        result["ws_MB"] = round(sum(r["ws"] for r in rows) / 2**20)
        result["detalhe"] = {f"{r['type']}{':' + r['sub'].split('.')[-1] if r['sub'] else ''}":
                             round(r["priv"] / 2**20) for r in sorted(rows, key=lambda r: -r["priv"])}
        text = log_since(pos)
        result["playback_error"] = text.count("playback_error")
        result["outros_erros"] = len(re.findall(r"(initialization|authentication|account)_error", text))
        return result
    finally:
        asyncio.run(cdp_close())
        try:
            proc.wait(timeout=20)
        except subprocess.TimeoutExpired:
            proc.terminate()
        time.sleep(3)


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    names = sys.argv[1:] or list(CONFIGS)
    results = []
    for n in names:
        r = run(n, CONFIGS[n])
        results.append(r)
        print(json.dumps(r, ensure_ascii=False), flush=True)
    print("\nconfig              tocando  privada  procs  playback_error", flush=True)
    for r in results:
        print(f"{r['config']:<20}{str(r.get('tocando')):<9}{r.get('privada_MB', '-'):>7}  "
              f"{r.get('processos', '-'):>5}  {r.get('playback_error', '-')}  {r.get('erro', '')}", flush=True)
    (HERE / "ram_bench.json").write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
