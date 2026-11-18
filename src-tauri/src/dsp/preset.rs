//! Formato próprio de preset (JSON, portátil para o futuro app Android) e conversão para a config
//! do CamillaDSP 4.1.3. Nomes de filtros conferidos em src/config/mod.rs da tag v4.1.3:
//! Biquad Peaking/Lowshelf/Highshelf {freq, gain, q}; Lowpass/Highpass/Notch/Bandpass/Allpass {freq, q};
//! Gain {gain, scale}; Limiter {clip_limit, soft_clip}.
//! A config é gerada como JSON (YAML aceita JSON) e enviada com o comando WebSocket SetConfigJson.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterType {
    Peaking,
    Lowshelf,
    Highshelf,
    Lowpass,
    Highpass,
    Notch,
    Bandpass,
    Allpass,
}

impl FilterType {
    pub fn has_gain(self) -> bool {
        matches!(self, FilterType::Peaking | FilterType::Lowshelf | FilterType::Highshelf)
    }
    fn camilla(self) -> &'static str {
        match self {
            FilterType::Peaking => "Peaking",
            FilterType::Lowshelf => "Lowshelf",
            FilterType::Highshelf => "Highshelf",
            FilterType::Lowpass => "Lowpass",
            FilterType::Highpass => "Highpass",
            FilterType::Notch => "Notch",
            FilterType::Bandpass => "Bandpass",
            FilterType::Allpass => "Allpass",
        }
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Band {
    #[serde(rename = "type")]
    pub kind: FilterType,
    pub freq: f64,
    #[serde(default)]
    pub gain_db: f64,
    pub q: f64,
    #[serde(default = "yes")]
    pub enabled: bool,
}

/// Correção de resposta (AutoEQ): fica separada do EQ do usuário e as duas valem ao mesmo tempo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub name: String,
    pub preamp_db: f64,
    pub filters: Vec<Band>,
    #[serde(default = "yes")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limiter {
    #[serde(default = "yes")]
    pub enabled: bool,
    pub clip_limit_db: f64,
}

impl Default for Limiter {
    fn default() -> Self {
        Self { enabled: true, clip_limit_db: -0.1 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    #[serde(default)]
    pub preamp_db: f64,
    #[serde(default)]
    pub filters: Vec<Band>,
    #[serde(default)]
    pub correction: Option<Correction>,
    #[serde(default)]
    pub limiter: Limiter,
    /// Efeitos ainda não implementados (crossfeed, ambiophonics...) ficam guardados sem uso.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Preset {
    fn default() -> Self {
        Self { name: "Plano".into(), preamp_db: 0.0, filters: vec![], correction: None, limiter: Limiter::default(), extra: Map::new() }
    }
}

/// Parâmetros de dispositivo (validados no Spike B).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Devices {
    pub samplerate: u32,
    pub chunksize: u32,
    pub target_level: u32,
    pub capture_device: String,
    /// None = saída padrão do Windows (acompanha a troca de saída sozinho).
    pub playback_device: Option<String>,
}

impl Default for Devices {
    fn default() -> Self {
        Self {
            samplerate: 48000,
            chunksize: 1024,
            target_level: 1024,
            capture_device: "CABLE Output (VB-Audio Virtual Cable)".into(),
            playback_device: None,
        }
    }
}

fn biquad(b: &Band) -> Value {
    let mut p = json!({ "type": b.kind.camilla(), "freq": b.freq, "q": b.q });
    if b.kind.has_gain() {
        p["gain"] = json!(b.gain_db);
    }
    json!({ "type": "Biquad", "parameters": p })
}

/// Gera a config completa do CamillaDSP. `enabled == false` = bypass (só o limiter fica, se ligado).
pub fn to_camilla(preset: &Preset, devices: &Devices, enabled: bool) -> Value {
    let mut filters = Map::new();
    let mut names: Vec<String> = vec![];

    if enabled {
        let corr = preset.correction.as_ref().filter(|c| c.enabled);
        let preamp = preset.preamp_db + corr.map_or(0.0, |c| c.preamp_db);
        if preamp != 0.0 {
            filters.insert("preamp".into(), json!({ "type": "Gain", "parameters": { "gain": preamp, "scale": "dB" } }));
            names.push("preamp".into());
        }
        if let Some(c) = corr {
            for (i, b) in c.filters.iter().enumerate().filter(|(_, b)| b.enabled) {
                let n = format!("corr_{i}");
                filters.insert(n.clone(), biquad(b));
                names.push(n);
            }
        }
        for (i, b) in preset.filters.iter().enumerate().filter(|(_, b)| b.enabled) {
            let n = format!("eq_{i}");
            filters.insert(n.clone(), biquad(b));
            names.push(n);
        }
    }
    if preset.limiter.enabled {
        filters.insert(
            "limiter".into(),
            json!({ "type": "Limiter", "parameters": { "clip_limit": preset.limiter.clip_limit_db, "soft_clip": false } }),
        );
        names.push("limiter".into());
    }
    if names.is_empty() {
        // Pipeline precisa de pelo menos um filtro: ganho de 0 dB é transparente.
        filters.insert("passthrough".into(), json!({ "type": "Gain", "parameters": { "gain": 0.0, "scale": "dB" } }));
        names.push("passthrough".into());
    }

    json!({
        "title": format!("Spotron - {}", preset.name),
        "devices": {
            "samplerate": devices.samplerate,
            "capture_samplerate": devices.samplerate,
            "chunksize": devices.chunksize,
            "queuelimit": 4,
            "enable_rate_adjust": true,
            "target_level": devices.target_level,
            "adjust_period": 10,
            "resampler": { "type": "AsyncSinc", "profile": "Balanced" },
            "silence_threshold": -100,
            "silence_timeout": 3.0,
            "capture": { "type": "Wasapi", "channels": 2, "device": devices.capture_device, "format": "F32", "exclusive": false, "loopback": false },
            "playback": { "type": "Wasapi", "channels": 2, "device": devices.playback_device, "format": "F32", "exclusive": false },
        },
        "filters": filters,
        "pipeline": [{ "type": "Filter", "description": "Spotron", "channels": [0, 1], "names": names }],
    })
}

/// Importa o formato paramétrico do AutoEQ ("ParametricEQ.txt"):
///   Preamp: -6.2 dB
///   Filter 1: ON LSC Fc 105 Hz Gain 5.5 dB Q 0.71
///   Filter 2: ON PK Fc 2000 Hz Gain -3.1 dB Q 1.41
pub fn parse_autoeq(name: &str, text: &str) -> Result<Correction, String> {
    let mut preamp = 0.0;
    let mut filters = vec![];
    for (ln, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks[0].eq_ignore_ascii_case("Preamp:") {
            preamp = toks.get(1).and_then(|v| v.parse().ok()).ok_or(format!("linha {}: preamp inválido", ln + 1))?;
            continue;
        }
        if !toks[0].eq_ignore_ascii_case("Filter") {
            continue;
        }
        // Filter N: ON TYPE Fc X Hz [Gain Y dB] [Q Z]
        let enabled = toks.get(2).is_some_and(|s| s.eq_ignore_ascii_case("ON"));
        let kind = match toks.get(3).map(|s| s.to_ascii_uppercase()).as_deref() {
            Some("PK") | Some("PEQ") => FilterType::Peaking,
            Some("LSC") | Some("LS") | Some("LSQ") => FilterType::Lowshelf,
            Some("HSC") | Some("HS") | Some("HSQ") => FilterType::Highshelf,
            Some("LP") | Some("LPQ") => FilterType::Lowpass,
            Some("HP") | Some("HPQ") => FilterType::Highpass,
            Some("NO") => FilterType::Notch,
            Some("BP") => FilterType::Bandpass,
            Some("AP") => FilterType::Allpass,
            Some(other) => return Err(format!("linha {}: tipo de filtro não suportado: {other}", ln + 1)),
            None => return Err(format!("linha {}: filtro incompleto", ln + 1)),
        };
        let value_after = |key: &str| -> Option<f64> {
            toks.iter().position(|t| t.eq_ignore_ascii_case(key)).and_then(|i| toks.get(i + 1)).and_then(|v| v.parse().ok())
        };
        let freq = value_after("Fc").ok_or(format!("linha {}: sem Fc", ln + 1))?;
        let gain_db = value_after("Gain").unwrap_or(0.0);
        // Shelves do AutoEQ usam Q 0.71 quando não informado.
        let q = value_after("Q").unwrap_or(0.71);
        filters.push(Band { kind, freq, gain_db, q, enabled });
    }
    if filters.is_empty() {
        return Err("nenhum filtro encontrado (esperado o formato ParametricEQ.txt do AutoEQ)".into());
    }
    Ok(Correction { name: name.to_owned(), preamp_db: preamp, filters, enabled: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AUTOEQ: &str = "Preamp: -6.2 dB\nFilter 1: ON LSC Fc 105 Hz Gain 5.5 dB Q 0.71\nFilter 2: ON PK Fc 2000 Hz Gain -3.1 dB Q 1.41\nFilter 3: OFF HSC Fc 10000 Hz Gain 2 dB Q 0.7\n";

    #[test]
    fn autoeq() {
        let c = parse_autoeq("HD650", AUTOEQ).unwrap();
        assert_eq!(c.preamp_db, -6.2);
        assert_eq!(c.filters.len(), 3);
        assert_eq!(c.filters[0].kind, FilterType::Lowshelf);
        assert_eq!(c.filters[1], Band { kind: FilterType::Peaking, freq: 2000.0, gain_db: -3.1, q: 1.41, enabled: true });
        assert!(!c.filters[2].enabled);
        assert!(parse_autoeq("x", "bobagem").is_err());
    }

    #[test]
    fn conversao() {
        let mut p = Preset { preamp_db: -2.0, ..Preset::default() };
        p.filters.push(Band { kind: FilterType::Peaking, freq: 1000.0, gain_db: 3.0, q: 1.0, enabled: true });
        p.filters.push(Band { kind: FilterType::Lowpass, freq: 18000.0, gain_db: 9.0, q: 0.7, enabled: true });
        p.filters.push(Band { kind: FilterType::Peaking, freq: 50.0, gain_db: 9.0, q: 0.7, enabled: false });
        p.correction = Some(parse_autoeq("HD650", AUTOEQ).unwrap());
        let cfg = to_camilla(&p, &Devices::default(), true);
        let names: Vec<_> = cfg["pipeline"][0]["names"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
        assert_eq!(names, ["preamp", "corr_0", "corr_1", "eq_0", "eq_1", "limiter"]);
        assert!((cfg["filters"]["preamp"]["parameters"]["gain"].as_f64().unwrap() + 8.2).abs() < 1e-9);
        assert!(cfg["filters"]["eq_1"]["parameters"].get("gain").is_none(), "Lowpass não leva gain");
        // Bypass: só o limiter.
        let off = to_camilla(&p, &Devices::default(), false);
        assert_eq!(off["pipeline"][0]["names"], json!(["limiter"]));
    }

    /// Valida as configs geradas com o próprio camilladsp.exe (`-c`), se o binário estiver presente.
    #[test]
    fn camilladsp_aceita() {
        let bin = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries/camilladsp-x86_64-pc-windows-msvc.exe");
        if !bin.exists() {
            eprintln!("camilladsp.exe ausente: teste pulado");
            return;
        }
        let mut p = Preset::default();
        for (kind, gain) in [
            (FilterType::Peaking, 3.0), (FilterType::Lowshelf, 2.0), (FilterType::Highshelf, -2.0), (FilterType::Lowpass, 0.0),
            (FilterType::Highpass, 0.0), (FilterType::Notch, 0.0), (FilterType::Bandpass, 0.0), (FilterType::Allpass, 0.0),
        ] {
            p.filters.push(Band { kind, freq: 1000.0, gain_db: gain, q: 0.7, enabled: true });
        }
        p.correction = Some(parse_autoeq("t", AUTOEQ).unwrap());
        let mut nada = Preset::default();
        nada.limiter.enabled = false;
        for (i, cfg) in [to_camilla(&p, &Devices::default(), true), to_camilla(&nada, &Devices::default(), true)].iter().enumerate() {
            let f = std::env::temp_dir().join(format!("spotron_test_{i}.yml"));
            std::fs::write(&f, cfg.to_string()).unwrap();
            let out = std::process::Command::new(&bin).arg("-c").arg(&f).output().unwrap();
            let text = String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
            assert!(text.contains("Config is valid"), "config {i} recusada: {text}");
        }
    }
}
