//! Haze Google Drive adapter process.
//!
//! The process now owns a cooperative long-running scheduler. Disabled mode is
//! an inert service state. Enabled modes fail closed until the concrete
//! provider/Core cycle composition is published; they never report a false
//! successful synchronization cycle.

use haze_gdrive_adapter::{
    AdapterRuntime, GuardedProductionCycleExecutor, RuntimeLoopOptions, ShutdownSignal,
    ThreadRuntimeSleeper,
};
use std::env;
use std::io;
use std::process::ExitCode;

const ENV_MAX_ITERATIONS: &str = "HAZE_GDRIVE_MAX_RUNTIME_ITERATIONS";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("haze-gdrive-adapter failed safely: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = AdapterRuntime::from_env()?;
    let status = runtime.start()?;
    let shutdown = ShutdownSignal::new();
    let signal_flag = shutdown.handler_flag();
    ctrlc::set_handler(move || {
        signal_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    })?;

    println!("haze-gdrive-adapter started: {status}");

    let mut executor = GuardedProductionCycleExecutor;
    let mut sleeper = ThreadRuntimeSleeper;
    let summary = runtime.run_loop(
        &mut executor,
        &mut sleeper,
        &shutdown,
        RuntimeLoopOptions {
            max_iterations: max_iterations_from_env()?,
            ..RuntimeLoopOptions::default()
        },
    )?;

    runtime.request_shutdown();
    runtime.stop();
    println!(
        "haze-gdrive-adapter stopped: successful_cycles={}, retryable_failures={}",
        summary.successful_cycles, summary.retryable_failures
    );
    Ok(())
}

fn max_iterations_from_env() -> Result<Option<u64>, Box<dyn std::error::Error>> {
    let Some(raw) = env::var_os(ENV_MAX_ITERATIONS) else {
        return Ok(None);
    };
    let value = raw
        .to_str()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "runtime iteration limit is not valid UTF-8",
            )
        })?
        .parse::<u64>()?;
    if value == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "runtime iteration limit must be greater than zero",
        )
        .into());
    }
    Ok(Some(value))
}
