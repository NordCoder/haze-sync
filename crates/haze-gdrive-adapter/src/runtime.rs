//! Runtime lifecycle skeleton for the Google Drive adapter.
//!
//! This phase validates configuration and exposes lifecycle hooks only. It does
//! not call Google Drive, Haze Sync Core/API, or any persistence boundary.

use crate::config::{AdapterConfig, AdapterMode};
use crate::error::RuntimeError;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Created,
    Running,
    ShutdownRequested,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupStatus {
    pub mode: AdapterMode,
    pub poll_interval_seconds: u64,
    pub full_scan_interval_seconds: u64,
    pub max_deletes_per_run: u32,
    pub max_delete_ratio_percent: u8,
}

impl StartupStatus {
    pub fn from_config(config: &AdapterConfig) -> Self {
        Self {
            mode: config.mode,
            poll_interval_seconds: config.intervals.poll_interval_seconds(),
            full_scan_interval_seconds: config.intervals.full_scan_interval_seconds(),
            max_deletes_per_run: config.delete_safety.max_deletes_per_run,
            max_delete_ratio_percent: config.delete_safety.max_delete_ratio_percent,
        }
    }

    pub const fn is_dry_run(&self) -> bool {
        self.mode.is_dry_run_mode()
    }
}

impl fmt::Display for StartupStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "mode={}, dry_run={}, poll_interval_seconds={}, full_scan_interval_seconds={}, max_deletes_per_run={}, max_delete_ratio_percent={}",
            self.mode,
            self.is_dry_run(),
            self.poll_interval_seconds,
            self.full_scan_interval_seconds,
            self.max_deletes_per_run,
            self.max_delete_ratio_percent
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterRuntime {
    config: AdapterConfig,
    state: RuntimeState,
}

impl AdapterRuntime {
    pub fn from_env() -> Result<Self, RuntimeError> {
        let config = AdapterConfig::load_from_env()?;
        Ok(Self::new(config))
    }

    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            state: RuntimeState::Created,
        }
    }

    pub fn config(&self) -> &AdapterConfig {
        &self.config
    }

    pub fn state(&self) -> RuntimeState {
        self.state
    }

    pub fn start(&mut self) -> Result<StartupStatus, RuntimeError> {
        self.state = RuntimeState::Running;
        Ok(StartupStatus::from_config(&self.config))
    }

    pub fn request_shutdown(&mut self) {
        if self.state == RuntimeState::Running {
            self.state = RuntimeState::ShutdownRequested;
        }
    }

    pub fn stop(&mut self) {
        self.state = RuntimeState::Stopped;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeleteSafetyConfig, RuntimeIntervals, SecretPath, SecretString};
    use std::time::Duration;

    fn config() -> AdapterConfig {
        AdapterConfig {
            server_url: "https://sync.example.test".to_owned(),
            adapter_token: SecretString::from_raw("test", "adapter-token").expect("secret"),
            drive_root_folder_id: "driveRoot123".to_owned(),
            oauth_token_path: SecretPath::from_raw("test", "/run/secrets/oauth.json")
                .expect("secret path"),
            mode: AdapterMode::ImportOnly,
            intervals: RuntimeIntervals::new(Duration::from_secs(30), Duration::from_secs(300))
                .expect("intervals"),
            delete_safety: DeleteSafetyConfig::new(5, 20).expect("delete safety"),
        }
    }

    #[test]
    fn runtime_start_returns_safe_status() {
        let mut runtime = AdapterRuntime::new(config());

        let status = runtime.start().expect("runtime should start");

        assert_eq!(runtime.state(), RuntimeState::Running);
        assert_eq!(status.mode, AdapterMode::ImportOnly);
        assert!(!status.is_dry_run());
        assert_eq!(status.poll_interval_seconds, 30);
        assert_eq!(status.full_scan_interval_seconds, 300);
        assert_eq!(status.max_deletes_per_run, 5);
        assert_eq!(status.max_delete_ratio_percent, 20);
    }

    #[test]
    fn startup_status_derives_dry_run_only_from_mode() {
        let mut config = config();
        let import_status = StartupStatus::from_config(&config);
        assert!(!import_status.is_dry_run());
        assert!(import_status.to_string().contains("dry_run=false"));

        config.mode = AdapterMode::DryRun;
        let dry_run_status = StartupStatus::from_config(&config);
        assert!(dry_run_status.is_dry_run());
        assert!(dry_run_status.to_string().contains("dry_run=true"));
    }

    #[test]
    fn runtime_shutdown_is_explicit() {
        let mut runtime = AdapterRuntime::new(config());
        runtime.start().expect("runtime should start");
        runtime.request_shutdown();
        assert_eq!(runtime.state(), RuntimeState::ShutdownRequested);

        runtime.stop();
        assert_eq!(runtime.state(), RuntimeState::Stopped);
    }

    #[test]
    fn startup_status_display_has_no_secret_material() {
        let mut runtime = AdapterRuntime::new(config());
        let status = runtime.start().expect("runtime should start");
        let rendered = status.to_string();

        assert!(!rendered.contains("adapter-token"));
        assert!(!rendered.contains("/run/secrets"));
        assert!(rendered.contains("mode=import_only"));
        assert!(rendered.contains("dry_run=false"));
    }
}
