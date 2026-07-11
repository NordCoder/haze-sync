//! Server-owned composition boundary for the built-in Worktree runtime.
//!
//! SRV-P7A wires accepted configuration and mode contracts only. It deliberately
//! does not construct a fake watcher, fake cycle executor, or background task.
//! Enabled modes remain honestly unavailable until SRV-P7B supplies the real
//! Core/API/Storage-backed cycle executor.

use haze_sync_common::AdapterMode;
use haze_sync_worktree::WorktreeMode;
use std::fmt;
use std::path::PathBuf;

/// Result of mapping the shared adapter mode into the accepted Worktree mode set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ServerWorktreeModeMapping {
    /// The shared mode has a direct Worktree runtime equivalent.
    Supported(WorktreeMode),
    /// Worktree has no accepted dry-run runtime contract.
    UnsupportedDryRun,
}

/// Exhaustively map the shared adapter mode without inventing a DryRun meaning.
#[must_use]
pub(crate) const fn map_adapter_mode(mode: AdapterMode) -> ServerWorktreeModeMapping {
    match mode {
        AdapterMode::Disabled => ServerWorktreeModeMapping::Supported(WorktreeMode::Disabled),
        AdapterMode::ReadOnly => ServerWorktreeModeMapping::Supported(WorktreeMode::ReadOnly),
        AdapterMode::ImportOnly => ServerWorktreeModeMapping::Supported(WorktreeMode::ImportOnly),
        AdapterMode::ExportOnly => ServerWorktreeModeMapping::Supported(WorktreeMode::ExportOnly),
        AdapterMode::Bidirectional => {
            ServerWorktreeModeMapping::Supported(WorktreeMode::Bidirectional)
        }
        AdapterMode::DryRun => ServerWorktreeModeMapping::UnsupportedDryRun,
    }
}

/// Server-visible lifecycle for the not-yet-hosted Worktree runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ServerWorktreeLifecycle {
    /// Configuration is composed, but explicit startup has not happened.
    Created,
    /// Disabled mode started as a completely inert boundary.
    Disabled,
    /// Configuration requested runtime work that cannot yet be hosted honestly.
    Unavailable,
    /// Explicit shutdown completed; implicit restart is forbidden.
    Shutdown,
}

impl ServerWorktreeLifecycle {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Disabled => "disabled",
            Self::Unavailable => "unavailable",
            Self::Shutdown => "shutdown",
        }
    }
}

/// Safe reason an enabled Worktree runtime is unavailable in SRV-P7A.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ServerWorktreeUnavailableReason {
    /// Shared DryRun has no accepted Worktree runtime equivalent.
    UnsupportedDryRun,
    /// A real Core/API/Storage-backed cycle executor is not wired yet.
    CycleExecutorNotWired,
}

impl ServerWorktreeUnavailableReason {
    const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedDryRun => "unsupported_dry_run",
            Self::CycleExecutorNotWired => "cycle_executor_not_wired",
        }
    }
}

/// Safe status snapshot with no configured root or internal error details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ServerWorktreeStatus {
    configured_mode: AdapterMode,
    mapped_mode: Option<WorktreeMode>,
    lifecycle: ServerWorktreeLifecycle,
    unavailable_reason: Option<ServerWorktreeUnavailableReason>,
    root_configured: bool,
}

impl ServerWorktreeStatus {
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn configured_mode(self) -> AdapterMode {
        self.configured_mode
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn mapped_mode(self) -> Option<WorktreeMode> {
        self.mapped_mode
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn lifecycle(self) -> ServerWorktreeLifecycle {
        self.lifecycle
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn unavailable_reason(self) -> Option<ServerWorktreeUnavailableReason> {
        self.unavailable_reason
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) const fn root_configured(self) -> bool {
        self.root_configured
    }
}

impl fmt::Display for ServerWorktreeStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mapped_mode = self.mapped_mode.map(worktree_mode_name).unwrap_or("unsupported");
        let unavailable_reason = self
            .unavailable_reason
            .map(ServerWorktreeUnavailableReason::as_str)
            .unwrap_or("none");

        write!(
            formatter,
            "configured_mode={}, mapped_mode={mapped_mode}, lifecycle={}, unavailable_reason={unavailable_reason}, root_configured={}",
            self.configured_mode,
            self.lifecycle.as_str(),
            self.root_configured,
        )
    }
}

/// Explicit lifecycle misuse at the Server composition boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ServerWorktreeLifecycleError {
    /// Startup was already performed.
    AlreadyStarted,
    /// Shutdown completed and this boundary cannot restart implicitly.
    AlreadyShutdown,
    /// Shutdown was requested before explicit startup.
    NotStarted,
}

impl fmt::Display for ServerWorktreeLifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::AlreadyStarted => "worktree composition is already started",
            Self::AlreadyShutdown => "worktree composition is already shut down",
            Self::NotStarted => "worktree composition is not started",
        })
    }
}

impl std::error::Error for ServerWorktreeLifecycleError {}

/// Server-owned Worktree composition state.
///
/// The root remains private and is never included in `Debug` or status output.
/// No filesystem operation occurs in this SRV-P7A boundary.
pub(crate) struct ServerWorktreeRuntime {
    configured_mode: AdapterMode,
    mapping: ServerWorktreeModeMapping,
    root: PathBuf,
    lifecycle: ServerWorktreeLifecycle,
    unavailable_reason: Option<ServerWorktreeUnavailableReason>,
}

impl ServerWorktreeRuntime {
    /// Compose configuration without starting any runtime activity.
    #[must_use]
    pub(crate) fn new(configured_mode: AdapterMode, root: PathBuf) -> Self {
        Self {
            configured_mode,
            mapping: map_adapter_mode(configured_mode),
            root,
            lifecycle: ServerWorktreeLifecycle::Created,
            unavailable_reason: None,
        }
    }

    /// Explicitly start the boundary without fabricating missing runtime services.
    pub(crate) fn start(
        &mut self,
    ) -> Result<ServerWorktreeStatus, ServerWorktreeLifecycleError> {
        match self.lifecycle {
            ServerWorktreeLifecycle::Created => {}
            ServerWorktreeLifecycle::Shutdown => {
                return Err(ServerWorktreeLifecycleError::AlreadyShutdown);
            }
            ServerWorktreeLifecycle::Disabled | ServerWorktreeLifecycle::Unavailable => {
                return Err(ServerWorktreeLifecycleError::AlreadyStarted);
            }
        }

        match self.mapping {
            ServerWorktreeModeMapping::Supported(WorktreeMode::Disabled) => {
                self.lifecycle = ServerWorktreeLifecycle::Disabled;
                self.unavailable_reason = None;
            }
            ServerWorktreeModeMapping::Supported(
                WorktreeMode::ReadOnly
                | WorktreeMode::ImportOnly
                | WorktreeMode::ExportOnly
                | WorktreeMode::Bidirectional,
            ) => {
                self.lifecycle = ServerWorktreeLifecycle::Unavailable;
                self.unavailable_reason =
                    Some(ServerWorktreeUnavailableReason::CycleExecutorNotWired);
            }
            ServerWorktreeModeMapping::UnsupportedDryRun => {
                self.lifecycle = ServerWorktreeLifecycle::Unavailable;
                self.unavailable_reason = Some(ServerWorktreeUnavailableReason::UnsupportedDryRun);
            }
        }

        Ok(self.status())
    }

    /// Explicitly close the composition boundary and prohibit implicit restart.
    pub(crate) fn shutdown(
        &mut self,
    ) -> Result<ServerWorktreeStatus, ServerWorktreeLifecycleError> {
        match self.lifecycle {
            ServerWorktreeLifecycle::Created => {
                return Err(ServerWorktreeLifecycleError::NotStarted);
            }
            ServerWorktreeLifecycle::Shutdown => {
                return Err(ServerWorktreeLifecycleError::AlreadyShutdown);
            }
            ServerWorktreeLifecycle::Disabled | ServerWorktreeLifecycle::Unavailable => {}
        }

        self.lifecycle = ServerWorktreeLifecycle::Shutdown;
        Ok(self.status())
    }

    /// Return a path-redacted status snapshot.
    #[must_use]
    pub(crate) fn status(&self) -> ServerWorktreeStatus {
        ServerWorktreeStatus {
            configured_mode: self.configured_mode,
            mapped_mode: match self.mapping {
                ServerWorktreeModeMapping::Supported(mode) => Some(mode),
                ServerWorktreeModeMapping::UnsupportedDryRun => None,
            },
            lifecycle: self.lifecycle,
            unavailable_reason: self.unavailable_reason,
            root_configured: !self.root.as_os_str().is_empty(),
        }
    }
}

impl fmt::Debug for ServerWorktreeRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServerWorktreeRuntime")
            .field("status", &self.status())
            .finish()
    }
}

const fn worktree_mode_name(mode: WorktreeMode) -> &'static str {
    match mode {
        WorktreeMode::Disabled => "disabled",
        WorktreeMode::ReadOnly => "read_only",
        WorktreeMode::ImportOnly => "import_only",
        WorktreeMode::ExportOnly => "export_only",
        WorktreeMode::Bidirectional => "bidirectional",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn maps_every_shared_adapter_mode_exhaustively() {
        let cases = [
            (
                AdapterMode::Disabled,
                ServerWorktreeModeMapping::Supported(WorktreeMode::Disabled),
            ),
            (
                AdapterMode::ReadOnly,
                ServerWorktreeModeMapping::Supported(WorktreeMode::ReadOnly),
            ),
            (
                AdapterMode::ImportOnly,
                ServerWorktreeModeMapping::Supported(WorktreeMode::ImportOnly),
            ),
            (
                AdapterMode::ExportOnly,
                ServerWorktreeModeMapping::Supported(WorktreeMode::ExportOnly),
            ),
            (
                AdapterMode::Bidirectional,
                ServerWorktreeModeMapping::Supported(WorktreeMode::Bidirectional),
            ),
            (
                AdapterMode::DryRun,
                ServerWorktreeModeMapping::UnsupportedDryRun,
            ),
        ];

        for (configured, expected) in cases {
            assert_eq!(map_adapter_mode(configured), expected);
        }
    }

    #[test]
    fn construction_is_explicit_and_does_not_start_or_touch_the_filesystem() {
        let root = unique_missing_root("created");
        let runtime = ServerWorktreeRuntime::new(AdapterMode::Bidirectional, root.clone());

        assert_eq!(runtime.status().lifecycle(), ServerWorktreeLifecycle::Created);
        assert!(runtime.status().root_configured());
        assert!(!root.exists());
        let _ = crate::routes::build_router();
        assert!(!root.exists());
    }

    #[test]
    fn disabled_mode_is_completely_inert() {
        let root = unique_missing_root("disabled");
        let mut runtime = ServerWorktreeRuntime::new(AdapterMode::Disabled, root.clone());

        let status = runtime.start().unwrap();

        assert_eq!(status.configured_mode(), AdapterMode::Disabled);
        assert_eq!(status.mapped_mode(), Some(WorktreeMode::Disabled));
        assert_eq!(status.lifecycle(), ServerWorktreeLifecycle::Disabled);
        assert_eq!(status.unavailable_reason(), None);
        assert!(!root.exists());
    }

    #[test]
    fn enabled_modes_are_honestly_unavailable_without_a_real_executor() {
        for mode in [
            AdapterMode::ReadOnly,
            AdapterMode::ImportOnly,
            AdapterMode::ExportOnly,
            AdapterMode::Bidirectional,
        ] {
            let root = unique_missing_root(mode.as_str());
            let mut runtime = ServerWorktreeRuntime::new(mode, root.clone());
            let status = runtime.start().unwrap();

            assert_eq!(status.lifecycle(), ServerWorktreeLifecycle::Unavailable);
            assert_eq!(
                status.unavailable_reason(),
                Some(ServerWorktreeUnavailableReason::CycleExecutorNotWired)
            );
            assert!(!root.exists());
        }
    }

    #[test]
    fn dry_run_is_not_silently_mapped_to_another_worktree_mode() {
        let mut runtime =
            ServerWorktreeRuntime::new(AdapterMode::DryRun, PathBuf::from("./private-root"));

        let status = runtime.start().unwrap();

        assert_eq!(status.mapped_mode(), None);
        assert_eq!(status.lifecycle(), ServerWorktreeLifecycle::Unavailable);
        assert_eq!(
            status.unavailable_reason(),
            Some(ServerWorktreeUnavailableReason::UnsupportedDryRun)
        );
    }

    #[test]
    fn debug_and_status_do_not_expose_the_configured_root() {
        let secret_root = PathBuf::from("/srv/private/token-like-worktree-root");
        let runtime = ServerWorktreeRuntime::new(AdapterMode::Bidirectional, secret_root.clone());

        let debug = format!("{runtime:?}");
        let status_debug = format!("{:?}", runtime.status());
        let status_display = runtime.status().to_string();

        for output in [debug, status_debug, status_display] {
            assert!(!output.contains(secret_root.to_string_lossy().as_ref()));
            assert!(!output.contains("/srv/private"));
            assert!(!output.contains("token-like"));
        }
    }

    #[test]
    fn shutdown_is_explicit_and_restart_is_forbidden() {
        let mut runtime =
            ServerWorktreeRuntime::new(AdapterMode::Disabled, PathBuf::from("./unused"));

        assert_eq!(
            runtime.shutdown().unwrap_err(),
            ServerWorktreeLifecycleError::NotStarted
        );
        runtime.start().unwrap();
        assert_eq!(
            runtime.start().unwrap_err(),
            ServerWorktreeLifecycleError::AlreadyStarted
        );
        assert_eq!(
            runtime.shutdown().unwrap().lifecycle(),
            ServerWorktreeLifecycle::Shutdown
        );
        assert_eq!(
            runtime.start().unwrap_err(),
            ServerWorktreeLifecycleError::AlreadyShutdown
        );
    }

    fn unique_missing_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "haze-sync-server-worktree-{name}-{}-{nonce}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).expect("test root should be removable");
        }
        root
    }
}
