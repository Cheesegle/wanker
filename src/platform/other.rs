//! macOS / Linux stubs. WKWebView and WebKitGTK don't expose the Chromium
//! DevTools protocol or a tunable process model, so these are no-ops. The menu
//! greys out the corresponding controls on non-Windows platforms.

use wry::WebView;

pub fn set_cpu_throttling(_webview: &WebView, _rate: f32) {}

pub fn set_priority(_level: &str) {}
