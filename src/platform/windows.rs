//! Windows power features: CPU throttling via the WebView2 DevTools protocol
//! and process-priority control. Practices adapted from glorp.

use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2;
use windows::Win32::{
    Foundation::CloseHandle,
    System::{Diagnostics::ToolHelp::*, Threading::*},
};
use windows_core::{w, PCWSTR};
use wry::{WebView, WebViewExtWindows};

/// Throttle the renderer's CPU via `Emulation.setCPUThrottlingRate`.
/// `rate` is a slowdown multiplier: 1.0 = no throttle, 2.0 = half speed, etc.
pub fn set_cpu_throttling(webview: &WebView, rate: f32) {
    let core: ICoreWebView2 = webview.webview();
    let json = format!("{{\"rate\":{}}}", rate);
    // Keep the UTF-16 buffer alive for the duration of the synchronous call.
    let wide: Vec<u16> = json.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = core.CallDevToolsProtocolMethod(
            w!("Emulation.setCPUThrottlingRate"),
            PCWSTR(wide.as_ptr()),
            None,
        );
    }
}

/// Set the priority class of this process and every WebView2 child process.
pub fn set_priority(level: &str) {
    let class = match level {
        "High" => HIGH_PRIORITY_CLASS,
        "Above Normal" => ABOVE_NORMAL_PRIORITY_CLASS,
        "Below Normal" => BELOW_NORMAL_PRIORITY_CLASS,
        "Idle" => IDLE_PRIORITY_CLASS,
        _ => NORMAL_PRIORITY_CLASS,
    };

    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), class);

        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(s) => s,
            Err(_) => return,
        };

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile);
                if name.to_lowercase().contains("webview2") {
                    if let Ok(handle) = OpenProcess(PROCESS_SET_INFORMATION, false, entry.th32ProcessID)
                    {
                        let _ = SetPriorityClass(handle, class);
                        let _ = CloseHandle(handle);
                    }
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
}
