//! Persisted client settings (JSON on disk).
//!
//! Stored in the per-user config dir (e.g. `%APPDATA%\wankle-client\config.json`
//! on Windows, `~/.config/wankle-client/config.json` on Linux/macOS).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const WANKLE_ONLINE: &str = "https://wankle.online";
/// The mirror domain — both serve identical Wankle3D content. Selectable in the
/// menu; kept here as documented API.
#[allow(dead_code)]
pub const WANSHOT_LOL: &str = "https://wanshot.lol";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    /// Preset site URL (one of the known domains).
    pub site: String,
    /// Custom URL used when `use_custom_url` is true.
    pub custom_url: String,
    pub use_custom_url: bool,

    /// ANGLE backend select: auto | d3d11 | d3d9 | gl | vulkan | swiftshader.
    /// (Only meaningful on Windows / WebView2.)
    pub rendering_api: String,
    /// Master GPU acceleration switch. When false, passes `--disable-gpu`.
    pub gpu_acceleration: bool,
    /// Unlock the frame rate (`--disable-frame-rate-limit --disable-gpu-vsync`).
    pub fps_unlock: bool,

    /// CPU throttle multiplier applied live via CDP. 1.0 = no throttle.
    pub cpu_throttle: f32,
    /// Process priority class: Idle | Below Normal | Normal | Above Normal | High.
    pub priority: String,

    /// Window mode: Windowed | Maximized | Borderless Fullscreen.
    pub window_mode: String,

    /// Advanced: extra raw Chromium flags appended verbatim.
    pub extra_flags: Vec<String>,
    /// Advanced: default flags to omit (matched by exact string).
    pub disabled_default_flags: Vec<String>,

    /// Key that toggles the overlay menu (F1 | F2 | Insert | Backtick).
    pub menu_hotkey: String,
    /// Show the launcher/menu overlay on startup instead of going straight in.
    pub open_menu_on_startup: bool,
    pub devtools: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site: WANKLE_ONLINE.to_string(),
            custom_url: String::new(),
            use_custom_url: false,
            rendering_api: "auto".to_string(),
            gpu_acceleration: true,
            fps_unlock: true,
            cpu_throttle: 1.0,
            priority: "High".to_string(),
            window_mode: "Windowed".to_string(),
            extra_flags: Vec::new(),
            disabled_default_flags: Vec::new(),
            menu_hotkey: "F1".to_string(),
            open_menu_on_startup: true,
            devtools: true,
        }
    }
}

impl Config {
    pub fn dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("wankle-client")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("config.json")
    }

    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_else(|_| Config::default()),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) {
        let dir = Self::dir();
        let _ = std::fs::create_dir_all(&dir);
        if let Ok(json) = serde_json::to_string_pretty(self) {
            // Atomic-ish write: temp then rename.
            let tmp = dir.join("config.json.tmp");
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, Self::path());
            }
        }
    }

    /// Resolve the URL to actually load.
    pub fn target_url(&self) -> String {
        if self.use_custom_url && !self.custom_url.trim().is_empty() {
            self.custom_url.trim().to_string()
        } else {
            self.site.clone()
        }
    }
}
