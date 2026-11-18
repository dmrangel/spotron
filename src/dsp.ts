// Tipos e comandos do DSP (espelham src-tauri/src/dsp/preset.rs).
import { invoke } from "@tauri-apps/api/core";

export type FilterType = "peaking" | "lowshelf" | "highshelf" | "lowpass" | "highpass" | "notch" | "bandpass" | "allpass";

export const FILTER_LABELS: Record<FilterType, string> = {
  peaking: "Peaking",
  lowshelf: "Low shelf",
  highshelf: "High shelf",
  lowpass: "Passa-baixa",
  highpass: "Passa-alta",
  notch: "Notch",
  bandpass: "Passa-banda",
  allpass: "All-pass",
};

export const hasGain = (t: FilterType) => t === "peaking" || t === "lowshelf" || t === "highshelf";

export interface Band {
  type: FilterType;
  freq: number;
  gain_db: number;
  q: number;
  enabled: boolean;
}

export interface Correction {
  name: string;
  preamp_db: number;
  filters: Band[];
  enabled: boolean;
}

export interface Preset {
  name: string;
  preamp_db: number;
  filters: Band[];
  correction: Correction | null;
  limiter: { enabled: boolean; clip_limit_db: number };
  [extra: string]: unknown;
}

export interface DspSettings {
  enabled: boolean;
  preset: Preset;
}

export interface DspStatus {
  running: boolean;
  state: string | null;
  capture_rms: number[] | null;
  playback_rms: number[] | null;
  clipped_samples: number | null;
  load: number | null;
  error: string | null;
}

export const dsp = {
  get: () => invoke<DspSettings>("dsp_get"),
  setPreset: (preset: Preset) => invoke<void>("dsp_set_preset", { preset }),
  setEnabled: (enabled: boolean) => invoke<void>("dsp_set_enabled", { enabled }),
  status: () => invoke<DspStatus>("dsp_status"),
  parseAutoeq: (name: string, text: string) => invoke<Correction>("autoeq_parse", { name, text }),
  presetList: () => invoke<string[]>("preset_list"),
  presetSave: (preset: Preset, name: string) => invoke<void>("preset_save", { preset, name }),
  presetLoad: (name: string) => invoke<Preset>("preset_load", { name }),
  presetDelete: (name: string) => invoke<void>("preset_delete", { name }),
  openVolumeMixer: () => invoke<void>("open_volume_mixer"),
};
