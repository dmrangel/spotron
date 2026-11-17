//! Fase 0 / Spike A: Spotify Web Playback SDK dentro do WebView2 (Tauri 2).
//!
//! O backend só faz duas coisas:
//! - `get_token`: lê o access token salvo pelo Spike C (`api.py login` / `api.py token`)
//!   em %APPDATA%\spotron-spike\tokens.json. Nada de token no repositório.
//! - `log`: imprime no terminal e acrescenta em `spike_a.log` (pasta atual) o que a página registra.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn tokens_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    base.join("spotron-spike").join("tokens.json")
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tauri::command]
fn get_token() -> Result<String, String> {
    let path = tokens_path();
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("não li {}: {e}. Rode `python api.py login` (Spike C).", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let expires_at = json["expires_at"].as_u64().unwrap_or(0);
    if expires_at <= now_secs() + 60 {
        return Err("access token expirado: rode `python api.py token` e tente de novo".into());
    }
    json["access_token"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "tokens.json sem access_token".into())
}

#[tauri::command]
fn log(msg: String) {
    let line = format!("[{}] {msg}", now_secs());
    println!("{line}");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("spike_a.log") {
        let _ = writeln!(f, "{line}");
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_token, log])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o app Tauri");
}
