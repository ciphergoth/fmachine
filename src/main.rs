use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;

use anyhow::Result;
use simple_signal::{self, Signal};
use structopt::StructOpt;

mod device;
mod joystick;
mod timeval;

#[derive(Debug, StructOpt)]
pub struct Opt {
    #[structopt(long, default_value = "35000")]
    max_accel: f64,

    #[structopt(long, default_value = "100")]
    min_velocity: f64,

    #[structopt(long, default_value = "1000")]
    init_velocity: f64,

    #[structopt(long, default_value = "20000")]
    max_velocity: f64,

    #[structopt(long, default_value = "400")]
    max_pos: i64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let ctrl = Arc::new(device::Control {
        run: AtomicBool::new(true),
        ends: [AtomicI64::new(0), AtomicI64::new(0)],
        target_velocity: [AtomicI64::new(0), AtomicI64::new(0)],
        accel: AtomicI64::new((opt.max_accel / device::CONTROL_FACTOR) as i64),
    });
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let ctrl = ctrl.clone();
        move |_| {
            ctrl.run.store(false, Ordering::Relaxed);
        }
    });
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device::device(ctrl))
    };
    joystick::main_loop(opt, ctrl.clone())?;
    println!("Run is false, stopping");
    ctrl.target_velocity[0].store(0, Ordering::Relaxed);
    ctrl.target_velocity[1].store(0, Ordering::Relaxed);
    device_thread.join().unwrap()?;
    println!("Finished successfully");
    Ok(())
}
