//! Janela de reprodução: o Spotify recusa a licença Widevine do WebView2 do Tauri (Spike A), então o
//! Web Playback SDK roda numa página local aberta no Edge em modo --app, com perfil próprio.
//!
//! - Servidor local (127.0.0.1:PLAYER_PORT): serve a página do player, entrega o token (/token) e
//!   recebe o estado do SDK (/state) e logs (/log), repassados à UI como eventos Tauri.
//! - Edge: flags validadas no ram_bench.py (403 -> ~213 MB), preso a um Job Object (procjob.rs):
//!   ao fechar (ou travar) o Spotron, o Edge do player morre junto.

use crate::config;
use crate::AppState;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub const PLAYER_PORT: u16 = 8891;
// O nome do dispositivo ("Spotron") fica na página do player.
const PAGE: &str = include_str!("../player/player.html");

const EDGE_FLAGS: &[&str] = &[
    "--autoplay-policy=no-user-gesture-required",
    "--no-first-run",
    "--no-default-browser-check",
    "--hide-crash-restore-bubble",
    // Fora da tela: o player não precisa ser visto (a UI é o Tauri).
    "--window-position=-32000,-32000",
    "--window-size=480,360",
    // Fora da tela a página fica "hidden" e o SDK não começava a tocar (17/11/2026): impedir que o
    // Edge suspenda/estrangule janelas ocultas.
    "--disable-backgrounding-occluded-windows",
    "--disable-renderer-backgrounding",
    "--disable-background-timer-throttling",
    // Não usar --disable-component-update: ele desliga o Widevine inteiro (initialization_error).
    // Economia de RAM (ram_bench.py, 17/11/2026).
    "--disable-gpu",
    "--disable-gpu-compositing",
    "--disable-software-rasterizer",
    "--disable-extensions",
    "--disable-sync",
    "--disable-default-apps",
    "--disable-breakpad",
    "--no-pings",
    "--disable-site-isolation-trials",
    "--disable-features=Translate,msEdgeShopping,msWalletCheckout,msEdgeSidebarV2,msEdgeCollections,msEdgeFeaturesSearchIndexer,IsolateOrigins,site-per-process,CalculateNativeWinOcclusion",
];

fn edge_path() -> Option<PathBuf> {
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(std::env::var_os)
        .map(|p| PathBuf::from(p).join(r"Microsoft\Edge\Application\msedge.exe"))
        .find(|p| p.exists())
}

pub type Edge = crate::procjob::Managed;

/// O Edge baixa sozinho um Widevine mais novo para `<perfil>\WidevineCdm` (4.10.3112.0 em
/// 17/11/2026) e o servidor de licença do Spotify responde 403 para ele (faixas pulando); o embutido
/// no Edge (4.10.3050.1) funciona. Remove a pasta baixada e põe um ARQUIVO vazio com o mesmo nome:
/// o component updater não consegue instalar e o Edge segue com o embutido.
fn pin_bundled_widevine(app: &AppHandle, profile: &std::path::Path) {
    let p = profile.join("WidevineCdm");
    if p.is_dir() {
        match std::fs::remove_dir_all(&p) {
            Ok(()) => log_line(app, "Widevine baixado no perfil removido (usando o embutido no Edge)"),
            Err(e) => log_line(app, &format!("não removi {}: {e}", p.display())),
        }
    }
    if !p.exists() {
        let _ = std::fs::create_dir_all(profile);
        if let Err(e) = std::fs::write(&p, b"") {
            log_line(app, &format!("não criei o bloqueio do Widevine: {e}"));
        }
    }
}

/// Abre o Edge do player se ainda não estiver aberto (precisa estar logado: a página pede o token).
pub fn ensure_edge(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut slot = state.edge.lock().unwrap();
    if slot.as_mut().is_some_and(|e| e.running()) {
        return Ok(());
    }
    let exe = edge_path().ok_or("Microsoft Edge não encontrado")?;
    let profile = config::local_dir().join("edge-profile");
    pin_bundled_widevine(app, &profile);
    let mut cmd = Command::new(exe);
    cmd.arg(format!("--app=http://127.0.0.1:{PLAYER_PORT}/"))
        .arg(format!("--user-data-dir={}", profile.display()))
        .args(EDGE_FLAGS);
    // Diagnóstico: SPOTRON_EDGE_DEBUG=1 liga o DevTools em 9223 (spikes/fase0/sdk/cdp_watch.py 9223).
    if std::env::var_os("SPOTRON_EDGE_DEBUG").is_some() {
        cmd.arg("--remote-debugging-port=9223");
    }
    *slot = Some(Edge::spawn(&mut cmd).map_err(|e| format!("não abri o Edge: {e}"))?);
    log_line(app, "Edge do player aberto");
    Ok(())
}

pub fn stop_edge(app: &AppHandle) {
    app.state::<AppState>().edge.lock().unwrap().take();
}

pub fn log_line(app: &AppHandle, msg: &str) {
    let dir = config::local_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("player.log")) {
        let _ = writeln!(f, "[{}] {msg}", config::now_secs());
    }
    let _ = app.emit("player-log", msg);
}

fn respond(req: tiny_http::Request, code: u16, body: impl Into<Vec<u8>>, ctype: &str) {
    let resp = tiny_http::Response::from_data(body.into())
        .with_status_code(code)
        .with_header(tiny_http::Header::from_bytes("Content-Type", ctype).unwrap())
        .with_header(tiny_http::Header::from_bytes("Cache-Control", "no-store").unwrap());
    let _ = req.respond(resp);
}

/// Sobe o servidor local do player numa thread própria.
pub fn start_server(app: AppHandle) -> Result<(), String> {
    let server = tiny_http::Server::http(("127.0.0.1", PLAYER_PORT))
        .map_err(|e| format!("porta {PLAYER_PORT} ocupada: outro Spotron aberto? ({e})"))?;
    let server = Arc::new(server);
    std::thread::spawn(move || {
        for mut req in server.incoming_requests() {
            let path = req.url().split('?').next().unwrap_or("").to_owned();
            match (req.method().as_str(), path.as_str()) {
                ("GET", "/") => respond(req, 200, PAGE, "text/html; charset=utf-8"),
                ("GET", "/token") => {
                    let state = app.state::<AppState>();
                    let cid = state.client_id();
                    match cid.ok_or_else(|| "sem Client ID".to_string()).and_then(|c| state.spotify.access_token(&c, 300)) {
                        Ok(t) => respond(req, 200, t, "text/plain"),
                        Err(e) => respond(req, 503, e, "text/plain; charset=utf-8"),
                    }
                }
                ("POST", "/state") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    if let Ok(v) = serde_json::from_str::<Value>(&body) {
                        let state = app.state::<AppState>();
                        if let Some(id) = v["device_id"].as_str() {
                            *state.device_id.lock().unwrap() = Some(id.to_owned());
                        }
                        if v["ready"] == Value::Bool(false) {
                            *state.device_id.lock().unwrap() = None;
                        }
                        let _ = app.emit("player-state", v);
                    }
                    respond(req, 204, Vec::new(), "text/plain");
                }
                ("POST", "/log") => {
                    let mut body = String::new();
                    let _ = req.as_reader().read_to_string(&mut body);
                    log_line(&app, &body);
                    respond(req, 204, Vec::new(), "text/plain");
                }
                _ => respond(req, 404, Vec::new(), "text/plain"),
            }
        }
    });
    Ok(())
}
