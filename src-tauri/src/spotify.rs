//! OAuth (Authorization Code + PKCE) e cliente da Web API, com renovação automática do token.
//! Validado no Spike C (spikes/fase0/api/api.py). Endpoints conferidos contra o changelog de
//! fev/2026 da Web API (modo de desenvolvimento): todos os de /me/player continuam disponíveis.
//! Tudo aqui é bloqueante: chamar de threads próprias ou via spawn_blocking.

use crate::config::{self, Tokens};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use reqwest::blocking::Client;
use reqwest::Method;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Mutex;
use std::time::Duration;

pub const REDIRECT_PORT: u16 = 8888;
pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const API: &str = "https://api.spotify.com/v1";
const SCOPES: &str = "streaming user-read-email user-read-private user-read-playback-state \
    user-modify-playback-state user-read-currently-playing playlist-read-private \
    playlist-read-collaborative user-library-read";

pub struct Spotify {
    http: Client,
    tokens: Mutex<Option<Tokens>>,
}

fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

impl Spotify {
    pub fn new() -> Self {
        Self {
            http: Client::builder().timeout(Duration::from_secs(20)).build().expect("cliente HTTP"),
            tokens: Mutex::new(config::load_tokens()),
        }
    }

    pub fn logged_in(&self) -> bool {
        self.tokens.lock().unwrap().is_some()
    }

    pub fn logout(&self) {
        *self.tokens.lock().unwrap() = None;
        config::delete_tokens();
    }

    fn post_token(&self, form: &[(&str, &str)]) -> Result<Value, String> {
        let r = self.http.post(TOKEN_URL).form(form).send().map_err(|e| format!("token: {e}"))?;
        let status = r.status();
        let body: Value = r.json().map_err(|e| format!("token: {e}"))?;
        if !status.is_success() {
            return Err(format!("token: HTTP {status} {}", body["error_description"].as_str().unwrap_or("")));
        }
        Ok(body)
    }

    fn store(&self, resp: &Value, old_refresh: Option<String>) -> Result<(), String> {
        let t = Tokens {
            access_token: resp["access_token"].as_str().ok_or("token sem access_token")?.to_owned(),
            // O Spotify pode não devolver um refresh token novo na renovação: manter o antigo.
            refresh_token: resp["refresh_token"].as_str().map(str::to_owned).or(old_refresh).ok_or("token sem refresh_token")?,
            expires_at: config::now_secs() + resp["expires_in"].as_u64().unwrap_or(3600),
            scope: resp["scope"].as_str().map(str::to_owned),
        };
        config::save_tokens(&t)?;
        *self.tokens.lock().unwrap() = Some(t);
        Ok(())
    }

    /// Login PKCE: abre o navegador padrão e espera o redirect em 127.0.0.1:8888 (até 5 min).
    pub fn login(&self, client_id: &str, open_url: impl Fn(&str) -> Result<(), String>) -> Result<(), String> {
        let verifier = random_b64(64);
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let state = random_b64(16);
        let url = url::Url::parse_with_params(
            AUTH_URL,
            &[
                ("client_id", client_id),
                ("response_type", "code"),
                ("redirect_uri", REDIRECT_URI),
                ("code_challenge_method", "S256"),
                ("code_challenge", &challenge),
                ("state", &state),
                ("scope", SCOPES),
            ],
        )
        .map_err(|e| e.to_string())?;

        let server = tiny_http::Server::http(("127.0.0.1", REDIRECT_PORT))
            .map_err(|e| format!("porta {REDIRECT_PORT} ocupada ({e}); feche outro login em andamento"))?;
        open_url(url.as_str())?;

        let deadline = std::time::Instant::now() + Duration::from_secs(300);
        let code = loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err("login expirou (5 min sem resposta do navegador)".into());
            }
            let Some(req) = server.recv_timeout(left).map_err(|e| e.to_string())? else { continue };
            let full = url::Url::parse(&format!("http://127.0.0.1{}", req.url())).map_err(|e| e.to_string())?;
            if full.path() != "/callback" {
                let _ = req.respond(tiny_http::Response::empty(404));
                continue;
            }
            let q: std::collections::HashMap<_, _> = full.query_pairs().into_owned().collect();
            let ok = q.get("state") == Some(&state) && q.contains_key("code");
            let msg = if ok { "Login concluído. Pode fechar esta aba e voltar ao Spotron." } else { "Login falhou. Volte ao Spotron e tente de novo." };
            let html = format!("<!doctype html><meta charset=utf-8><p style=\"font:16px system-ui\">{msg}</p>");
            let header = tiny_http::Header::from_bytes("Content-Type", "text/html; charset=utf-8").unwrap();
            let _ = req.respond(tiny_http::Response::from_string(html).with_header(header));
            if ok {
                break q["code"].clone();
            }
            return Err(format!("login recusado: {}", q.get("error").map(String::as_str).unwrap_or("state inválido")));
        };

        let resp = self.post_token(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", REDIRECT_URI),
            ("client_id", client_id),
            ("code_verifier", &verifier),
        ])?;
        self.store(&resp, None)
    }

    /// Access token válido por pelo menos `min_left` segundos (renova se preciso).
    pub fn access_token(&self, client_id: &str, min_left: u64) -> Result<String, String> {
        let current = self.tokens.lock().unwrap().clone().ok_or("não logado")?;
        if current.expires_at > config::now_secs() + min_left {
            return Ok(current.access_token);
        }
        let resp = self.post_token(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", &current.refresh_token),
            ("client_id", client_id),
        ])?;
        self.store(&resp, Some(current.refresh_token))?;
        Ok(self.tokens.lock().unwrap().as_ref().unwrap().access_token.clone())
    }

    /// Chamada à Web API. Devolve (status, corpo JSON ou Null).
    pub fn api(
        &self,
        client_id: &str,
        method: Method,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<(u16, Value), String> {
        let token = self.access_token(client_id, 120)?;
        let mut req = self.http.request(method, format!("{API}{path}")).bearer_auth(token).query(query);
        req = match body {
            Some(b) => req.json(&b),
            // Endpoints PUT/POST sem corpo exigem Content-Length: 0.
            None => req.header("Content-Length", "0"),
        };
        let r = req.send().map_err(|e| format!("{path}: {e}"))?;
        let status = r.status().as_u16();
        let text = r.text().unwrap_or_default();
        Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
    }
}

/// Converte links do open.spotify.com e URIs em URI do Spotify.
/// Ex.: https://open.spotify.com/intl-pt/track/ID?si=... -> spotify:track:ID
pub fn parse_uri(input: &str) -> Option<String> {
    let s = input.trim();
    if let Some(rest) = s.strip_prefix("spotify:") {
        let mut parts = rest.split(':');
        let (kind, id) = (parts.next()?, parts.next()?);
        return valid(kind, id);
    }
    let url = url::Url::parse(s).ok()?;
    if url.host_str()? != "open.spotify.com" {
        return None;
    }
    let segs: Vec<_> = url.path_segments()?.filter(|p| !p.is_empty() && !p.starts_with("intl-")).collect();
    match segs.as_slice() {
        [kind, id, ..] => valid(kind, id),
        _ => None,
    }
}

fn valid(kind: &str, id: &str) -> Option<String> {
    let kinds = ["track", "album", "playlist", "artist", "episode", "show"];
    (kinds.contains(&kind) && !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| format!("spotify:{kind}:{id}"))
}

#[cfg(test)]
mod tests {
    use super::parse_uri;

    #[test]
    fn links_e_uris() {
        assert_eq!(parse_uri("spotify:track:1HPQtDNiXB8gSFWX0nG3wA").as_deref(), Some("spotify:track:1HPQtDNiXB8gSFWX0nG3wA"));
        assert_eq!(
            parse_uri("https://open.spotify.com/intl-pt/track/1HPQtDNiXB8gSFWX0nG3wA?si=abc").as_deref(),
            Some("spotify:track:1HPQtDNiXB8gSFWX0nG3wA")
        );
        assert_eq!(parse_uri("https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M").as_deref(), Some("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"));
        assert_eq!(parse_uri("https://example.com/track/abc"), None);
        assert_eq!(parse_uri("spotify:user:abc"), None);
        assert_eq!(parse_uri("bobagem"), None);
    }
}
