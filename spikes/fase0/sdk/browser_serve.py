"""Fase 0 / Spike A, plano B: a mesma página (ui/index.html) num navegador de verdade.

O WebView2 recebeu HTTP 403 do servidor de licença Widevine do Spotify. Este servidor local
serve a página em http://127.0.0.1:8890/ para abrir no Chrome/Edge (de preferência em modo
--app com perfil próprio) e entrega o token salvo pelo Spike C (renovando se preciso).

    ..\\.venv\\Scripts\\python.exe browser_serve.py
    start chrome --app=http://127.0.0.1:8890/ --user-data-dir=%LOCALAPPDATA%\\spotron-spike\\chrome

Só escuta em 127.0.0.1. O log da página vai para spike_a.log, como no Tauri.
"""

import http.server
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "api"))
import api  # noqa: E402  (reaproveita o login/refresh do Spike C)

PORT = 8890
PAGE = (HERE / "ui" / "index.html").read_bytes()


class Handler(http.server.BaseHTTPRequestHandler):
    def _send(self, code, body, ctype="text/plain; charset=utf-8"):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path in ("/", "/index.html"):
            self._send(200, PAGE, "text/html; charset=utf-8")
        elif self.path == "/token":
            try:
                self._send(200, api.ensure_token(min_left=300).encode())
            except SystemExit as e:
                self._send(500, str(e).encode())
        else:
            self._send(404, b"")

    def do_POST(self):
        if self.path == "/log":
            msg = self.rfile.read(int(self.headers.get("Content-Length", 0))).decode(errors="replace")
            line = f"[{int(time.time())}] [browser] {msg}"
            print(line, flush=True)
            with open(HERE / "spike_a.log", "a", encoding="utf-8") as f:
                f.write(line + "\n")
            self._send(204, b"")
        else:
            self._send(404, b"")

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    print(f"Servindo em http://127.0.0.1:{PORT}/", flush=True)
    http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
