"""Fase 0 / Spike C: login OAuth (Authorization Code + PKCE) e testes da Web API.

Só biblioteca padrão do Python. Tokens ficam em %APPDATA%\\spotron-spike\\ (fora do repositório).

Uso:
    python api.py login --client-id <CLIENT_ID>   # abre o navegador; salva o Client ID e os tokens
    python api.py token                           # renova o access token se faltar < 10 min
    python api.py test                            # testes só de leitura
    python api.py test --write                    # também transfere para "Spotron Fase0" e põe 1 faixa na fila

Pré-requisito: app em developer.spotify.com/dashboard com o redirect URI
http://127.0.0.1:8888/callback (o Spotify não aceita "localhost").
Endpoints conferidos contra o changelog de fev/2026 da Web API (modo de desenvolvimento).
"""

import argparse
import base64
import hashlib
import http.server
import json
import os
import secrets
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
import webbrowser
from pathlib import Path

REDIRECT_URI = "http://127.0.0.1:8888/callback"
AUTH_URL = "https://accounts.spotify.com/authorize"
TOKEN_URL = "https://accounts.spotify.com/api/token"
API = "https://api.spotify.com/v1"
SCOPES = " ".join([
    "streaming",                      # Web Playback SDK
    "user-read-email",                # exigido pelo SDK
    "user-read-private",              # exigido pelo SDK
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-currently-playing",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-library-read",
])
DEVICE_NAME = "Spotron Fase0"

STORE = Path(os.environ.get("APPDATA", Path.home())) / "spotron-spike"
CONFIG_FILE = STORE / "config.json"
TOKENS_FILE = STORE / "tokens.json"


# ---------------------------------------------------------------- armazenamento

def load_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return None


def save_json(path, data):
    STORE.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2), encoding="utf-8")


def client_id():
    cfg = load_json(CONFIG_FILE) or {}
    if not cfg.get("client_id"):
        sys.exit("Sem Client ID. Rode primeiro: python api.py login --client-id <CLIENT_ID>")
    return cfg["client_id"]


# ---------------------------------------------------------------- OAuth PKCE

def post_token(form):
    data = urllib.parse.urlencode(form).encode()
    req = urllib.request.Request(TOKEN_URL, data=data, method="POST",
                                 headers={"Content-Type": "application/x-www-form-urlencoded"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return json.loads(r.read())
    except urllib.error.HTTPError as e:
        sys.exit(f"Erro no endpoint de token: HTTP {e.code} {e.read().decode(errors='replace')}")


def store_tokens(resp, old=None):
    tokens = {
        "access_token": resp["access_token"],
        # o Spotify pode não devolver um refresh token novo na renovação: manter o antigo
        "refresh_token": resp.get("refresh_token") or (old or {}).get("refresh_token"),
        "scope": resp.get("scope"),
        "expires_at": int(time.time()) + int(resp.get("expires_in", 3600)),
    }
    save_json(TOKENS_FILE, tokens)
    return tokens


def login(cid):
    save_json(CONFIG_FILE, {"client_id": cid})
    verifier = base64.urlsafe_b64encode(secrets.token_bytes(64)).rstrip(b"=").decode()
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    state = secrets.token_urlsafe(16)
    result = {}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            url = urllib.parse.urlparse(self.path)
            if url.path != "/callback":
                self.send_response(404)
                self.end_headers()
                return
            q = urllib.parse.parse_qs(url.query)
            result.update({k: v[0] for k, v in q.items()})
            ok = "code" in q and q.get("state", [""])[0] == state
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.end_headers()
            msg = "Login concluído. Pode fechar esta aba." if ok else f"Falhou: {result}"
            self.wfile.write(f"<p>{msg}</p>".encode())
            threading.Thread(target=self.server.shutdown, daemon=True).start()

        def log_message(self, *args):
            pass

    server = http.server.HTTPServer(("127.0.0.1", 8888), Handler)
    params = {
        "client_id": cid,
        "response_type": "code",
        "redirect_uri": REDIRECT_URI,
        "code_challenge_method": "S256",
        "code_challenge": challenge,
        "state": state,
        "scope": SCOPES,
    }
    url = AUTH_URL + "?" + urllib.parse.urlencode(params)
    print("Abrindo o navegador para o login do Spotify...\nSe não abrir, cole esta URL:\n" + url)
    webbrowser.open(url)
    server.serve_forever()

    if result.get("state") != state or "code" not in result:
        sys.exit(f"Login falhou: {result.get('error', result)}")
    resp = post_token({
        "grant_type": "authorization_code",
        "code": result["code"],
        "redirect_uri": REDIRECT_URI,
        "client_id": cid,
        "code_verifier": verifier,
    })
    tokens = store_tokens(resp)
    print(f"OK. Tokens salvos em {TOKENS_FILE}")
    print(f"Escopos concedidos: {tokens['scope']}")


def ensure_token(min_left=600):
    tokens = load_json(TOKENS_FILE)
    if not tokens:
        sys.exit("Sem tokens. Rode: python api.py login --client-id <CLIENT_ID>")
    left = tokens["expires_at"] - time.time()
    if left < min_left:
        resp = post_token({
            "grant_type": "refresh_token",
            "refresh_token": tokens["refresh_token"],
            "client_id": client_id(),
        })
        tokens = store_tokens(resp, tokens)
        print(f"Access token renovado (válido por {tokens['expires_at'] - int(time.time())} s).")
    else:
        print(f"Access token ainda válido por {int(left)} s.")
    return tokens["access_token"]


# ---------------------------------------------------------------- Web API

def call(token, method, path, params=None, body=None):
    url = API + path + ("?" + urllib.parse.urlencode(params) if params else "")
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method,
                                 headers={"Authorization": f"Bearer {token}",
                                          "Content-Type": "application/json"})
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            raw = r.read()
            status = r.status
    except urllib.error.HTTPError as e:
        raw, status = e.read(), e.code
    ms = (time.perf_counter() - t0) * 1000
    try:
        payload = json.loads(raw) if raw else None
    except json.JSONDecodeError:
        payload = raw.decode(errors="replace")
    return status, payload, ms


results = []


def check(token, label, method, path, params=None, body=None, summarize=None):
    status, payload, ms = call(token, method, path, params, body)
    ok = 200 <= status < 300
    if ok and summarize:
        try:
            info = summarize(payload)
        except Exception as e:  # resumo é só para leitura humana
            info = f"(resumo falhou: {e})"
    elif ok:
        info = ""
    else:
        info = json.dumps(payload, ensure_ascii=False)[:200]
    print(f"[{'OK ' if ok else 'ERR'}] {status} {ms:6.0f} ms  {label}  {info}")
    results.append((label, status, ok))
    return status, payload


def names(items, key="name"):
    return ", ".join(str(i.get(key)) for i in items[:3])


def test(write):
    token = ensure_token()
    print()

    _, me = check(token, "GET /me", "GET", "/me",
                  summarize=lambda p: f"id={p.get('id')}")
    my_id = (me or {}).get("id")

    check(token, "GET /search limit=10", "GET", "/search",
          {"q": "radiohead", "type": "track", "limit": 10},
          summarize=lambda p: f"{len(p['tracks']['items'])} itens: " + names(p["tracks"]["items"]))
    check(token, "GET /search limit=20 (esperado falhar: máx. 10 desde fev/2026)", "GET", "/search",
          {"q": "radiohead", "type": "track", "limit": 20},
          summarize=lambda p: f"{len(p['tracks']['items'])} itens")
    check(token, "GET /search offset=10 (paginação)", "GET", "/search",
          {"q": "radiohead", "type": "track", "limit": 10, "offset": 10},
          summarize=lambda p: f"{len(p['tracks']['items'])} itens")

    _, pls = check(token, "GET /me/playlists", "GET", "/me/playlists", {"limit": 50},
                   summarize=lambda p: f"{p.get('total')} playlists: " + names(p["items"]))
    items = (pls or {}).get("items") or []
    own = next((p for p in items if p and p.get("owner", {}).get("id") == my_id), None)
    other = next((p for p in items if p and p.get("owner", {}).get("id") != my_id
                  and not p.get("collaborative")), None)
    if own:
        check(token, f"GET /playlists/{{id}}/items (própria: {own['name']})", "GET",
              f"/playlists/{own['id']}/items", {"limit": 5},
              summarize=lambda p: f"{p.get('total')} itens")
    else:
        print("[---] nenhuma playlist própria para testar /items")
    if other:
        check(token, f"GET /playlists/{{id}}/items (seguida, não própria: {other['name']})", "GET",
              f"/playlists/{other['id']}/items", {"limit": 5},
              summarize=lambda p: f"{p.get('total')} itens")
    else:
        print("[---] nenhuma playlist seguida (não própria) para testar a restrição")

    _, liked = check(token, "GET /me/tracks (curtidas)", "GET", "/me/tracks", {"limit": 5},
                     summarize=lambda p: f"{p.get('total')} curtidas: "
                     + names([i["track"] for i in p["items"]]))
    check(token, "GET /me/albums", "GET", "/me/albums", {"limit": 5},
          summarize=lambda p: f"{p.get('total')} álbuns")

    _, devs = check(token, "GET /me/player/devices", "GET", "/me/player/devices",
                    summarize=lambda p: names(p["devices"]))
    check(token, "GET /me/player", "GET", "/me/player",
          summarize=lambda p: "sem reprodução ativa" if not p else
          f"device={p['device']['name']} tocando={p.get('is_playing')}")
    check(token, "GET /me/player/queue", "GET", "/me/player/queue",
          summarize=lambda p: f"{len(p.get('queue', []))} na fila")

    if write:
        dev = next((d for d in (devs or {}).get("devices", []) if d.get("name") == DEVICE_NAME), None)
        if dev:
            check(token, f"PUT /me/player (transferir para {DEVICE_NAME})", "PUT", "/me/player",
                  body={"device_ids": [dev["id"]], "play": True})
            time.sleep(2)
        else:
            print(f"[---] dispositivo '{DEVICE_NAME}' não encontrado: abra o Spike A antes do --write")
        first = ((liked or {}).get("items") or [{}])[0].get("track")
        if first:
            check(token, f"POST /me/player/queue ({first['name']})", "POST", "/me/player/queue",
                  {"uri": first["uri"]})

    failed = [r for r in results if not r[2]]
    print(f"\nResumo: {len(results) - len(failed)} OK, {len(failed)} com erro.")
    for label, status, _ in failed:
        print(f"  - {status} {label}")


def main():
    # Acentos corretos também quando a saída vai para pipe/arquivo (code page do Windows).
    sys.stdout.reconfigure(encoding="utf-8", errors="replace")
    ap =argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    lg = sub.add_parser("login")
    lg.add_argument("--client-id", help="Client ID do app no dashboard (salvo para os próximos usos)")
    sub.add_parser("token")
    ts = sub.add_parser("test")
    ts.add_argument("--write", action="store_true", help="também transfere a reprodução e mexe na fila")
    args = ap.parse_args()

    if args.cmd == "login":
        login(args.client_id or client_id())
    elif args.cmd == "token":
        ensure_token()
    elif args.cmd == "test":
        test(args.write)


if __name__ == "__main__":
    main()
