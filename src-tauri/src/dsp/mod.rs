//! CamillaDSP 4.1.3 como sidecar: captura o "CABLE Output" (para onde o Edge do player toca),
//! aplica o preset e toca na saída padrão. Controle por WebSocket (SetConfigJson, Get*).
//!
//! - O binário vem de src-tauri/binaries/camilladsp-x86_64-pc-windows-msvc.exe (externalBin do
//!   Tauri, copiado para o lado do spotron.exe no build). SPOTRON_CAMILLADSP sobrescreve o caminho.
//! - Preso a um Job Object (procjob.rs): fechar o app não deixa camilladsp.exe órfão.
//! - Supervisor: se o processo cair (ex.: dispositivo removido), reinicia com espera crescente.

pub mod preset;

use crate::config;
use crate::procjob::{self, Managed};
use preset::{Devices, Preset};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const WS_PORT: u16 = 12340;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DspSettings {
    pub enabled: bool,
    pub preset: Preset,
    #[serde(default)]
    pub devices: Devices,
}

impl Default for DspSettings {
    fn default() -> Self {
        Self { enabled: true, preset: Preset::default(), devices: Devices::default() }
    }
}

pub struct Dsp {
    pub settings: Mutex<DspSettings>,
    proc: Mutex<Option<Managed>>,
    shutting_down: AtomicBool,
    pub last_error: Mutex<Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DspStatus {
    pub running: bool,
    pub state: Option<String>,
    pub capture_rms: Option<Vec<f64>>,
    pub playback_rms: Option<Vec<f64>>,
    pub clipped_samples: Option<u64>,
    pub load: Option<f64>,
    pub error: Option<String>,
}

fn settings_path() -> PathBuf {
    config::roaming_dir().join("dsp.json")
}

pub fn presets_dir() -> PathBuf {
    config::roaming_dir().join("presets")
}

fn config_path() -> PathBuf {
    config::local_dir().join("camilladsp.yml")
}

fn log_path() -> PathBuf {
    config::local_dir().join("camilladsp.log")
}

fn binary() -> Result<PathBuf, String> {
    if let Some(p) = std::env::var_os("SPOTRON_CAMILLADSP") {
        return Ok(PathBuf::from(p));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let p = exe.parent().ok_or("sem pasta do executável")?.join("camilladsp.exe");
    if p.exists() { Ok(p) } else { Err(format!("camilladsp.exe não encontrado em {}", p.display())) }
}

impl Dsp {
    pub fn new() -> Self {
        let settings = std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { settings: Mutex::new(settings), proc: Mutex::new(None), shutting_down: AtomicBool::new(false), last_error: Mutex::new(None) }
    }

    fn camilla_config(&self) -> Value {
        let s = self.settings.lock().unwrap();
        preset::to_camilla(&s.preset, &s.devices, s.enabled)
    }

    fn save_settings(&self) -> Result<(), String> {
        let s = self.settings.lock().unwrap().clone();
        std::fs::create_dir_all(config::roaming_dir()).map_err(|e| e.to_string())?;
        std::fs::write(settings_path(), serde_json::to_string_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    fn write_config_file(&self) -> Result<(), String> {
        std::fs::create_dir_all(config::local_dir()).map_err(|e| e.to_string())?;
        // JSON é YAML válido: o CamillaDSP lê direto.
        std::fs::write(config_path(), serde_json::to_string_pretty(&self.camilla_config()).unwrap()).map_err(|e| e.to_string())
    }

    fn spawn(&self) -> Result<(), String> {
        self.write_config_file()?;
        let mut cmd = Command::new(binary()?);
        cmd.arg("-p").arg(WS_PORT.to_string()).arg("-o").arg(log_path()).arg(config_path());
        procjob::hide_console(&mut cmd);
        *self.proc.lock().unwrap() = Some(Managed::spawn(&mut cmd).map_err(|e| format!("não iniciei o CamillaDSP: {e}"))?);
        Ok(())
    }

    /// Envia comandos e devolve as respostas (uma conexão, vários comandos).
    pub fn ws(&self, cmds: &[Value]) -> Result<Vec<Value>, String> {
        let (mut sock, _) = tungstenite::connect(format!("ws://127.0.0.1:{WS_PORT}")).map_err(|e| format!("WebSocket do CamillaDSP: {e}"))?;
        let mut out = vec![];
        for c in cmds {
            sock.send(tungstenite::Message::Text(c.to_string().into())).map_err(|e| e.to_string())?;
            loop {
                match sock.read().map_err(|e| e.to_string())? {
                    tungstenite::Message::Text(t) => {
                        out.push(serde_json::from_str(t.as_str()).unwrap_or(Value::Null));
                        break;
                    }
                    tungstenite::Message::Close(_) => return Err("WebSocket fechado".into()),
                    _ => continue,
                }
            }
        }
        let _ = sock.close(None);
        Ok(out)
    }

    /// Salva as configurações e aplica ao vivo (SetConfigJson). Se o CamillaDSP ainda está subindo,
    /// o arquivo de config já atualizado vale na próxima inicialização.
    pub fn apply(&self) -> Result<(), String> {
        self.save_settings()?;
        self.write_config_file()?;
        let cfg = self.camilla_config().to_string();
        let mut last = String::new();
        for _ in 0..3 {
            match self.ws(&[json!({ "SetConfigJson": cfg })]) {
                Ok(r) => {
                    let res = &r[0]["SetConfigJson"]["result"];
                    if res == "Ok" {
                        return Ok(());
                    }
                    last = format!("CamillaDSP recusou a config: {res}");
                }
                Err(e) => last = e,
            }
            std::thread::sleep(Duration::from_millis(300));
        }
        Err(last)
    }

    pub fn status(&self) -> DspStatus {
        let running = self.proc.lock().unwrap().as_mut().is_some_and(|p| p.running());
        let mut st = DspStatus { running, state: None, capture_rms: None, playback_rms: None, clipped_samples: None, load: None, error: self.last_error.lock().unwrap().clone() };
        if !running {
            return st;
        }
        let cmds = ["GetState", "GetCaptureSignalRms", "GetPlaybackSignalRms", "GetClippedSamples", "GetProcessingLoad"].map(|c| json!(c));
        if let Ok(r) = self.ws(&cmds) {
            let v = |i: usize, k: &str| r.get(i).map(|x| x[k]["value"].clone()).unwrap_or(Value::Null);
            st.state = v(0, "GetState").as_str().map(str::to_owned);
            st.capture_rms = serde_json::from_value(v(1, "GetCaptureSignalRms")).ok();
            st.playback_rms = serde_json::from_value(v(2, "GetPlaybackSignalRms")).ok();
            st.clipped_samples = v(3, "GetClippedSamples").as_u64();
            st.load = v(4, "GetProcessingLoad").as_f64();
        }
        st
    }

    pub fn shutdown(&self) {
        self.shutting_down.store(true, Ordering::SeqCst);
        self.proc.lock().unwrap().take();
    }
}

fn log_tail() -> String {
    let text = std::fs::read_to_string(log_path()).unwrap_or_default();
    text.lines().rev().find(|l| l.contains("ERROR") || l.contains("WARN")).unwrap_or("").chars().take(300).collect()
}

/// Inicia o CamillaDSP e um supervisor que o reinicia se cair.
pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let dsp = &app.state::<crate::AppState>().dsp;
        let mut backoff = Duration::from_secs(2);
        let mut started_at = Instant::now();
        loop {
            if dsp.shutting_down.load(Ordering::SeqCst) {
                return;
            }
            let running = dsp.proc.lock().unwrap().as_mut().is_some_and(|p| p.running());
            if !running {
                let had_proc = dsp.proc.lock().unwrap().is_some();
                if had_proc {
                    let msg = format!("CamillaDSP parou; reiniciando. {}", log_tail());
                    *dsp.last_error.lock().unwrap() = Some(msg.clone());
                    let _ = app.emit("dsp-log", msg);
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(Duration::from_secs(30));
                }
                match dsp.spawn() {
                    Ok(()) => {
                        started_at = Instant::now();
                        let _ = app.emit("dsp-log", "CamillaDSP iniciado");
                    }
                    Err(e) => {
                        *dsp.last_error.lock().unwrap() = Some(e.clone());
                        let _ = app.emit("dsp-log", e);
                        std::thread::sleep(backoff);
                        backoff = (backoff * 2).min(Duration::from_secs(30));
                    }
                }
            } else if started_at.elapsed() > Duration::from_secs(60) {
                backoff = Duration::from_secs(2);
                if dsp.last_error.lock().unwrap().take().is_some() {
                    let _ = app.emit("dsp-log", "CamillaDSP estável");
                }
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });
}
