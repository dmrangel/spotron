//! Comandos Tauri expostos à UI. A reprodução é controlada pela Web API, sempre apontando para o
//! dispositivo do player (device_id informado pela página do SDK).

use crate::{config, player, spotify, AppState};
use reqwest::Method;
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager, State};

/// Roda trabalho bloqueante (HTTP) fora da thread do async runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct Status {
    client_id: Option<String>,
    logged_in: bool,
    device_ready: bool,
    redirect_uri: &'static str,
}

#[tauri::command]
pub fn app_status(state: State<'_, AppState>) -> Status {
    Status {
        client_id: state.client_id(),
        logged_in: state.spotify.logged_in(),
        device_ready: state.device_id.lock().unwrap().is_some(),
        redirect_uri: spotify::REDIRECT_URI,
    }
}

#[tauri::command]
pub fn set_client_id(state: State<'_, AppState>, client_id: String) -> Result<(), String> {
    let id = client_id.trim().to_owned();
    if id.len() != 32 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Client ID inválido (são 32 caracteres hexadecimais)".into());
    }
    let mut cfg = state.config.lock().unwrap();
    cfg.client_id = Some(id);
    config::save_config(&cfg)
}

#[tauri::command]
pub async fn login(app: AppHandle) -> Result<(), String> {
    let cid = app.state::<AppState>().client_id().ok_or("configure o Client ID primeiro")?;
    let app2 = app.clone();
    blocking(move || {
        let state = app2.state::<AppState>();
        state.spotify.login(&cid, |url| {
            tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| format!("não abri o navegador: {e}"))
        })
    })
    .await?;
    player::ensure_edge(&app)
}

#[tauri::command]
pub fn logout(app: AppHandle) {
    player::stop_edge(&app);
    let state = app.state::<AppState>();
    state.spotify.logout();
    *state.device_id.lock().unwrap() = None;
}

/// Chama a Web API no dispositivo do player e transforma erros HTTP em mensagens.
async fn player_call(app: AppHandle, method: Method, path: &'static str, mut query: Vec<(&'static str, String)>, body: Option<Value>) -> Result<(), String> {
    blocking(move || {
        let state = app.state::<AppState>();
        let cid = state.client_id().ok_or("sem Client ID")?;
        let device = state.device_id.lock().unwrap().clone().ok_or("o player ainda está conectando; tente em alguns segundos")?;
        query.push(("device_id", device));
        let (status, resp) = state.spotify.api(&cid, method, path, &query, body)?;
        if (200..300).contains(&status) {
            Ok(())
        } else {
            let msg = resp["error"]["message"].as_str().unwrap_or("");
            Err(format!("Spotify respondeu HTTP {status} {msg}"))
        }
    })
    .await
}

#[tauri::command]
pub async fn play_uri(app: AppHandle, input: String) -> Result<String, String> {
    let uri = spotify::parse_uri(&input).ok_or("não reconheci o link/URI do Spotify")?;
    let body = if uri.starts_with("spotify:track:") || uri.starts_with("spotify:episode:") {
        json!({ "uris": [uri] })
    } else {
        json!({ "context_uri": uri })
    };
    player_call(app, Method::PUT, "/me/player/play", vec![], Some(body)).await?;
    Ok(uri)
}

#[tauri::command]
pub async fn transfer_here(app: AppHandle) -> Result<(), String> {
    blocking(move || {
        let state = app.state::<AppState>();
        let cid = state.client_id().ok_or("sem Client ID")?;
        let device = state.device_id.lock().unwrap().clone().ok_or("o player ainda está conectando")?;
        let (status, resp) = state.spotify.api(&cid, Method::PUT, "/me/player", &[], Some(json!({ "device_ids": [device], "play": true })))?;
        if (200..300).contains(&status) { Ok(()) } else { Err(format!("HTTP {status} {}", resp["error"]["message"].as_str().unwrap_or(""))) }
    })
    .await
}

#[tauri::command]
pub async fn pause(app: AppHandle) -> Result<(), String> {
    player_call(app, Method::PUT, "/me/player/pause", vec![], None).await
}

#[tauri::command]
pub async fn resume(app: AppHandle) -> Result<(), String> {
    player_call(app, Method::PUT, "/me/player/play", vec![], None).await
}

#[tauri::command]
pub async fn next(app: AppHandle) -> Result<(), String> {
    player_call(app, Method::POST, "/me/player/next", vec![], None).await
}

#[tauri::command]
pub async fn prev(app: AppHandle) -> Result<(), String> {
    player_call(app, Method::POST, "/me/player/previous", vec![], None).await
}

#[tauri::command]
pub async fn seek(app: AppHandle, position_ms: u64) -> Result<(), String> {
    player_call(app, Method::PUT, "/me/player/seek", vec![("position_ms", position_ms.to_string())], None).await
}

#[tauri::command]
pub async fn set_volume(app: AppHandle, percent: u8) -> Result<(), String> {
    player_call(app, Method::PUT, "/me/player/volume", vec![("volume_percent", percent.min(100).to_string())], None).await
}
