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
    #[structopt(long, default_value = "800")]
    steps: i64,

    #[structopt(long, default_value = "200")]
    accel: f64,

    #[structopt(long, default_value = "1000")]
    velocity_hz: f64,
}

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
//const GPIO_DIR: u8 = 16;

const PULSE_DURATION_US: u64 = 1;
const PULSE_DURATION: Duration = Duration::from_micros(PULSE_DURATION_US);
const MIN_T: f64 = 0.0001;

fn device(opt: Opt) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    //let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();

    let mut velocity_hz = 0.0;
    let mut t = (2.0 / opt.accel).sqrt();
    for i in (0..opt.steps).rev() {
        let max_delta_v = opt.accel * t;
        let delta_v = (opt.velocity_hz - velocity_hz).min(max_delta_v).max(-max_delta_v);
        let new_vel = velocity_hz + delta_v;
        let delta_v = if new_vel * new_vel / opt.accel < (i * 2) as f64 {
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
    let run = Arc::new(AtomicBool::new(true));
    let device_thread = thread::spawn(move || device(opt));

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
