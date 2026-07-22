//! Haze Google Drive adapter binary.
//!
//! This phase validates explicit configuration and then exits through the
//! lifecycle skeleton. It does not start provider calls, Core/API calls, mapping
//! persistence, or a sync loop.

use haze_gdrive_adapter::{AdapterRuntime, RuntimeError};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("haze-gdrive-adapter failed safely: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), RuntimeError> {
    let mut runtime = AdapterRuntime::from_env()?;
    let status = runtime.start()?;

    println!("haze-gdrive-adapter started: {status}");
    println!("haze-gdrive-adapter provider/core calls are disabled in this phase");

    runtime.request_shutdown();
    runtime.stop();
    println!("haze-gdrive-adapter stopped");

    Ok(())
}
