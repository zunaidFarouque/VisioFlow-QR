use crate::error::Result;

/// Platform-agnostic system executor for rule actions (WiFi connect, etc.).
#[cfg_attr(test, mockall::automock)]
pub trait SystemExecutor: Send + Sync {
    fn connect_wifi(&self, ssid: &str, password: &str) -> Result<()>;
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "windows")]
pub use windows::PlatformExecutor;
#[cfg(target_os = "linux")]
pub use linux::PlatformExecutor;

/// Returns the platform-specific executor implementation.
#[must_use]
pub fn platform_executor() -> PlatformExecutor {
    PlatformExecutor
}
