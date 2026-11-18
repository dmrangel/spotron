//! Efeitos espaciais: crossfeed (fones) e Ambiophonic R.A.C.E. (caixas). Um exclui o outro.
//!
//! Ambos usam o mesmo esqueleto no CamillaDSP 4.1.3 (sintaxe conferida em src/config/mod.rs):
//!   Mixer 2→4 [L, R, R, L] → filtros por canal → Mixer 4→2 (out0 = ch0 + ch2, out1 = ch1 + ch3).
//! Assim cada saída recebe o "próprio" canal (ch0/ch1) mais o caminho cruzado do oposto (ch2/ch3).
//!
//! R.A.C.E. (Glasgal & Miller, 2007): Lo = L − a·BP·z^−d·Ro e Ro = R − a·BP·z^−d·Lo, recursivo.
//! O pipeline do CamillaDSP não tem realimentação, então a recursão é simulada aqui com um impulso
//! (com o passa-faixa dentro do laço) e vira dois FIR: h_same (L→Lo) e h_cross (R→Lo).

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crossfeed {
    pub enabled: bool,
    /// Frequência de corte do caminho cruzado (bs2b: 300–2000 Hz).
    pub cutoff_hz: f64,
    /// Quanto o caminho cruzado fica abaixo do direto nos graves (bs2b: 4,5 / 6 / 9,5 dB).
    pub level_db: f64,
}

impl Default for Crossfeed {
    fn default() -> Self {
        Self { enabled: false, cutoff_hz: 700.0, level_db: 4.5 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ambiophonics {
    pub enabled: bool,
    /// Atraso por recursão (60–120 µs; ~70 µs típico). Arredondado para amostras.
    pub delay_us: f64,
    /// Atenuação por recursão (2–4 dB).
    pub attenuation_db: f64,
    /// Faixa processada; graves e agudos passam direto.
    pub low_hz: f64,
    pub high_hz: f64,
}

impl Default for Ambiophonics {
    fn default() -> Self {
        Self { enabled: false, delay_us: 70.0, attenuation_db: 3.0, low_hz: 250.0, high_hz: 5000.0 }
    }
}

fn db_to_lin(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Mixer 2→4 que duplica em [L, R, R, L].
fn split_mixer() -> Value {
    let src = |c: usize| json!([{ "channel": c, "gain": 0.0, "scale": "dB" }]);
    json!({
        "channels": { "in": 2, "out": 4 },
        "mapping": [
            { "dest": 0, "sources": src(0) },
            { "dest": 1, "sources": src(1) },
            { "dest": 2, "sources": src(1) },
            { "dest": 3, "sources": src(0) },
        ]
    })
}

/// Mixer 4→2: out0 = g_same·ch0 + g_cross·ch2, out1 = g_same·ch1 + g_cross·ch3 (ganhos lineares).
fn merge_mixer(g_same: f64, g_cross: f64) -> Value {
    let pair = |a: usize, b: usize| {
        json!([
            { "channel": a, "gain": g_same, "scale": "linear" },
            { "channel": b, "gain": g_cross, "scale": "linear" },
        ])
    };
    json!({
        "channels": { "in": 4, "out": 2 },
        "mapping": [ { "dest": 0, "sources": pair(0, 2) }, { "dest": 1, "sources": pair(1, 3) } ]
    })
}

/// Passos do pipeline e definições (filters/mixers) de um efeito espacial.
pub struct Spatial {
    pub filters: Map<String, Value>,
    pub mixers: Map<String, Value>,
    pub steps: Vec<Value>,
}

/// Crossfeed estilo bs2b: caminho cruzado = passa-baixa de 1ª ordem + atraso de 0,3 ms, com nível
/// `level_db` abaixo do direto nos graves. Ganhos normalizados para a soma em fase não passar de 0 dB.
pub fn crossfeed(cf: &Crossfeed) -> Spatial {
    let c = db_to_lin(-cf.level_db);
    let (g_same, g_cross) = (1.0 / (1.0 + c), c / (1.0 + c));
    let mut filters = Map::new();
    filters.insert("xf_lp".into(), json!({ "type": "Biquad", "parameters": { "type": "LowpassFO", "freq": cf.cutoff_hz } }));
    filters.insert("xf_delay".into(), json!({ "type": "Delay", "parameters": { "delay": 0.3, "unit": "ms", "subsample": true } }));
    let mut mixers = Map::new();
    mixers.insert("xf_split".into(), split_mixer());
    mixers.insert("xf_merge".into(), merge_mixer(g_same, g_cross));
    let steps = vec![
        json!({ "type": "Mixer", "name": "xf_split" }),
        json!({ "type": "Filter", "channels": [2, 3], "names": ["xf_lp", "xf_delay"] }),
        json!({ "type": "Mixer", "name": "xf_merge" }),
    ];
    Spatial { filters, mixers, steps }
}

/// Biquad RBJ (Audio EQ Cookbook) para a simulação do R.A.C.E.
#[derive(Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn new(highpass: bool, freq: f64, fs: f64) -> Self {
        let w0 = 2.0 * PI * freq / fs;
        let (cw, alpha) = (w0.cos(), w0.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2));
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = if highpass {
            ((1.0 + cw) / 2.0, -(1.0 + cw), (1.0 + cw) / 2.0)
        } else {
            ((1.0 - cw) / 2.0, 1.0 - cw, (1.0 - cw) / 2.0)
        };
        Self { b0: b0 / a0, b1: b1 / a0, b2: b2 / a0, a1: -2.0 * cw / a0, a2: (1.0 - alpha) / a0, z1: 0.0, z2: 0.0 }
    }
    // Forma direta II transposta.
    fn run(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Simula a recursão com impulso em L (R = 0) e devolve (h_same, h_cross, atraso em amostras).
pub fn race_impulse(a: &Ambiophonics, fs: f64) -> (Vec<f64>, Vec<f64>, usize) {
    let g = db_to_lin(-a.attenuation_db);
    let d = ((a.delay_us * 1e-6 * fs).round() as usize).max(1);
    const MAX: usize = 8192;
    // Passa-faixa dentro do laço, um por sentido (L→R e R→L).
    let bp = || (Biquad::new(true, a.low_hz, fs), Biquad::new(false, a.high_hz, fs));
    let (mut bp_to_l, mut bp_to_r) = (bp(), bp());
    let (mut lo, mut ro) = (vec![0.0; MAX], vec![0.0; MAX]);
    let mut quiet = 0;
    let mut len = MAX;
    for n in 0..MAX {
        let x = if n == 0 { 1.0 } else { 0.0 };
        let r_del = if n >= d { ro[n - d] } else { 0.0 };
        let l_del = if n >= d { lo[n - d] } else { 0.0 };
        let fb_l = bp_to_l.1.run(bp_to_l.0.run(r_del));
        let fb_r = bp_to_r.1.run(bp_to_r.0.run(l_del));
        lo[n] = x - g * fb_l;
        ro[n] = -g * fb_r;
        // Para quando o laço morreu (−100 dB por 4·d amostras seguidas).
        if n > 64 && lo[n].abs() < 1e-5 && ro[n].abs() < 1e-5 {
            quiet += 1;
            if quiet > 4 * d + 256 {
                len = n + 1;
                break;
            }
        } else {
            quiet = 0;
        }
    }
    lo.truncate(len);
    ro.truncate(len);
    (lo, ro, d)
}

/// Pico (dB) da resposta de um FIR entre 20 Hz e 20 kHz.
fn peak_db(h: &[f64], fs: f64) -> f64 {
    let mut peak: f64 = 0.0;
    for i in 0..240 {
        let f = 20.0 * (1000f64).powf(i as f64 / 239.0);
        let w = 2.0 * PI * f / fs;
        let (mut re, mut im) = (0.0, 0.0);
        for (n, v) in h.iter().enumerate() {
            re += v * (w * n as f64).cos();
            im -= v * (w * n as f64).sin();
        }
        peak = peak.max((re * re + im * im).sqrt());
    }
    20.0 * peak.max(1e-12).log10()
}

/// R.A.C.E. como dois FIR, com compensação automática de ganho (o pior caso entre sinal em fase e
/// em contrafase não passa de 0 dB). Devolve também o atraso efetivo e o ganho aplicado.
pub fn ambiophonics(a: &Ambiophonics, fs: f64) -> (Spatial, f64, f64) {
    let (same, cross, d) = race_impulse(a, fs);
    let mid: Vec<f64> = same.iter().zip(&cross).map(|(s, c)| s + c).collect();
    let side: Vec<f64> = same.iter().zip(&cross).map(|(s, c)| s - c).collect();
    let peak = peak_db(&mid, fs).max(peak_db(&side, fs)).max(0.0);
    let comp = db_to_lin(-peak);
    let mut filters = Map::new();
    filters.insert("race_same".into(), json!({ "type": "Conv", "parameters": { "type": "Values", "values": same } }));
    filters.insert("race_cross".into(), json!({ "type": "Conv", "parameters": { "type": "Values", "values": cross } }));
    let mut mixers = Map::new();
    mixers.insert("race_split".into(), split_mixer());
    mixers.insert("race_merge".into(), merge_mixer(comp, comp));
    let steps = vec![
        json!({ "type": "Mixer", "name": "race_split" }),
        json!({ "type": "Filter", "channels": [0, 1], "names": ["race_same"] }),
        json!({ "type": "Filter", "channels": [2, 3], "names": ["race_cross"] }),
        json!({ "type": "Mixer", "name": "race_merge" }),
    ];
    let delay_us = d as f64 / fs * 1e6;
    (Spatial { filters, mixers, steps }, delay_us, -peak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_converge_e_cancela() {
        let a = Ambiophonics { enabled: true, ..Default::default() };
        let (same, cross, d) = race_impulse(&a, 48000.0);
        assert_eq!(d, 3, "70 µs a 48 kHz = 3,36 amostras → 3");
        assert!(same.len() < 8192, "recursão precisa morrer antes do limite (len {})", same.len());
        assert_eq!(same[0], 1.0);
        // O primeiro termo cruzado (n = d) é negativo: cancelamento.
        let first_cross = cross.iter().position(|v| v.abs() > 1e-6).unwrap();
        assert!(first_cross >= d && cross[first_cross] < 0.0);
        let (_, delay_us, gain_db) = ambiophonics(&a, 48000.0);
        assert!((delay_us - 62.5).abs() < 0.01);
        assert!(gain_db <= 0.0 && gain_db > -20.0, "compensação {gain_db} dB");
    }

    #[test]
    fn crossfeed_ganhos_normalizados() {
        let s = crossfeed(&Crossfeed { enabled: true, cutoff_hz: 700.0, level_db: 4.5 });
        let m = &s.mixers["xf_merge"]["mapping"][0]["sources"];
        let sum = m[0]["gain"].as_f64().unwrap() + m[1]["gain"].as_f64().unwrap();
        assert!((sum - 1.0).abs() < 1e-9);
    }
}
