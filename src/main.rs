use std::{process::ExitCode, sync::Arc, thread};

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, error, info};
use tokio::sync::mpsc;

mod device;
mod evloop;
mod joystick;

#[derive(Debug, Parser, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Opt {
    config_file: std::path::PathBuf,

    #[arg(long)]
    report_events: bool,
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct Config {
    max_accel: f64,
    min_speed: f64,
    init_speed: f64,
    max_speed: f64,
    min_stroke: i64,
    max_pos: i64,
    time_to_max_s: f64,
    report_events: bool,
}

fn run_evloop(
    config: Config,
    ctrl: Arc<device::Control>,
    status: mpsc::UnboundedReceiver<device::StatusMessage>,
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?
        .block_on(async { evloop::main_loop(config, ctrl.clone(), status).await })
        .context("in tokio runtime")?;
    Ok(())
}

fn thread_result_unwrap<T>(r: std::thread::Result<T>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => std::panic::resume_unwind(e),
    }
}

fn inner_main() -> Result<()> {
    let opt = Opt::parse();
    debug!("{:?}", opt);
    let mut config: Config =
        toml::from_str(&std::fs::read_to_string(&opt.config_file).context("reading config file")?)
            .context("parsing config file")?;
    config.report_events |= opt.report_events;
    let config = config;
    debug!("{:?}", config);
    let ctrl = Arc::new(device::Control::new(config.max_accel));
    for sig in signal_hook::consts::TERM_SIGNALS {
        signal_hook::flag::register_conditional_default(*sig, ctrl.stop.clone())?;
        signal_hook::flag::register(*sig, ctrl.stop.clone())?;
    }
    let (sender, receiver) = mpsc::unbounded_channel();
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device::device(ctrl, sender))
    };
    let evloop_result = run_evloop(config, ctrl.clone(), receiver);
    debug!("Event loop finished");
    ctrl.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    thread_result_unwrap(device_thread.join())?;
    evloop_result.context("in event loop")?;
    Ok(())
}

fn main() -> ExitCode {
    env_logger::init();
    match inner_main() {
        Ok(()) => {
            info!("Finished successfully");
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!("{e:?}");
            ExitCode::FAILURE
        }
    }
}
