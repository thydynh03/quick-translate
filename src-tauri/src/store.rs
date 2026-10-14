//! The Microsoft Store build: the same program, installed from an MSIX package.
//! The Store updates it, and it starts with Windows through the package's
//! startup task instead of the Run key, which a package cannot keep.

/// The `TaskId` of the startup task in the package manifest.
#[cfg(windows)]
const STARTUP_TASK: &str = "SwipelateStartup";

/// Whether this copy runs from an MSIX package, that is, was installed from the Store.
#[cfg(windows)]
pub fn packaged() -> bool {
    use std::sync::OnceLock;
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    static PACKAGED: OnceLock<bool> = OnceLock::new();
    *PACKAGED.get_or_init(|| {
        let mut length = 0u32;
        // With no buffer it asks for one when there is a package (APPMODEL_ERROR_NO_PACKAGE otherwise).
        let result = unsafe { GetCurrentPackageFullName(&mut length, None) };
        result == ERROR_INSUFFICIENT_BUFFER
    })
}

#[cfg(not(windows))]
pub fn packaged() -> bool {
    false
}

/// Whether Windows started the package at sign-in, the packaged form of `--hidden`.
#[cfg(windows)]
pub fn started_at_sign_in() -> bool {
    use windows::ApplicationModel::Activation::ActivationKind;
    use windows::ApplicationModel::AppInstance;
    packaged()
        && AppInstance::GetActivatedEventArgs()
            .and_then(|args| args.Kind())
            .is_ok_and(|kind| kind == ActivationKind::StartupTask)
}

#[cfg(not(windows))]
pub fn started_at_sign_in() -> bool {
    false
}

#[cfg(windows)]
fn startup_task() -> windows::core::Result<windows::ApplicationModel::StartupTask> {
    windows::ApplicationModel::StartupTask::GetAsync(&windows::core::HSTRING::from(STARTUP_TASK))?.join()
}

/// Whether the package's startup task is on.
#[cfg(windows)]
fn startup_enabled() -> Result<bool, String> {
    use windows::ApplicationModel::StartupTaskState;
    let state = startup_task().and_then(|task| task.State()).map_err(|e| e.to_string())?;
    Ok(state == StartupTaskState::Enabled || state == StartupTaskState::EnabledByPolicy)
}

#[cfg(windows)]
fn set_startup(on: bool) -> Result<bool, String> {
    use windows::ApplicationModel::StartupTaskState;
    let task = startup_task().map_err(|e| e.to_string())?;
    if !on {
        task.Disable().map_err(|e| e.to_string())?;
        return Ok(false);
    }
    match task.RequestEnableAsync().and_then(|request| request.join()).map_err(|e| e.to_string())? {
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => Ok(true),
        // Turned off in Task Manager or the Settings app: only the user can turn it back on there.
        StartupTaskState::DisabledByUser => {
            Err("Bạn đã tắt Swipelate trong mục Khởi động của Windows. Bật lại ở Cài đặt › Ứng dụng › Khởi động.".into())
        }
        _ => Err("Windows không cho Swipelate khởi động cùng hệ thống.".into()),
    }
}

/// Whether the app starts with the system, from whichever place this copy uses.
#[tauri::command]
pub async fn autostart_get(app: tauri::AppHandle) -> Result<bool, String> {
    #[cfg(windows)]
    if packaged() {
        return startup_enabled();
    }
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Turns starting with the system on or off and reports how it ended up.
#[tauri::command]
pub async fn autostart_set(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    #[cfg(windows)]
    if packaged() {
        return set_startup(on);
    }
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if on { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    Ok(on)
}

/// Lets the settings window leave out what the Store does itself.
#[tauri::command]
pub fn store_build() -> bool {
    packaged()
}
