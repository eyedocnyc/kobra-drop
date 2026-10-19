#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kobra_converter::{profiles, Conversion};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager, State};

#[derive(Default)]
struct Session {
    busy: bool,
    pending: Option<String>,
    installing: bool,
    last_output: Option<String>,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    profile_root: Option<String>,
    slicer_exe: Option<String>,
    automatic_updates: Option<bool>,
}
fn preference_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|p| p.join("preferences.json"))
        .map_err(|e| e.to_string())
}
fn prefs(app: &tauri::AppHandle) -> Preferences {
    preference_path(app)
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn get_preferences(app: tauri::AppHandle) -> Preferences {
    prefs(&app)
}
#[tauri::command]
fn save_preferences(app: tauri::AppHandle, preferences: Preferences) -> Result<(), String> {
    let path = preference_path(&app)?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, serde_json::to_vec_pretty(&preferences).unwrap())
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn discover_profiles(app: tauri::AppHandle) -> profiles::Discovery {
    let p = prefs(&app);
    profiles::status(p.profile_root.as_deref().map(std::path::Path::new))
}
#[tauri::command]
fn pending_file(state: State<'_, Mutex<Session>>) -> Result<Option<String>, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.pending.clone())
}
#[tauri::command]
fn select_file(path: Option<String>, state: State<'_, Mutex<Session>>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.busy || s.installing {
        return Err("Please wait for the current operation to finish".into());
    }
    if let Some(p) = &path {
        if !p.to_lowercase().ends_with(".3mf") {
            return Err("Choose an editable .3mf project".into());
        }
    }
    s.pending = path;
    Ok(())
}
#[tauri::command]
async fn convert_project(app: tauri::AppHandle) -> Result<Conversion, String> {
    let input = {
        let state = app.state::<Mutex<Session>>();
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.busy || s.installing {
            return Err("Another operation is still running".into());
        }
        let p = s.pending.clone().ok_or("Choose a 3MF project first")?;
        s.busy = true;
        p
    };
    let p = prefs(&app);
    let events = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        kobra_converter::convert(
            std::path::Path::new(&input),
            p.profile_root.as_deref().map(std::path::Path::new),
            |stage| {
                let _ = events.emit("conversion-progress", stage);
            },
        )
        .map_err(|e| format!("{e:#}"))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|x| x);
    let state = app.state::<Mutex<Session>>();
    let mut s = state.lock().map_err(|e| e.to_string())?;
    s.busy = false;
    if let Ok(r) = &result {
        s.pending = None;
        s.last_output = Some(r.output.clone());
    }
    result
}
#[tauri::command]
fn open_in_slicer(app: tauri::AppHandle) -> Result<(), String> {
    let output = app
        .state::<Mutex<Session>>()
        .lock()
        .map_err(|e| e.to_string())?
        .last_output
        .clone()
        .ok_or("Convert a project first")?;
    let preferences = prefs(&app);
    let candidates = preferences.slicer_exe.into_iter().map(PathBuf::from).chain(
        std::env::var_os("ProgramFiles")
            .map(|p| PathBuf::from(p).join("AnycubicSlicerNext/AnycubicSlicerNext.exe")),
    );
    let exe = candidates
        .into_iter()
        .find(|p| p.is_file())
        .ok_or("Locate AnycubicSlicerNext.exe in Settings, then try again")?;
    if !exe
        .file_name()
        .map(|n| {
            n.to_string_lossy()
                .eq_ignore_ascii_case("AnycubicSlicerNext.exe")
        })
        .unwrap_or(false)
    {
        return Err("Choose AnycubicSlicerNext.exe".into());
    }
    // Direct executable + separate argv, never a shell or file association.
    std::process::Command::new(exe)
        .arg(output)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
#[derive(Serialize)]
struct UpdateConfig {
    configured: bool,
}
#[tauri::command]
fn update_config() -> UpdateConfig {
    UpdateConfig {
        configured: option_env!("KOBRA_UPDATE_REPOSITORY")
            .map(|s| !s.is_empty() && s.split('/').count() == 2)
            .unwrap_or(false)
            && option_env!("KOBRA_UPDATE_PUBLIC_KEY")
                .map(|s| !s.is_empty())
                .unwrap_or(false),
    }
}
#[tauri::command]
fn reserve_update(state: State<'_, Mutex<Session>>) -> Result<bool, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if s.busy || s.pending.is_some() || s.installing {
        Ok(false)
    } else {
        s.installing = true;
        Ok(true)
    }
}
#[tauri::command]
fn release_update(state: State<'_, Mutex<Session>>) {
    if let Ok(mut s) = state.lock() {
        s.installing = false;
    }
}

fn main() {
    let pending = std::env::args()
        .skip(1)
        .find(|s| s.to_lowercase().ends_with(".3mf"));
    tauri::Builder::default()
        .manage(Mutex::new(Session {
            pending,
            ..Session::default()
        }))
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if let Some(path) = args
                .into_iter()
                .skip(1)
                .find(|s| s.to_lowercase().ends_with(".3mf"))
            {
                let state = app.state::<Mutex<Session>>();
                if let Ok(mut s) = state.lock() {
                    if !s.busy && !s.installing && s.pending.is_none() {
                        s.pending = Some(path.clone());
                        let _ = app.emit("open-file", path);
                    } else {
                        let _ = app.emit(
                            "file-waiting",
                            "Finish or clear the current file, then open the other project again.",
                        );
                    }
                };
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_preferences,
            save_preferences,
            discover_profiles,
            pending_file,
            select_file,
            convert_project,
            open_in_slicer,
            update_config,
            reserve_update,
            release_update
        ])
        .run(tauri::generate_context!())
        .expect("Could not start Kobra Drop");
}
