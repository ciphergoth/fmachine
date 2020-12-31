use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
    thread,
};

use anyhow::Result;
use simple_signal::{self, Signal};
use structopt::StructOpt;

mod device;
mod joystick;
mod timeval;

#[derive(Debug, StructOpt, Clone, Copy)]
pub struct Opt {
    #[structopt(long, default_value = "35000")]
    max_accel: f64,

    #[structopt(long, default_value = "100")]
    min_speed: f64,

    #[structopt(long, default_value = "1000")]
    init_speed: f64,

    #[structopt(long, default_value = "20000")]
    max_speed: f64,

    #[structopt(long, default_value = "400")]
    max_pos: i64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let ctrl = Arc::new(device::Control {
        run: AtomicBool::new(true),
        ends: [AtomicI64::new(0), AtomicI64::new(0)],
        target_speed: [AtomicI64::new(0), AtomicI64::new(0)],
        accel: AtomicI64::new((opt.max_accel / device::CONTROL_FACTOR) as i64),
    });
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let ctrl = ctrl.clone();
        move |_| {
            ctrl.run.store(false, Ordering::Relaxed);
            ctrl.target_speed[0].store(0, Ordering::Relaxed);
            ctrl.target_speed[1].store(0, Ordering::Relaxed);
        }
    });
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device::device(ctrl))
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { joystick::main_loop(opt, ctrl.clone()).await })?;
    println!("Run is false, stopping");
    ctrl.target_speed[0].store(0, Ordering::Relaxed);
    ctrl.target_speed[1].store(0, Ordering::Relaxed);
    device_thread.join().unwrap()?;
    println!("Finished successfully");
    Ok(())
}
