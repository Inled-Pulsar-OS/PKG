use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

use crate::ootb::{self, OotbConfig};

pub struct OotbState {
    pub running: Mutex<bool>,
}

#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    progress: f64,
    status: String,
}

#[derive(Clone, serde::Serialize)]
struct LogPayload {
    message: String,
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn start_ootb_setup(
    app: AppHandle,
    fullname: String,
    username: String,
    password: String,
    language: String,
    keymap: String,
    timezone: String,
    avatar_path: Option<String>,
    state: State<'_, OotbState>,
) -> Result<(), String> {
    {
        let mut running = state.running.lock().map_err(|e| e.to_string())?;
        if *running {
            return Err("Setup already in progress".into());
        }
        *running = true;
    }

    let state_clone = state.inner();
    let app_clone = app.clone();

    let config = OotbConfig {
        fullname,
        username,
        password,
        language,
        keymap,
        timezone,
        avatar_path,
    };

    let result = tokio::task::spawn_blocking(move || {
        let log_fn = {
            let app = app_clone.clone();
            move |msg: String| {
                let _ = app.emit("ootb-log", LogPayload { message: msg });
            }
        };

        let progress_fn = {
            let app = app_clone.clone();
            move |pct: f64, msg: String| {
                let _ = app.emit(
                    "ootb-progress",
                    ProgressPayload {
                        progress: pct,
                        status: msg,
                    },
                );
            }
        };

        ootb::run_setup(&config, &log_fn, &progress_fn)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let mut running = state_clone.running.lock().map_err(|e| e.to_string())?;
    *running = false;

    result
}

#[tauri::command]
pub fn ootb_final_cleanup(username: String) -> Result<(), String> {
    let log_fn = |msg: String| {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/pulsar-ootb.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{}", msg)
            });
    };

    ootb::run_final_cleanup(&username, &log_fn)
}
