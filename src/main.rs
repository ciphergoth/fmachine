use std::fs::File;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicI8, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use evdev_rs::Device;
use rppal::gpio::Gpio;
//use simple_signal::{self, Signal};
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
const STEPS: u64 = 1600;
const CONTROL_FACTOR: f64 = 0.001;

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
        let delta_v = (target_velocity - velocity_hz).min(max_delta_v).max(-max_delta_v);
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
        if t < MIN_T { // this should never happen
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

fn joystick(run: Arc<AtomicBool>) -> Result<()> {
    let f = File::open("/dev/input/event0").unwrap();

    let mut d = Device::new().unwrap();
    d.set_fd(f).unwrap();
    while run.load(Ordering::Relaxed) {
        let a = d.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
        match a {
            Ok(k) => println!("Event: {:?}", k.1),
            Err(e) => println!("{:?}", e),
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let ctrl = Arc::new(Control {
        target_velocity: AtomicU64::new((opt.max_velocity / CONTROL_FACTOR) as u64),
        accel: AtomicU64::new((opt.max_accel / CONTROL_FACTOR) as u64)
    });
    let run = Arc::new(AtomicBool::new(true));
    let device_thread = {
        let ctrl = ctrl.clone();
        thread::spawn(move || device(ctrl))
    };

    let joystick_thread = {
        let run = run.clone();
        thread::spawn(move || joystick(run))
    };
    device_thread.join().unwrap()?;
    run.store(false, Ordering::Relaxed);
    joystick_thread.join().unwrap()?;
    println!("Finished successfully");
    Ok(())
}
