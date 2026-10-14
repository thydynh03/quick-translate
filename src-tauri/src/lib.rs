use std::collections::HashMap;
use std::io::Write;
use std::sync::atomic::AtomicU64;

use parking_lot::{Mutex, RwLock};
use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

mod capture;
mod clipboard;
mod engine;
mod history;
mod platform;
mod popup;
mod sentence;
mod settings;
mod subtitles;
mod telemetry;
mod trigger;
mod updater;
mod vocab;

use settings::Settings;

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub popup: Mutex<popup::PopupState>,
    pub cache: Mutex<HashMap<String, engine::Translation>>,
    /// Counts mouse presses; a pending gesture is dropped when it changes.
    pub gesture: AtomicU64,
}

/// Appends a line to `quick-translate.log` in the temp folder.
pub fn log(message: impl AsRef<str>) {
    let path = std::env::temp_dir().join("quick-translate.log");
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "{secs} {}", message.as_ref());
    }
}

/// The application the user last worked in, other than this one.
static LAST_APP: Mutex<Option<String>> = Mutex::new(None);

/// Notes which application is in front. Call before a window of ours takes the focus.
pub fn remember_app() -> Option<String> {
    let own = std::env::current_exe()
        .ok()
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().to_lowercase()));
    let mut last = LAST_APP.lock();
    if let Some(name) = platform::foreground_app().filter(|name| Some(name) != own.as_ref()) {
        *last = Some(name);
    }
    last.clone()
}

/// The settings for the application in front: the general ones plus its profile.
pub fn effective(app: &AppHandle) -> Settings {
    let name = remember_app();
    app.state::<AppState>().settings.read().for_app(name.as_deref())
}

/// Translates a subtitle file; answers with the path of the translated file.
#[tauri::command]
async fn subtitle_translate(app: AppHandle, path: String) -> Result<String, String> {
    subtitles::translate_file(&app, std::path::Path::new(&path))
        .await
        .map(|output| output.display().to_string())
        .map_err(|error| format!("{error:#}"))
}

/// Lets the settings window offer the last used application for a new profile.
/// The version that ran before this one, when the app has just been updated.
struct UpdatedFrom(Mutex<Option<String>>);

/// Asked once by the settings window, to tell the user about the update.
#[tauri::command]
fn updated_from(state: tauri::State<UpdatedFrom>) -> Option<String> {
    let previous = state.0.lock().take();
    if let Some(previous) = &previous {
        log(format!("updated from {previous:?}"));
    }
    previous
}

#[tauri::command]
fn last_app() -> Option<String> {
    LAST_APP.lock().clone()
}

fn show_settings(app: &AppHandle) {
    remember_app();
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// (Re)registers the global shortcuts from the current settings. Returns the
/// shortcuts that could not be registered.
fn register_hotkeys(app: &AppHandle) -> Vec<String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();
    let hotkey = app.state::<AppState>().settings.read().hotkey.clone();

    let mut failed = Vec::new();
    if !hotkey.trim().is_empty() {
        let result = shortcuts.on_shortcut(hotkey.as_str(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                popup::translate_selection(app);
            }
        });
        if let Err(error) = result {
            log(format!("hotkey {hotkey}: {error}"));
            failed.push(hotkey);
        }
    }

    let ocr_hotkey = app.state::<AppState>().settings.read().ocr_hotkey.clone();
    if !ocr_hotkey.trim().is_empty() {
        let result = shortcuts.on_shortcut(ocr_hotkey.as_str(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                capture::start(app);
            }
        });
        if let Err(error) = result {
            log(format!("hotkey {ocr_hotkey}: {error}"));
            failed.push(ocr_hotkey);
        }
    }

    let window_hotkey = app.state::<AppState>().settings.read().window_hotkey.clone();
    if !window_hotkey.trim().is_empty() {
        let result = shortcuts.on_shortcut(window_hotkey.as_str(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                capture::start_window(app);
            }
        });
        if let Err(error) = result {
            log(format!("hotkey {window_hotkey}: {error}"));
            failed.push(window_hotkey);
        }
    }

    let live_hotkey = app.state::<AppState>().settings.read().live_hotkey.clone();
    if !live_hotkey.trim().is_empty() {
        let result = shortcuts.on_shortcut(live_hotkey.as_str(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                capture::toggle_live(app);
            }
        });
        if let Err(error) = result {
            log(format!("hotkey {live_hotkey}: {error}"));
            failed.push(live_hotkey);
        }
    }
    let clipboard_hotkey = app.state::<AppState>().settings.read().clipboard_hotkey.clone();
    if !clipboard_hotkey.trim().is_empty() {
        let result = shortcuts.on_shortcut(clipboard_hotkey.as_str(), |app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                clipboard::toggle(app);
            }
        });
        if let Err(error) = result {
            log(format!("hotkey {clipboard_hotkey}: {error}"));
            failed.push(clipboard_hotkey);
        }
    }
    failed
}

#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    app.state::<AppState>().settings.read().clone()
}

/// Saves the settings and applies them. Returns shortcuts that are taken.
#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) -> Result<Vec<String>, String> {
    settings::save(&app, &settings).map_err(|e| e.to_string())?;
    *app.state::<AppState>().settings.write() = settings;
    app.state::<AppState>().cache.lock().clear();
    let failed = register_hotkeys(&app);
    sync_tray(&app);
    let _ = app.emit("settings-changed", ());
    Ok(failed)
}

/// Translates text typed into the settings window's test box.
#[tauri::command]
async fn translate_now(app: AppHandle, text: String) -> Result<engine::Translation, String> {
    let settings = app.state::<AppState>().settings.read().clone();
    engine::translate(&settings, &engine::offline::models_dir(&app), text.trim())
        .await
        .map_err(|e| e.to_string())
}

/// The tray menu's check item, kept so a left click on the icon can update it.
struct TrayItems {
    auto: tauri::menu::CheckMenuItem<tauri::Wry>,
    update: tauri::menu::MenuItem<tauri::Wry>,
}

/// Renames the tray's update item once a newer release is found.
pub fn show_update_in_tray(app: &AppHandle, latest: &str) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.update.set_text(format!("Cập nhật lên bản {latest}…"));
    }
}

/// Makes the tray icon, tooltip and menu reflect whether the app is on.
fn sync_tray(app: &AppHandle) {
    let on = app.state::<AppState>().settings.read().auto_trigger;
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.auto.set_checked(on);
    }
    let Some(tray) = app.tray_by_id("main") else { return };
    let icon = if on { app.default_window_icon().cloned() } else { Some(tauri::include_image!("icons/tray-off.png")) };
    let _ = tray.set_icon(icon);
    let state = if on { "đang bật" } else { "đang tắt" };
    let _ = tray.set_tooltip(Some(format!("Quick Translate: {state} (bấm để đổi)")));
}

/// Turns the selection icon on or off, like the E/V switch of an input method.
fn toggle_auto(app: &AppHandle) {
    let updated = {
        let state = app.state::<AppState>();
        let mut settings = state.settings.write();
        settings.auto_trigger = !settings.auto_trigger;
        settings.clone()
    };
    if !updated.auto_trigger {
        popup::dismiss(app);
    }
    let _ = settings::save(app, &updated);
    let _ = app.emit("settings-changed", ());
    sync_tray(app);
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<AppState>().settings.read().clone();
    let translate = MenuItemBuilder::with_id("translate", format!("Dịch vùng bôi đen ({})", settings.hotkey)).build(app)?;
    let ocr = MenuItemBuilder::with_id("ocr", format!("Dịch vùng màn hình ({})", settings.ocr_hotkey)).build(app)?;
    let live = MenuItemBuilder::with_id("live", format!("Bật/tắt dịch thời gian thực ({})", settings.live_hotkey)).build(app)?;
    let auto = CheckMenuItemBuilder::with_id("auto", "Tự hiện khi bôi đen")
        .checked(settings.auto_trigger)
        .build(app)?;
    let open = MenuItemBuilder::with_id("settings", "Cài đặt…").build(app)?;
    let update = MenuItemBuilder::with_id("update", "Kiểm tra cập nhật…").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Thoát").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&translate, &ocr, &live, &auto]).separator().items(&[&open, &update, &quit]).build()?;
    app.manage(TrayItems { auto, update });

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        // Left click switches on/off, double click opens the settings, right click shows the menu.
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => {
                toggle_auto(tray.app_handle())
            }
            TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => show_settings(tray.app_handle()),
            _ => {}
        })
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "translate" => popup::translate_selection(app),
            "auto" => toggle_auto(app),
            "ocr" => capture::start(app),
            "live" => capture::toggle_live(app),
            "settings" => show_settings(app),
            "update" => {
                // The About tab shows the version check and the install button.
                show_settings(app);
                let _ = app.emit("show-update", ());
            }
            "quit" => {
                capture::leave_captions();
                app.exit(0)
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    sync_tray(app);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // A second launch just brings up the settings of the running app.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_settings(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            translate_now,
            last_app,
            popup::popup_state,
            popup::popup_activate,
            popup::popup_resize,
            popup::popup_close,
            popup::popup_pin,
            popup::popup_lookup,
            popup::copy_text,
            capture::capture_region,
            capture::capture_cancel,
            capture::live_begin,
            capture::live_listen,
            clipboard::clipboard_list,
            clipboard::clipboard_paste,
            clipboard::clipboard_pin,
            clipboard::clipboard_remove,
            clipboard::clipboard_clear,
            clipboard::clipboard_hide,
            clipboard::clipboard_translate,
            subtitle_translate,
            history::history_list,
            history::history_clear,
            updater::update_check,
            updater::update_install,
            updated_from,
            vocab::vocab_add,
            vocab::vocab_list,
            vocab::vocab_remove,
            vocab::vocab_has,
            engine::offline::offline_models,
            engine::offline::offline_download,
            engine::offline::offline_remove,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let first_run = !settings::data_dir(&handle).join("settings.json").exists();
            // A version other than the one that ran last: the app was just updated.
            // The installer starts it again without a window, so it has to say so itself.
            // Managed first: the settings window may ask while the rest is still being set up.
            let version_file = settings::data_dir(&handle).join("last-version");
            let current = app.package_info().version.to_string();
            let previous = std::fs::read_to_string(&version_file).map(|text| text.trim().to_string()).ok();
            // Without the note of this version, every start would look like an update.
            let noted = std::fs::write(&version_file, &current).is_ok();
            let updated = noted && !first_run && previous.as_deref() != Some(current.as_str());
            app.manage(UpdatedFrom(Mutex::new(updated.then(|| previous.unwrap_or_default()))));
            let loaded = settings::load(&handle);
            if first_run {
                let _ = settings::save(&handle, &loaded);
                // Runs in the background from sign-in by default; the settings can turn it off.
                if !cfg!(debug_assertions) {
                    use tauri_plugin_autostart::ManagerExt;
                    let _ = handle.autolaunch().enable();
                }
            }
            app.manage(AppState {
                settings: RwLock::new(loaded),
                popup: Mutex::new(popup::PopupState::default()),
                cache: Mutex::new(HashMap::new()),
                gesture: AtomicU64::new(0),
            });
            log("started");

            let overlay = popup::create_window(&handle)?;
            platform::prepare_overlay(&overlay);
            capture::create_window(&handle)?;
            clipboard::create_window(&handle)?;
            clipboard::start(handle.clone());
            build_tray(&handle)?;
            register_hotkeys(&handle);
            telemetry::start(handle.clone());
            app.manage(updater::UpdateState(Mutex::new(None)));

            updater::start(handle.clone());
            engine::warm(&handle.state::<AppState>().settings.read());

            let (tx, rx) = std::sync::mpsc::channel();
            platform::start_mouse_hook(tx);
            let worker = handle.clone();
            std::thread::spawn(move || trigger::run(worker, rx));
            let hover = handle.clone();
            std::thread::spawn(move || trigger::run_hover(hover));

            let hidden = std::env::args().any(|arg| arg == "--hidden");
            let open_settings = std::env::args().any(|arg| arg == "--settings");
            if ((first_run || updated) && !hidden) || open_settings {
                show_settings(&handle);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                if window.label() == clipboard::LABEL {
                    clipboard::blurred(window.app_handle());
                }
            }
            // Closing the settings window keeps the app running in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
