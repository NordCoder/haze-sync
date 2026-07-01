//! Haze Sync server route shell binary.
//!
//! This phase exposes deterministic HTTP route boundaries only. It does not
//! start a production listener, connect to storage, run migrations, verify
//! adapter tokens, or perform Core sync behavior.

pub mod config;
pub mod http;
pub mod routes;

fn main() {
    let _app = routes::build_router();
    println!("haze-sync-server route shell");
}
