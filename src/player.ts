// Estado do player (vem da página do SDK via evento "player-state") e comandos do backend.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { reactive } from "vue";

export interface Track {
  uri: string;
  name: string;
  artists: string;
  album: string | null;
  image: string | null;
}

export interface PlayerState {
  device_id: string | null;
  ready: boolean;
  active: boolean;
  paused: boolean;
  position: number;
  duration: number;
  ts: number;
  track: Track | null;
  error?: string;
}

export interface Status {
  client_id: string | null;
  logged_in: boolean;
  device_ready: boolean;
  redirect_uri: string;
}

export const player = reactive<{ state: PlayerState | null; logs: string[] }>({ state: null, logs: [] });

export function startListening() {
  listen<PlayerState>("player-state", (e) => {
    player.state = e.payload;
  });
  listen<string>("player-log", (e) => {
    player.logs = [...player.logs.slice(-49), e.payload];
  });
}

export const api = {
  status: () => invoke<Status>("app_status"),
  setClientId: (clientId: string) => invoke<void>("set_client_id", { clientId }),
  login: () => invoke<void>("login"),
  logout: () => invoke<void>("logout"),
  playUri: (input: string) => invoke<string>("play_uri", { input }),
  transferHere: () => invoke<void>("transfer_here"),
  pause: () => invoke<void>("pause"),
  resume: () => invoke<void>("resume"),
  next: () => invoke<void>("next"),
  prev: () => invoke<void>("prev"),
  seek: (positionMs: number) => invoke<void>("seek", { positionMs: Math.round(positionMs) }),
  setVolume: (percent: number) => invoke<void>("set_volume", { percent: Math.round(percent) }),
};

export function fmtTime(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
