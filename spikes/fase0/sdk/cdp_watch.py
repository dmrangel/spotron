"""Fase 0 / Spike A: diagnóstico via Chrome DevTools Protocol do WebView2.

Abra o app com a depuração remota ligada (cmd):
    set WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
    target\\release\\spotron-fase0-sdk.exe
e rode, com o .venv de spikes/fase0:
    ..\\.venv\\Scripts\\python.exe cdp_watch.py

Imprime mensagens de console, exceções e requisições com falha/HTTP >= 400 da página e dos
iframes (o player do SDK roda num iframe de sdk.scdn.co). Não imprime corpos nem cabeçalhos
(têm token).
"""

import asyncio
import json
import sys
import urllib.request

import websockets

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9222
seen = set()


def short(url, n=140):
    return url.split("?")[0][:n]


async def watch(target):
    ws_url = target["webSocketDebuggerUrl"]
    tag = f"{target['type']}:{short(target['url'], 50)}"
    async with websockets.connect(ws_url, max_size=None) as ws:
        msg_id = 0

        async def send(method, params=None):
            nonlocal msg_id
            msg_id += 1
            await ws.send(json.dumps({"id": msg_id, "method": method, "params": params or {}}))

        for m in ("Runtime.enable", "Log.enable", "Network.enable"):
            await send(m)
        print(f"== conectado a {tag}", flush=True)
        reqs = {}
        async for raw in ws:
            ev = json.loads(raw)
            method, p = ev.get("method"), ev.get("params", {})
            if method == "Network.requestWillBeSent":
                reqs[p["requestId"]] = p["request"]["url"]
            elif method == "Network.responseReceived":
                st = p["response"]["status"]
                if st >= 400:
                    print(f"[{tag}] HTTP {st} {p['response'].get('statusText','')} {short(p['response']['url'])}", flush=True)
                elif any(k in p["response"]["url"] for k in ("license", "widevine", "audio", "seektable", "storage-resolve")):
                    print(f"[{tag}] HTTP {st} {short(p['response']['url'])}", flush=True)
            elif method == "Network.loadingFailed":
                print(f"[{tag}] FALHOU {p.get('errorText')} {p.get('blockedReason','')} "
                      f"{short(reqs.get(p['requestId'], '?'))}", flush=True)
            elif method == "Runtime.consoleAPICalled":
                args = " ".join(str(a.get("value", a.get("description", ""))) for a in p.get("args", []))
                print(f"[{tag}] console.{p.get('type')}: {args[:400]}", flush=True)
            elif method == "Runtime.exceptionThrown":
                d = p["exceptionDetails"]
                desc = d.get("exception", {}).get("description") or d.get("text")
                print(f"[{tag}] EXCEÇÃO: {str(desc)[:400]}", flush=True)
            elif method == "Log.entryAdded":
                e = p["entry"]
                print(f"[{tag}] log.{e['level']}: {e['text'][:400]} {short(e.get('url',''))}", flush=True)


async def main():
    tasks = {}
    while True:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/list", timeout=5) as r:
                targets = json.loads(r.read())
        except Exception as e:
            print(f"sem CDP em {PORT}: {e}", flush=True)
            await asyncio.sleep(3)
            continue
        for t in targets:
            if t["type"] in ("page", "iframe") and t["id"] not in tasks and "webSocketDebuggerUrl" in t:
                tasks[t["id"]] = asyncio.create_task(watch(t))
        await asyncio.sleep(2)


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    asyncio.run(main())
