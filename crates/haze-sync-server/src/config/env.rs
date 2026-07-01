//! Environment variable names and defaults for server config primitives.

/// Server TCP listen address.
pub const HAZE_SYNC_LISTEN_ADDR: &str = "HAZE_SYNC_LISTEN_ADDR";
/// Required PostgreSQL database URL placeholder.
pub const HAZE_SYNC_DATABASE_URL: &str = "HAZE_SYNC_DATABASE_URL";
/// Content-addressed object store root path.
pub const HAZE_SYNC_OBJECT_STORE_PATH: &str = "HAZE_SYNC_OBJECT_STORE_PATH";
/// Built-in worktree materialized view root path.
pub const HAZE_SYNC_WORKTREE_PATH: &str = "HAZE_SYNC_WORKTREE_PATH";
/// Built-in worktree adapter mode.
pub const HAZE_SYNC_WORKTREE_ADAPTER_MODE: &str = "HAZE_SYNC_WORKTREE_ADAPTER_MODE";
/// Google Drive adapter rollout mode.
pub const HAZE_GDRIVE_ADAPTER_MODE: &str = "HAZE_GDRIVE_ADAPTER_MODE";
/// Obsidian plugin adapter rollout mode.
pub const HAZE_OBSIDIAN_ADAPTER_MODE: &str = "HAZE_OBSIDIAN_ADAPTER_MODE";

/// Safe local-development listen address default aligned with `.env.example`.
pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";
/// Safe local-development object store path default aligned with `.env.example`.
pub const DEFAULT_OBJECT_STORE_PATH: &str = "./data/objects";
/// Safe local-development worktree path default aligned with `.env.example`.
pub const DEFAULT_WORKTREE_PATH: &str = "./data/worktree";
/// Safe disabled adapter mode default for primitive-only config loading.
pub const DEFAULT_ADAPTER_MODE: &str = "disabled";
