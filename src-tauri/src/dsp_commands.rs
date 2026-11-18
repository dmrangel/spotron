//! Comandos Tauri do DSP: preset atual, liga/desliga, biblioteca de presets, importação AutoEQ e estado.

use crate::dsp::preset::{self, Preset};
use crate::dsp::{self, DspSettings, DspStatus};
use crate::AppState;
use tauri::{AppHandle, Manager, State};

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn dsp_get(state: State<'_, AppState>) -> DspSettings {
    state.dsp.settings.lock().unwrap().clone()
}

/// Troca o preset atual e aplica ao vivo.
#[tauri::command]
pub async fn dsp_set_preset(app: AppHandle, preset: Preset) -> Result<(), String> {
    blocking(move || {
        let dsp = &app.state::<AppState>().dsp;
        dsp.settings.lock().unwrap().preset = preset;
        dsp.apply()
    })
    .await
}

#[tauri::command]
pub async fn dsp_set_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    blocking(move || {
        let dsp = &app.state::<AppState>().dsp;
        dsp.settings.lock().unwrap().enabled = enabled;
        dsp.apply()
    })
    .await
}

#[tauri::command]
pub async fn dsp_status(app: AppHandle) -> Result<DspStatus, String> {
    blocking(move || Ok(app.state::<AppState>().dsp.status())).await
}

#[derive(serde::Serialize)]
pub struct RaceInfo {
    delay_us: f64,
    delay_samples: usize,
    gain_db: f64,
    taps: usize,
}

/// O que o R.A.C.E. realmente aplica: atraso arredondado para amostras e compensação de ganho.
#[tauri::command]
pub fn race_info(state: State<'_, AppState>, params: crate::dsp::spatial::Ambiophonics) -> RaceInfo {
    let fs = state.dsp.settings.lock().unwrap().devices.samplerate as f64;
    let (same, _, d) = crate::dsp::spatial::race_impulse(&params, fs);
    let (_, delay_us, gain_db) = crate::dsp::spatial::ambiophonics(&params, fs);
    RaceInfo { delay_us, delay_samples: d, gain_db, taps: same.len() }
}

#[tauri::command]
pub fn autoeq_parse(name: String, text: String) -> Result<preset::Correction, String> {
    preset::parse_autoeq(name.trim(), &text)
}

fn file_name(name: &str) -> Result<String, String> {
    let clean: String = name.trim().chars().filter(|c| !r#"\/:*?"<>|"#.contains(*c)).collect();
    if clean.is_empty() {
        return Err("nome de preset vazio".into());
    }
    Ok(format!("{clean}.json"))
}

#[tauri::command]
pub fn preset_list() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dsp::presets_dir())
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json")).map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

#[tauri::command]
pub fn preset_save(mut preset: Preset, name: String) -> Result<(), String> {
    let dir = dsp::presets_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    preset.name = name.trim().to_owned();
    let raw = serde_json::to_string_pretty(&preset).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(file_name(&name)?), raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn preset_load(name: String) -> Result<Preset, String> {
    let raw = std::fs::read_to_string(dsp::presets_dir().join(file_name(&name)?)).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("preset inválido: {e}"))
}

#[tauri::command]
pub fn preset_delete(name: String) -> Result<(), String> {
    std::fs::remove_file(dsp::presets_dir().join(file_name(&name)?)).map_err(|e| e.to_string())
}

/// Abre Configurações > Som > Mixer de volume (para mandar o Edge do player para o "CABLE Input").
#[tauri::command]
pub fn open_volume_mixer() -> Result<(), String> {
    tauri_plugin_opener::open_url("ms-settings:apps-volume", None::<&str>).map_err(|e| e.to_string())
}
