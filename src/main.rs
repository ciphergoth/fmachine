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
    #[structopt(long, default_value = "200")]
    max_accel: f64,

    #[structopt(long, default_value = "1000")]
    max_velocity: f64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let ctrl = Arc::new(device::Control {
        run: AtomicBool::new(true),
        ends: [AtomicI64::new(400), AtomicI64::new(-400)],
        target_velocity: AtomicI64::new(0),
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
    ctrl.target_velocity.store(0, Ordering::Relaxed);
    device_thread.join().unwrap()?;
    println!("Finished successfully");
    Ok(())
}
