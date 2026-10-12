//! New versions: the latest GitHub release is compared with this build; the
//! installer is downloaded, checked against SHA256SUMS.txt and started.

use std::path::{Path, PathBuf};
use std::time::Duration;

use futures_util::StreamExt;
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::{log, AppState};

const LATEST: &str = "https://api.github.com/repos/thydynh03/quick-translate/releases/latest";
const CHECKSUMS: &str = "SHA256SUMS.txt";

#[cfg(windows)]
const ASSET: &str = "QuickTranslate-Setup-x64.exe";
#[cfg(target_os = "macos")]
const ASSET: &str = "QuickTranslate-macOS-AppleSilicon.dmg";
#[cfg(not(any(windows, target_os = "macos")))]
const ASSET: &str = "";

#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub published: String,
    pub page: String,
    /// Download of the installer for this system, when the release has one.
    #[serde(skip)]
    pub installer: Option<String>,
    #[serde(skip)]
    pub checksums: Option<String>,
}

/// The last check, for the tray and the settings window.
pub struct UpdateState(pub Mutex<Option<UpdateInfo>>);

/// "v0.1.10" -> [0, 1, 10]; anything after a '-' is ignored.
fn parse_version(text: &str) -> Vec<u64> {
    text.trim()
        .trim_start_matches('v')
        .split('-')
        .next()
        .unwrap_or("")
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let (mut a, mut b) = (parse_version(latest), parse_version(current));
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a > b
}

fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("QuickTranslate/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        // A stalled connection must end in an error, not a stuck button.
        .read_timeout(Duration::from_secs(60))
        .timeout(Duration::from_secs(30 * 60))
        .build()
}

fn asset_url(release: &Value, name: &str) -> Option<String> {
    release.get("assets")?.as_array()?.iter().find_map(|asset| {
        (asset.get("name")?.as_str()? == name).then(|| asset.get("browser_download_url")?.as_str().map(String::from))?
    })
}

pub fn info_from_release(release: &Value, current: &str) -> UpdateInfo {
    let field = |name: &str| release.get(name).and_then(Value::as_str).unwrap_or_default().to_string();
    let latest = field("tag_name").trim_start_matches('v').to_string();
    let installer = if ASSET.is_empty() { None } else { asset_url(release, ASSET) };
    UpdateInfo {
        current: current.to_string(),
        // Only offer what this system can install.
        available: !latest.is_empty() && is_newer(&latest, current) && installer.is_some(),
        latest,
        notes: field("body"),
        published: field("published_at"),
        page: field("html_url"),
        installer,
        checksums: asset_url(release, CHECKSUMS),
    }
}

pub async fn check(app: &AppHandle) -> anyhow::Result<UpdateInfo> {
    let response = client()?.get(LATEST).header("Accept", "application/vnd.github+json").send().await?;
    anyhow::ensure!(response.status().is_success(), "GitHub trả lỗi {}", response.status());
    let info = info_from_release(&response.json().await?, env!("CARGO_PKG_VERSION"));
    *app.state::<UpdateState>().0.lock() = Some(info.clone());
    if info.available {
        crate::show_update_in_tray(app, &info.latest);
        let _ = app.emit("update-available", &info);
    }
    Ok(info)
}

/// Checks shortly after start and then once a day, unless turned off.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            if app.state::<AppState>().settings.read().check_updates {
                match check(&app).await {
                    Ok(info) => log(format!("update: latest {} (current {})", info.latest, info.current)),
                    Err(error) => log(format!("update: check failed: {error:#}")),
                }
            }
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
        }
    });
}

/// The digest listed for `name` in a SHA256SUMS file ("<hex>  <name>" lines).
pub fn listed_digest(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let digest = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name).then(|| digest.to_lowercase())
    })
}

async fn download(app: &AppHandle, url: &str, path: &Path) -> anyhow::Result<String> {
    let response = client()?.get(url).send().await?;
    anyhow::ensure!(response.status().is_success(), "Tải bộ cài lỗi {}", response.status());
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(path).await?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut shown = u64::MAX;
    let mut stream = response.bytes_stream();
    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        hasher.update(&chunk);
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        // One event per percent, not per chunk.
        let percent = if total > 0 { done * 100 / total } else { done >> 20 };
        if percent != shown {
            shown = percent;
            let _ = app.emit("update-progress", serde_json::json!({ "done": done, "total": total }));
        }
    }
    file.flush().await?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Downloads the installer, verifies it, starts it and quits so the files can be replaced.
pub async fn install(app: &AppHandle) -> anyhow::Result<()> {
    let info = match app.state::<UpdateState>().0.lock().clone() {
        Some(info) if info.available => info,
        _ => anyhow::bail!("Chưa có bản mới để cài"),
    };
    let url = info.installer.clone().ok_or_else(|| anyhow::anyhow!("Bản phát hành không có bộ cài cho hệ điều hành này"))?;
    let sums_url = info.checksums.clone().ok_or_else(|| anyhow::anyhow!("Bản phát hành thiếu SHA256SUMS.txt"))?;
    let response = client()?.get(&sums_url).send().await?;
    anyhow::ensure!(response.status().is_success(), "Tải SHA256SUMS.txt lỗi {}", response.status());
    let sums = response.text().await?;
    let expected = listed_digest(&sums, ASSET).ok_or_else(|| anyhow::anyhow!("SHA256SUMS.txt không có {ASSET}"))?;

    let dir: PathBuf = std::env::temp_dir().join("quick-translate-update");
    tokio::fs::create_dir_all(&dir).await?;
    let path = dir.join(ASSET);
    let actual = download(app, &url, &path).await?;
    if actual != expected {
        let _ = tokio::fs::remove_file(&path).await;
        anyhow::bail!("Bộ cài tải về không khớp mã SHA-256, đã huỷ");
    }
    log(format!("update: installing {} from {}", info.latest, path.display()));
    launch(app, &path)
}

#[cfg(windows)]
fn launch(app: &AppHandle, installer: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    // Passive install (progress bar only), then start the new version.
    let exe = std::env::current_exe()?;
    let script = format!(
        "start \"\" /wait \"{}\" /P & start \"\" \"{}\"",
        installer.display(),
        exe.display()
    );
    std::process::Command::new("cmd").args(["/C", &script]).creation_flags(CREATE_NO_WINDOW).spawn()?;
    app.exit(0);
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch(_app: &AppHandle, installer: &Path) -> anyhow::Result<()> {
    // The disk image opens in Finder; dragging the app to Applications replaces it.
    std::process::Command::new("open").arg(installer).spawn()?;
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn launch(_app: &AppHandle, _installer: &Path) -> anyhow::Result<()> {
    anyhow::bail!("Hệ điều hành này không hỗ trợ tự cập nhật")
}

#[tauri::command]
pub async fn update_check(app: AppHandle) -> Result<UpdateInfo, String> {
    check(&app).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_install(app: AppHandle) -> Result<(), String> {
    install(&app).await.map_err(|e| {
        log(format!("update: install failed: {e:#}"));
        e.to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(is_newer("v0.2.0", "0.1.99"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.1.10", "0.1.10"));
        assert!(!is_newer("0.1.9", "0.1.10"));
        assert!(!is_newer("0.1.10-beta", "0.1.10"));
    }

    #[test]
    fn reads_the_release() {
        let release = serde_json::json!({
            "tag_name": "v9.0.0",
            "body": "notes",
            "html_url": "https://github.com/x",
            "published_at": "2026-10-12T00:00:00Z",
            "assets": [
                { "name": "SHA256SUMS.txt", "browser_download_url": "https://d/sums" },
                { "name": ASSET, "browser_download_url": "https://d/setup" }
            ]
        });
        let info = info_from_release(&release, "0.1.10");
        assert_eq!(info.available, !ASSET.is_empty());
        assert_eq!(info.latest, "9.0.0");
        assert_eq!(info.checksums.as_deref(), Some("https://d/sums"));
        if !ASSET.is_empty() {
            assert_eq!(info.installer.as_deref(), Some("https://d/setup"));
        }
    }

    #[test]
    fn finds_the_digest_for_a_file() {
        let sums = "ABC123  QuickTranslate-Setup-x64.exe\ndef456 *QuickTranslate-macOS-AppleSilicon.dmg\n";
        assert_eq!(listed_digest(sums, "QuickTranslate-Setup-x64.exe").as_deref(), Some("abc123"));
        assert_eq!(listed_digest(sums, "QuickTranslate-macOS-AppleSilicon.dmg").as_deref(), Some("def456"));
        assert_eq!(listed_digest(sums, "other"), None);
    }
}
