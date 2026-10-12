//! Anonymous usage ping: one request per day with a random install id, the version and the OS.
//! Off when the build has no `QT_STATS_URL` or the user turns off `usage_stats`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::{settings, AppState};

const ENDPOINT: Option<&str> = option_env!("QT_STATS_URL");

/// A random id stored next to the settings; created on first use.
fn install_id(app: &AppHandle) -> String {
    let path = settings::data_dir(app).join("install-id");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim().to_string();
        if valid_id(&id) {
            return id;
        }
    }
    let id = new_id();
    let _ = std::fs::write(&path, &id);
    id
}

fn new_id() -> String {
    // RandomState is seeded from the OS; two hashers give 128 random bits.
    (0..2)
        .map(|_| {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            );
            format!("{:016x}", hasher.finish())
        })
        .collect()
}

fn valid_id(id: &str) -> bool {
    (8..=64).contains(&id.len()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn body(id: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "app",
        "id": id,
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
    })
}

/// Pings 10 minutes after start, so a first run can turn it off before anything is sent,
/// then every 24 hours; a failed ping is retried after an hour.
pub fn start(app: AppHandle) {
    let Some(endpoint) = ENDPOINT.map(str::trim).filter(|url| !url.is_empty()) else {
        return;
    };
    let url = format!("{}/ping", endpoint.trim_end_matches('/'));
    tauri::async_runtime::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
        {
            Ok(client) => client,
            Err(_) => return,
        };
        tokio::time::sleep(Duration::from_secs(600)).await;
        loop {
            let mut wait = 24 * 3600;
            let enabled = app.state::<AppState>().settings.read().usage_stats;
            if enabled {
                let id = install_id(&app);
                let sent = client.post(&url).json(&body(&id)).send().await;
                match sent.and_then(|response| response.error_for_status()) {
                    Ok(_) => {}
                    Err(error) => {
                        crate::log(format!("stats: {error}"));
                        wait = 3600;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(wait)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_random_and_valid() {
        let (a, b) = (new_id(), new_id());
        assert_ne!(a, b);
        assert!(valid_id(&a) && a.len() == 32);
    }

    #[test]
    fn body_has_no_personal_data() {
        let value = body("abcdef0123456789");
        let keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys.len(), 4);
        assert_eq!(value["kind"], "app");
    }
}
