"""Fase 0 / Spike A, plano B: abre o player "Spotron Fase0" (Chrome em modo app) de uma vez.

Sobe o servidor local (página + token, ver browser_serve.py) numa thread e abre o Chrome em
--app com perfil próprio. O servidor vive enquanto a janela do Chrome estiver aberta, então a
reprodução pode ir para o app oficial e voltar para o "Spotron Fase0" quantas vezes for preciso.

    ..\\.venv\\Scripts\\python.exe player.py            (Edge: padrão, existe em todo Windows)
    ..\\.venv\\Scripts\\python.exe player.py --chrome   (Chrome, para comparar)
    ..\\.venv\\Scripts\\python.exe player.py --debug    (DevTools em 9222, para o cdp_watch.py)
"""

import argparse
import http.server
import os
import subprocess
import sys
import threading
from pathlib import Path

import browser_serve

OPTIMIZE_FLAGS = [
    "--disable-gpu", "--disable-gpu-compositing", "--disable-software-rasterizer",
    "--disable-extensions", "--disable-sync", "--disable-default-apps", "--disable-breakpad", "--no-pings",
    "--disable-site-isolation-trials",
    "--disable-features=Translate,msEdgeShopping,msWalletCheckout,msEdgeSidebarV2,msEdgeCollections,"
    "msEdgeFeaturesSearchIndexer,IsolateOrigins,site-per-process",
]

BROWSERS = {
    "chrome": r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "edge": r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
}


def main():
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    ap = argparse.ArgumentParser()
    ap.add_argument("--chrome", action="store_true")
    ap.add_argument("--debug", action="store_true")
    ap.add_argument("--extra", default="", help="flags extras do navegador, separadas por espaço (benchmark de RAM)")
    ap.add_argument("--no-optimize", action="store_true", help="sem as flags de economia de RAM")
    args = ap.parse_args()

    name = "chrome" if args.chrome else "edge"
    profile = Path(os.environ["LOCALAPPDATA"]) / "spotron-spike" / name
    url = f"http://127.0.0.1:{browser_serve.PORT}/"

    try:
        server = http.server.ThreadingHTTPServer(("127.0.0.1", browser_serve.PORT), browser_serve.Handler)
    except OSError:
        sys.exit(f"Porta {browser_serve.PORT} ocupada: já existe um player aberto?")
    threading.Thread(target=server.serve_forever, daemon=True).start()
    print(f"Servidor em {url}", flush=True)

    cmd = [
        BROWSERS[name],
        f"--app={url}",
        f"--user-data-dir={profile}",
        "--autoplay-policy=no-user-gesture-required",  # tocar ao ser escolhido no celular, sem clique
        "--no-first-run",
        "--no-default-browser-check",
        "--window-size=960,760",
    ]
    if args.debug:
        cmd.append("--remote-debugging-port=9222")
    if not args.no_optimize and not args.extra:
        # Vencedora do ram_bench.py (17/11/2026): 403 -> ~213 MB privados, música tocando, 0 erros.
        # Sem GPU (a janela não tem vídeo), sem serviços extras e sem isolamento de sites
        # (o perfil só abre a página local do player com o iframe do Spotify).
        cmd += OPTIMIZE_FLAGS
    cmd += args.extra.split()
    print(f"Abrindo {name} em modo app (perfil {profile})", flush=True)
    # Com --user-data-dir próprio, este processo é o navegador em si: wait() volta quando a janela fecha.
    subprocess.run(cmd)
    print("Janela fechada; encerrando o servidor.", flush=True)
    server.shutdown()


if __name__ == "__main__":
    main()
