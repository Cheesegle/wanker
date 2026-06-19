//! Wankle Client — an optimized desktop wrapper for Wankle3D
//! (wankle.online / wanshot.lol) built on tauri-apps/wry.
//!
//! Design goals:
//!  - The game runs untouched in its own webview (no DOM injection).
//!  - All client controls live in a separate transparent overlay window,
//!    toggled with an OS-level hotkey (default F1).
//!  - Windows gets glorp-style power features (Chromium flag tuning, ANGLE
//!    rendering-API select, CPU throttle, process priority). macOS/Linux build
//!    and run with those features gracefully degraded.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod flags;
mod platform;

use config::Config;
use global_hotkey::{
    hotkey::{Code, HotKey},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use serde_json::Value;
use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Fullscreen, ResizeDirection, Window, WindowBuilder},
};
use wry::{WebView, WebViewBuilder};

/// Self-contained settings UI (HTML + inline CSS/JS).
const MENU_HTML: &str = include_str!("menu.html");

#[derive(Debug, Clone)]
enum UserEvent {
    ToggleMenu,
    Ipc(String),
}

fn main() {
    let mut cfg = Config::load();

    // Relaunch passes --play; otherwise honor the "show menu on startup" pref.
    let force_play = std::env::args().any(|a| a == "--play");
    let auto_play = force_play || !cfg.open_menu_on_startup;

    // Browser flags are fixed for the lifetime of a process (WebView2 only
    // honors one set per user-data-folder). Changing them triggers a relaunch.
    let startup_args = flags::build_browser_args(&cfg);
    let os = std::env::consts::OS; // "windows" | "macos" | "linux"

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // ---- OS-level hotkey to toggle the menu (no page injection needed) ----
    let hotkey_manager = GlobalHotKeyManager::new().ok();
    if let Some(mgr) = &hotkey_manager {
        let hk = HotKey::new(None, parse_hotkey(&cfg.menu_hotkey));
        let _ = mgr.register(hk);
        let hk_proxy = proxy.clone();
        std::thread::spawn(move || {
            let rx = GlobalHotKeyEvent::receiver();
            while let Ok(ev) = rx.recv() {
                if ev.state == HotKeyState::Pressed {
                    let _ = hk_proxy.send_event(UserEvent::ToggleMenu);
                }
            }
        });
    }

    // ---- Main game window ----
    let game_window = WindowBuilder::new()
        .with_title("Wankle3D")
        .with_inner_size(LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .expect("failed to create game window");

    // ---- Free-floating, draggable/resizable overlay window for the menu ----
    // Borderless + transparent so it reads as a panel over the game; move/resize
    // are driven from the webview via drag_window()/drag_resize_window().
    let mut overlay_builder = WindowBuilder::new()
        .with_title("Wankle Client — Menu")
        .with_decorations(false)
        .with_always_on_top(true)
        .with_resizable(true)
        .with_inner_size(LogicalSize::new(620.0, 780.0))
        .with_min_inner_size(LogicalSize::new(420.0, 460.0))
        .with_visible(!auto_play);
    #[cfg(windows)]
    {
        use tao::platform::windows::WindowBuilderExtWindows;
        overlay_builder = overlay_builder.with_skip_taskbar(true);
    }
    let overlay_window = overlay_builder
        .build(&event_loop)
        .expect("failed to create overlay window");
    center_overlay(&overlay_window, &game_window);

    // ---- Menu webview (transparent) inside the overlay window ----
    let ipc_proxy = proxy.clone();
    let menu_webview = {
        let mut b = WebViewBuilder::new()
            .with_html(MENU_HTML)
            .with_ipc_handler(move |req| {
                let _ = ipc_proxy.send_event(UserEvent::Ipc(req.body().to_string()));
            });
        b = apply_windows_args(b, &startup_args);
        b.build(&overlay_window).expect("failed to build menu webview")
    };

    // ---- State carried across event-loop iterations ----
    let mut game_webview: Option<WebView> = None;
    let mut menu_open = !auto_play;

    if auto_play {
        start_game(&game_window, &overlay_window, &menu_webview, &mut game_webview, &cfg);
        menu_open = false;
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        // Keep the hotkey manager alive for the whole run.
        let _keep_alive = &hotkey_manager;

        match event {
            Event::UserEvent(UserEvent::ToggleMenu) => {
                menu_open = !menu_open;
                overlay_window.set_visible(menu_open);
                let _ = menu_webview.set_visible(menu_open);
                if menu_open {
                    overlay_window.set_focus();
                    let _ = menu_webview.focus();
                    send_config(&menu_webview, &cfg, os, game_webview.is_some());
                } else if game_webview.is_some() {
                    // Return keyboard focus to the game so it responds
                    // immediately after the menu is dismissed.
                    game_window.set_focus();
                }
            }

            Event::UserEvent(UserEvent::Ipc(body)) => {
                let msg: Value = match serde_json::from_str(&body) {
                    Ok(v) => v,
                    Err(_) => return,
                };
                let kind = msg.get("type").and_then(Value::as_str).unwrap_or("");

                match kind {
                    // Menu finished loading -> push it the current config.
                    "ready" => send_config(&menu_webview, &cfg, os, game_webview.is_some()),

                    // Launch (first time) or apply-and-relaunch (already in game).
                    "play" => {
                        if let Some(c) = msg.get("config") {
                            if let Ok(nc) = serde_json::from_value::<Config>(c.clone()) {
                                cfg = nc;
                            }
                        }
                        cfg.save();
                        let new_args = flags::build_browser_args(&cfg);
                        if game_webview.is_none() && new_args == startup_args {
                            start_game(
                                &game_window,
                                &overlay_window,
                                &menu_webview,
                                &mut game_webview,
                                &cfg,
                            );
                            menu_open = false;
                        } else {
                            // Flags differ from this process's fixed set (or we're
                            // already in-game): relaunch for a clean WebView2 env.
                            relaunch();
                            *control_flow = ControlFlow::Exit;
                        }
                    }

                    // Live CPU throttle (Windows only; no-op elsewhere).
                    "throttle" => {
                        if let Some(rate) = msg.get("rate").and_then(Value::as_f64) {
                            cfg.cpu_throttle = rate as f32;
                            if let Some(gw) = &game_webview {
                                platform::set_cpu_throttling(gw, cfg.cpu_throttle);
                            }
                            cfg.save();
                        }
                    }

                    // Live process priority (Windows only; no-op elsewhere).
                    "priority" => {
                        if let Some(v) = msg.get("value").and_then(Value::as_str) {
                            cfg.priority = v.to_string();
                            platform::set_priority(v);
                            cfg.save();
                        }
                    }

                    "devtools" => {
                        if let Some(gw) = &game_webview {
                            gw.open_devtools();
                        }
                    }

                    // Hide the overlay and resume the game.
                    "close" => {
                        menu_open = false;
                        overlay_window.set_visible(false);
                        let _ = menu_webview.set_visible(false);
                        if game_webview.is_some() {
                            game_window.set_focus();
                        }
                    }

                    // Quit the entire client (drops both windows and exits).
                    "quit" => {
                        *control_flow = ControlFlow::Exit;
                    }

                    // Drag the menu window (mousedown on the title bar).
                    "drag" => {
                        let _ = overlay_window.drag_window();
                    }

                    // Resize the menu window (mousedown on an edge/corner grip).
                    "resize" => {
                        if let Some(dir) = msg.get("dir").and_then(Value::as_str) {
                            if let Some(d) = resize_dir(dir) {
                                let _ = overlay_window.drag_resize_window(d);
                            }
                        }
                    }

                    _ => {}
                }
            }

            Event::WindowEvent { window_id, event, .. } => {
                if window_id == game_window.id() {
                    if let WindowEvent::CloseRequested = event {
                        *control_flow = ControlFlow::Exit;
                    }
                } else if window_id == overlay_window.id() {
                    if let WindowEvent::CloseRequested = event {
                        // Don't kill the app from the overlay; just hide it.
                        menu_open = false;
                        overlay_window.set_visible(false);
                        let _ = menu_webview.set_visible(false);
                        if game_webview.is_some() {
                            game_window.set_focus();
                        }
                    }
                }
            }

            _ => {}
        }
    });
}

/// Create the game webview, apply live settings, and hide the overlay.
fn start_game(
    game_window: &Window,
    overlay_window: &Window,
    menu_webview: &WebView,
    game_webview: &mut Option<WebView>,
    cfg: &Config,
) {
    apply_window_mode(game_window, &cfg.window_mode);

    let mut b = WebViewBuilder::new()
        .with_url(cfg.target_url())
        .with_background_color((0, 0, 0, 255))
        .with_devtools(cfg.devtools);
    b = apply_windows_args(b, &flags::build_browser_args(cfg));
    let gw = b.build(game_window).expect("failed to build game webview");

    platform::set_priority(&cfg.priority);
    if (cfg.cpu_throttle - 1.0).abs() > f32::EPSILON {
        platform::set_cpu_throttling(&gw, cfg.cpu_throttle);
    }

    *game_webview = Some(gw);

    overlay_window.set_visible(false);
    let _ = menu_webview.set_visible(false);
    game_window.set_focus();
}

/// Apply Windows-only Chromium browser args. No-op on other platforms.
fn apply_windows_args<'a>(builder: WebViewBuilder<'a>, args: &str) -> WebViewBuilder<'a> {
    #[cfg(windows)]
    {
        use wry::WebViewBuilderExtWindows;
        return builder
            .with_additional_browser_args(args.to_string())
            .with_browser_accelerator_keys(false);
    }
    #[cfg(not(windows))]
    {
        let _ = args;
        builder
    }
}

/// Push the current config into the menu webview.
fn send_config(menu: &WebView, cfg: &Config, os: &str, in_game: bool) {
    if let Ok(json) = serde_json::to_string(cfg) {
        let os_json = serde_json::to_string(os).unwrap_or_else(|_| "\"\"".into());
        let script = format!(
            "window.applyConfig && window.applyConfig({}, {}, {});",
            json, os_json, in_game
        );
        let _ = menu.evaluate_script(&script);
    }
}

/// Center the floating menu window over the game window (initial placement only;
/// the user is free to drag it elsewhere afterwards).
fn center_overlay(overlay: &Window, game: &Window) {
    let gpos = game.outer_position().unwrap_or(PhysicalPosition::new(0, 0));
    let gsize = game.outer_size();
    let osize = overlay.outer_size();
    let x = gpos.x + (gsize.width as i32 - osize.width as i32) / 2;
    let y = gpos.y + (gsize.height as i32 - osize.height as i32) / 2;
    overlay.set_outer_position(PhysicalPosition::new(x.max(0), y.max(0)));
}

fn resize_dir(dir: &str) -> Option<ResizeDirection> {
    Some(match dir {
        "n" => ResizeDirection::North,
        "s" => ResizeDirection::South,
        "e" => ResizeDirection::East,
        "w" => ResizeDirection::West,
        "ne" => ResizeDirection::NorthEast,
        "nw" => ResizeDirection::NorthWest,
        "se" => ResizeDirection::SouthEast,
        "sw" => ResizeDirection::SouthWest,
        _ => return None,
    })
}

fn apply_window_mode(window: &Window, mode: &str) {
    match mode {
        "Maximized" => window.set_maximized(true),
        "Borderless Fullscreen" => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
        _ => {} // "Windowed"
    }
}

fn parse_hotkey(s: &str) -> Code {
    match s.to_uppercase().as_str() {
        "F1" => Code::F1,
        "F2" => Code::F2,
        "INSERT" => Code::Insert,
        "`" | "BACKTICK" | "BACKQUOTE" => Code::Backquote,
        _ => Code::F1,
    }
}

/// Relaunch a fresh instance (with --play) and let this one exit. Used to apply
/// Chromium-flag changes that need a clean WebView2 environment.
fn relaunch() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--play").spawn();
    }
}
