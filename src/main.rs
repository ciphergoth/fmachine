use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use simple_signal::{self, Signal};
use structopt::StructOpt;

mod device;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "200")]
    max_accel: f64,

    #[structopt(long, default_value = "1000")]
    max_velocity: f64,
}

const CONTROL_SLEEP: Duration = Duration::from_micros(100000);

fn joystick(opt: Opt) -> Result<()> {
    let mut target_velocity = opt.max_velocity;
    let ctrl = Arc::new(device::Control {
        run: AtomicBool::new(true),
        ends: [AtomicI64::new(400), AtomicI64::new(-400)],
        target_velocity: AtomicI64::new((opt.max_velocity / device::CONTROL_FACTOR) as i64),
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

    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_fd(fd)?;
    while ctrl.run.load(Ordering::Relaxed) {
        let a = ev_device.next_event(evdev_rs::ReadFlag::NORMAL);
        match a {
            Ok(k) => {
                println!("Event: {:?}", k.1);
                if let evdev_rs::enums::EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_RY) =
                    k.1.event_code
                {
                    target_velocity = (target_velocity - 0.01 * (k.1.value as f64))
                        .max(0.0)
                        .min(opt.max_velocity);
                    println!("val: {} target_velocity: {}", k.1.value, target_velocity);
                    ctrl.target_velocity
                        .store((target_velocity / device::CONTROL_FACTOR) as i64, Ordering::Relaxed);
                }
            }
            Err(e) => {
                println!("{:?}", e);
                thread::sleep(CONTROL_SLEEP);
            }
        }
    }
    println!("Run is false, stopping");
    ctrl.target_velocity.store(0, Ordering::Relaxed);
    device_thread.join().unwrap()?;
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    joystick(opt)?;
    println!("Finished successfully");
    Ok(())
}
