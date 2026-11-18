//! Arquivos do app em %APPDATA%\Spotron (config e tokens) e %LOCALAPPDATA%\Spotron (perfil do Edge, logs).
//! Nada disso vai para o repositório.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn roaming_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Spotron")
}

pub fn local_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Spotron")
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Client ID do app de desenvolvedor do Spotify (público no fluxo PKCE).
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: u64,
    pub scope: Option<String>,
}

fn load<T: DeserializeOwned>(name: &str) -> Option<T> {
    let raw = std::fs::read_to_string(roaming_dir().join(name)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn save<T: Serialize>(name: &str, value: &T) -> Result<(), String> {
    let dir = roaming_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let raw = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(name), raw).map_err(|e| e.to_string())
}

pub fn load_config() -> Config {
    load("config.json").unwrap_or_default()
}

pub fn save_config(cfg: &Config) -> Result<(), String> {
    save("config.json", cfg)
}

pub fn load_tokens() -> Option<Tokens> {
    load("tokens.json")
}

pub fn save_tokens(t: &Tokens) -> Result<(), String> {
    save("tokens.json", t)
}

pub fn delete_tokens() {
    let _ = std::fs::remove_file(roaming_dir().join("tokens.json"));
}
