use std::{process::ExitCode, sync::Arc, thread};

use anyhow::Result;
use clap::Parser;
use log::{debug, error, info};
use tokio::sync::mpsc;

mod device;
mod evloop;
mod joystick;

#[derive(Debug, Parser, Clone, Copy)]
#[command(author, version, about, long_about = None)]
pub struct Opt {
    #[arg(long, default_value = "20000")]
    max_accel: f64,

    #[arg(long, default_value = "100")]
    min_speed: f64,

    #[arg(long, default_value = "1000")]
    init_speed: f64,

    #[arg(long, default_value = "5000")]
    max_speed: f64,

    #[arg(long, default_value = "40")]
    min_stroke: i64,

    #[arg(long, default_value = "1340")]
    max_pos: i64,

    #[arg(long, default_value = "20.0")]
    time_to_max_s: f64,

    #[arg(long)]
    report_events: bool,
}

fn run_evloop(
    opt: Opt,
    ctrl: Arc<device::Control>,
    status: mpsc::UnboundedReceiver<device::StatusMessage>,
) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { evloop::main_loop(opt, ctrl.clone(), status).await })?;
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
    let ctrl = Arc::new(device::Control::new(opt.max_accel));
    for sig in signal_hook::consts::TERM_SIGNALS {
        signal_hook::flag::register_conditional_default(*sig, ctrl.stop.clone())?;
        signal_hook::flag::register(*sig, ctrl.stop.clone())?;
    }
    let (sender, receiver) = mpsc::unbounded_channel();
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device::device(ctrl, sender))
    };
    let evloop_result = run_evloop(opt, ctrl.clone(), receiver);
    debug!("Event loop finished");
    ctrl.stop.store(true, std::sync::atomic::Ordering::SeqCst);
    thread_result_unwrap(device_thread.join())?;
    evloop_result?;
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
