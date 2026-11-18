// Sem janela de console no build de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod dsp;
mod dsp_commands;
mod player;
mod procjob;
mod spotify;

use std::sync::Mutex;

pub struct AppState {
    pub config: Mutex<config::Config>,
    pub spotify: spotify::Spotify,
    pub dsp: dsp::Dsp,
    /// device_id do Web Playback SDK (None enquanto o player não está pronto).
    pub device_id: Mutex<Option<String>>,
    pub edge: Mutex<Option<player::Edge>>,
}

impl AppState {
    pub fn client_id(&self) -> Option<String> {
        self.config.lock().unwrap().client_id.clone()
    }
}

fn main() {
    let state = AppState {
        config: Mutex::new(config::load_config()),
        spotify: spotify::Spotify::new(),
        dsp: dsp::Dsp::new(),
        device_id: Mutex::new(None),
        edge: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .setup(|app| {
            let handle = app.handle().clone();
            player::start_server(handle.clone())?;
            dsp::start(handle.clone());
            use tauri::Manager;
            let st = handle.state::<AppState>();
            if st.client_id().is_some() && st.spotify.logged_in() {
                if let Err(e) = player::ensure_edge(&handle) {
                    player::log_line(&handle, &format!("Edge: {e}"));
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_status,
            commands::set_client_id,
            commands::login,
            commands::logout,
            commands::play_uri,
            commands::transfer_here,
            commands::pause,
            commands::resume,
            commands::next,
            commands::prev,
            commands::seek,
            commands::set_volume,
            dsp_commands::dsp_get,
            dsp_commands::dsp_set_preset,
            dsp_commands::dsp_set_enabled,
            dsp_commands::dsp_status,
            dsp_commands::autoeq_parse,
            dsp_commands::race_info,
            dsp_commands::preset_list,
            dsp_commands::preset_save,
            dsp_commands::preset_load,
            dsp_commands::preset_delete,
            dsp_commands::open_volume_mixer,
        ])
        .build(tauri::generate_context!())
        .expect("erro ao iniciar o Spotron")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                app.state::<AppState>().dsp.shutdown();
                player::stop_edge(app);
            }
        });
}
