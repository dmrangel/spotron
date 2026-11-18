//! Demais efeitos do Neutron (PLANO.md, "Efeitos de DSP"), para CamillaDSP 4.1.3:
//! - Canais: mono, troca L/R (Mixer), inversão de fase, balanço e atraso por canal (Gain/Delay).
//! - Proteção de caixas: corte subsônico/ultrassônico (BiquadCombo Butterworth).
//! - Loudness adaptativo: filtro Loudness ligado ao volume principal (fader Main).
//! O volume principal do CamillaDSP existe sempre (SetVolume / -g), sem filtro no pipeline.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Channels {
    /// −1 = só esquerda, 0 = centro, +1 = só direita.
    #[serde(default)]
    pub balance: f64,
    #[serde(default)]
    pub mono: bool,
    #[serde(default)]
    pub swap: bool,
    #[serde(default)]
    pub invert_left: bool,
    #[serde(default)]
    pub invert_right: bool,
    /// Alinhamento de tempo (caixas a distâncias diferentes).
    #[serde(default)]
    pub delay_left_ms: f64,
    #[serde(default)]
    pub delay_right_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Protection {
    pub enabled: bool,
    /// Corte subsônico (0 = desligado).
    pub subsonic_hz: f64,
    /// Corte ultrassônico (0 = desligado).
    pub ultrasonic_hz: f64,
}

impl Default for Protection {
    fn default() -> Self {
        Self { enabled: false, subsonic_hz: 20.0, ultrasonic_hz: 0.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loudness {
    pub enabled: bool,
    /// Volume (dB) a partir do qual não há compensação; abaixo dele graves/agudos sobem.
    pub reference_level_db: f64,
    pub low_boost_db: f64,
    pub high_boost_db: f64,
}

impl Default for Loudness {
    fn default() -> Self {
        Self { enabled: false, reference_level_db: -25.0, low_boost_db: 10.0, high_boost_db: 7.0 }
    }
}

/// Mixer de mono/troca (antes do EQ). None se não houver nada a fazer.
pub fn channel_mixer(c: &Channels) -> Option<Value> {
    let src = |ch: usize, g: f64| json!({ "channel": ch, "gain": g, "scale": "linear" });
    let mapping = if c.mono {
        let both = json!([src(0, 0.5), src(1, 0.5)]);
        json!([{ "dest": 0, "sources": both.clone() }, { "dest": 1, "sources": both }])
    } else if c.swap {
        json!([{ "dest": 0, "sources": [src(1, 1.0)] }, { "dest": 1, "sources": [src(0, 1.0)] }])
    } else {
        return None;
    };
    Some(json!({ "channels": { "in": 2, "out": 2 }, "mapping": mapping }))
}

/// Filtros por canal (fase, balanço, atraso): devolve (nomes do canal 0, nomes do canal 1).
pub fn per_channel(c: &Channels, filters: &mut Map<String, Value>) -> (Vec<String>, Vec<String>) {
    let mut out = (vec![], vec![]);
    let b = c.balance.clamp(-1.0, 1.0);
    // Balanço: atenua o lado oposto (máx. −60 dB).
    let att = |x: f64| if x <= 0.0 { -60.0 } else { (20.0 * x.log10()).max(-60.0) };
    let (gain_l, gain_r) = (if b > 0.0 { att(1.0 - b) } else { 0.0 }, if b < 0.0 { att(1.0 + b) } else { 0.0 });
    for (i, (invert, gain, delay)) in [(c.invert_left, gain_l, c.delay_left_ms), (c.invert_right, gain_r, c.delay_right_ms)].into_iter().enumerate() {
        let names = if i == 0 { &mut out.0 } else { &mut out.1 };
        if invert || gain != 0.0 {
            let n = format!("ch{i}_gain");
            filters.insert(n.clone(), json!({ "type": "Gain", "parameters": { "gain": gain, "scale": "dB", "inverted": invert } }));
            names.push(n);
        }
        if delay > 0.0 {
            let n = format!("ch{i}_delay");
            filters.insert(n.clone(), json!({ "type": "Delay", "parameters": { "delay": delay, "unit": "ms", "subsample": true } }));
            names.push(n);
        }
    }
    out
}

/// Proteção de caixas e loudness (nomes para o canal 0 e 1, na ordem).
pub fn protection_and_loudness(p: &Protection, l: &Loudness, filters: &mut Map<String, Value>) -> Vec<String> {
    let mut names = vec![];
    if p.enabled && p.subsonic_hz > 0.0 {
        filters.insert("prot_hp".into(), json!({ "type": "BiquadCombo", "parameters": { "type": "ButterworthHighpass", "freq": p.subsonic_hz, "order": 4 } }));
        names.push("prot_hp".into());
    }
    if p.enabled && p.ultrasonic_hz > 0.0 {
        filters.insert("prot_lp".into(), json!({ "type": "BiquadCombo", "parameters": { "type": "ButterworthLowpass", "freq": p.ultrasonic_hz, "order": 4 } }));
        names.push("prot_lp".into());
    }
    if l.enabled {
        filters.insert(
            "loudness".into(),
            json!({ "type": "Loudness", "parameters": {
                "reference_level": l.reference_level_db, "low_boost": l.low_boost_db, "high_boost": l.high_boost_db,
                "fader": "Main", "attenuate_mid": false } }),
        );
        names.push("loudness".into());
    }
    names
}

/// Volume do app (0–100 na UI) → dB do volume principal do CamillaDSP. Curva quadrática:
/// 50% ≈ −12 dB, 10% = −40 dB; 0 = mudo.
pub fn percent_to_db(p: f64) -> f64 {
    if p <= 0.0 {
        -100.0
    } else {
        (40.0 * (p.min(100.0) / 100.0).log10()).max(-100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume() {
        assert_eq!(percent_to_db(100.0), 0.0);
        assert!((percent_to_db(50.0) + 12.04).abs() < 0.01);
        assert_eq!(percent_to_db(0.0), -100.0);
    }

    #[test]
    fn balanco() {
        let mut f = Map::new();
        let (l, r) = per_channel(&Channels { balance: 0.5, ..Default::default() }, &mut f);
        assert_eq!((l.len(), r.len()), (1, 0), "balanço à direita atenua só a esquerda");
        assert!((f["ch0_gain"]["parameters"]["gain"].as_f64().unwrap() + 6.02).abs() < 0.01);
    }
}
