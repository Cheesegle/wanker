//! Platform-gated "power" features. On Windows these talk to WebView2/Win32
//! directly; everywhere else they are graceful no-ops.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use other::*;
