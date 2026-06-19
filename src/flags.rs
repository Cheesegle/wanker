//! Builds the Chromium command-line string passed to WebView2 via
//! `WebViewBuilderExtWindows::with_additional_browser_args`.
//!
//! The base list is a curated, *generic* performance set adapted from glorp
//! (https://github.com/slavcp/glorp) — game-specific entries removed. These
//! switches only take effect on Windows (WebView2 is Chromium); on macOS
//! (WKWebView) and Linux (WebKitGTK) they are simply ignored.

use crate::config::Config;

/// Curated generic performance flags. Multi-value `--enable-features` /
/// `--disable-features` are consolidated into one switch each so Chromium
/// honours them reliably.
pub const DEFAULT_FLAGS: &[&str] = &[
    "--enable-features=BlinkCompositorUseDisplayThreadPriority,GpuUseDisplayThreadPriority,BrowserUseDisplayThreadPriority,JavaScriptExperimentalSharedMemory,WebAssemblyBaseline,WebAssemblyTiering,WebAssemblyLazyCompilation,V8VmFuture",
    "--disable-features=msSmartScreenProtection,CalculateNativeWinOcclusion,HappinessTrackingSurveysForDesktopDemo,site-isolation-trial-opt-out,MediaRouter,PerformanceInterventionUI,IPH_DemoMode",
    "--ignore-gpu-blocklist",
    "--enable-gpu-rasterization",
    "--enable-zero-copy",
    "--enable-native-gpu-memory-buffers",
    "--enable-threaded-compositing",
    "--disable-gpu-driver-bug-workarounds",
    "--disable-gpu-watchdog",
    "--disable-background-timer-throttling",
    "--disable-renderer-backgrounding",
    "--disable-backgrounding-occluded-windows",
    "--disable-low-end-device-mode",
    "--disable-hang-monitor",
    "--disable-component-update",
    "--enable-quic",
    "--no-pings",
    "--autoplay-policy=no-user-gesture-required",
];

/// Build the full additional-browser-args string from config.
pub fn build_browser_args(cfg: &Config) -> String {
    let mut flags: Vec<String> = DEFAULT_FLAGS
        .iter()
        .filter(|f| !cfg.disabled_default_flags.iter().any(|d| d == *f))
        .map(|s| s.to_string())
        .collect();

    // Rendering API select (ANGLE backend).
    match cfg.rendering_api.as_str() {
        "d3d11" => flags.push("--use-angle=d3d11".into()),
        "d3d9" => flags.push("--use-angle=d3d9".into()),
        "gl" => flags.push("--use-angle=gl".into()),
        "vulkan" => flags.push("--use-angle=vulkan".into()),
        "swiftshader" => flags.push("--use-angle=swiftshader".into()),
        _ => {} // "auto" -> let the engine decide
    }

    if cfg.fps_unlock {
        flags.push("--disable-frame-rate-limit".into());
        flags.push("--disable-gpu-vsync".into());
    }

    if !cfg.gpu_acceleration {
        flags.push("--disable-gpu".into());
    }

    for f in &cfg.extra_flags {
        let f = f.trim();
        if !f.is_empty() {
            flags.push(f.to_string());
        }
    }

    flags.join(" ")
}
