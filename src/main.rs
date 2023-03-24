use std::{sync::Arc, thread};

use anyhow::Result;
use clap::Parser;
use simple_signal::{self, Signal};

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

    #[arg(long)]
    report_events: bool,
}

fn run_evloop(opt: Opt, ctrl: Arc<device::Control>) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { evloop::main_loop(opt, ctrl.clone()).await })?;
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::parse();
    println!("{:?}", opt);
    let ctrl = Arc::new(device::Control::new(opt.max_accel));
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let ctrl = ctrl.clone();
        move |_| {
            ctrl.stop();
        }
    });
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device::device(ctrl))
    };
    let evloop_result = run_evloop(opt, ctrl.clone());
    println!("Event loop finished");
    ctrl.stop();
    // unwrap() here, otherwise we see
    // the trait `std::error::Error` is not implemented for `dyn Any + Send`
    // `dyn Any + Send` cannot be shared between threads safely
    device_thread.join().unwrap()?;
    evloop_result?;
    println!("Finished successfully");
    Ok(())
}
