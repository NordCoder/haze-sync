//! Environment variable names and defaults for server config primitives.

pub const HAZE_SYNC_LISTEN_ADDR: &str = "HAZE_SYNC_LISTEN_ADDR";
pub const HAZE_SYNC_DATABASE_URL: &str = "HAZE_SYNC_DATABASE_URL";
pub const HAZE_SYNC_OBJECT_STORE_PATH: &str = "HAZE_SYNC_OBJECT_STORE_PATH";
pub const HAZE_SYNC_WORKTREE_PATH: &str = "HAZE_SYNC_WORKTREE_PATH";
pub const HAZE_SYNC_WORKTREE_ADAPTER_MODE: &str = "HAZE_SYNC_WORKTREE_ADAPTER_MODE";
pub const HAZE_GDRIVE_ADAPTER_MODE: &str = "HAZE_GDRIVE_ADAPTER_MODE";
pub const HAZE_OBSIDIAN_ADAPTER_MODE: &str = "HAZE_OBSIDIAN_ADAPTER_MODE";

pub const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:8080";
pub const DEFAULT_OBJECT_STORE_PATH: &str = "./data/objects";
pub const DEFAULT_WORKTREE_PATH: &str = "./data/worktree";
pub const DEFAULT_ADAPTER_MODE: &str = "disabled";
