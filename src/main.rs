use std::fs::File;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use rppal::gpio::Gpio;
use simple_signal::{self, Signal};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "200")]
    max_accel: f64,

    #[structopt(long, default_value = "1000")]
    max_velocity: f64,
}

#[derive(Debug)]
struct Control {
    target_velocity: AtomicU64,
    accel: AtomicU64,
}

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
//const GPIO_DIR: u8 = 16;

const PULSE_DURATION_US: u64 = 1;
const PULSE_DURATION: Duration = Duration::from_micros(PULSE_DURATION_US);
const MIN_T: f64 = 0.0001;
const STEPS: u64 = 16000;
const CONTROL_FACTOR: f64 = 0.001;
const CONTROL_SLEEP: Duration = Duration::from_micros(100000);

fn device(ctrl: Arc<Control>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    //let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();

    let mut velocity_hz = 0.0;
    let accel = ctrl.accel.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
    let mut t = (2.0 / accel).sqrt();
    for i in (0..STEPS).rev() {
        let target_velocity = ctrl.target_velocity.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
        let accel = ctrl.accel.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
        let max_delta_v = accel * t;
        let delta_v = (target_velocity - velocity_hz)
            .min(max_delta_v)
            .max(-max_delta_v);
        let new_vel = velocity_hz + delta_v;
        let delta_v = if new_vel * new_vel / accel < (i * 2) as f64 {
            delta_v
        } else {
            -max_delta_v
        };
        velocity_hz += delta_v;
        if velocity_hz <= 1.0 {
            println!("{} {}", i, velocity_hz);
            break;
        }
        t = (1.0 + delta_v * t / 2.0) / velocity_hz;
        if t < MIN_T {
            // this should never happen
            t = MIN_T;
        }
        pul_pin.set_high();
        thread::sleep(PULSE_DURATION);
        pul_pin.set_low();
        thread::sleep(Duration::from_secs_f64(
            t - 0.000001 * (PULSE_DURATION_US as f64),
        ));
        //println!("{} {} {}", i, pulse_width, velocity_hz);
    }
    println!("Finished successfully");
    Ok(())
}

fn joystick(opt: Opt) -> Result<()> {
    let mut target_velocity = opt.max_velocity;
    let ctrl = Arc::new(Control {
        target_velocity: AtomicU64::new((opt.max_velocity / CONTROL_FACTOR) as u64),
        accel: AtomicU64::new((opt.max_accel / CONTROL_FACTOR) as u64),
    });
    let run = Arc::new(AtomicBool::new(true));
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let run = run.clone();
        move |_| {
            run.store(false, Ordering::Relaxed);
        }
    });
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device(ctrl))
    };

    let fd = File::open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_fd(fd)?;
    while run.load(Ordering::Relaxed) {
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
                        .store((target_velocity / CONTROL_FACTOR) as u64, Ordering::Relaxed);
                }
            }
            Err(e) => {
                println!("{:?}", e);
                thread::sleep(CONTROL_SLEEP);
            },
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
