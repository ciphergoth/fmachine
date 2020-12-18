//use std::cmp;
//use std::sync::atomic::{AtomicBool, AtomicI64, AtomicI8, AtomicU64, Ordering};
//use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;

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

fn run(opt: Opt) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    //let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();

    let mut velocity_hz = 0.0;
    let mut pulse_width = (2.0/opt.accel).sqrt();
    for i in (0..opt.steps).rev() {
        let mut new_vel = velocity_hz + opt.accel * pulse_width;
        // Remember to fix this when we can change velocity, to avoid sudden deceleration
        if new_vel > opt.velocity_hz {
            new_vel = opt.velocity_hz;
        }
        velocity_hz = if new_vel * new_vel / opt.accel < (i*2) as f64 {
            new_vel
        } else {
            velocity_hz - opt.accel * pulse_width
        };
        if velocity_hz <= 1.0 {
            println!("{} {}", i, velocity_hz);
            break;
        }
        pulse_width = 1.0/velocity_hz;
        if pulse_width < 0.0001 {
            pulse_width = 0.0001;
        }
        pul_pin.set_high();
        thread::sleep(PULSE_DURATION);
        pul_pin.set_low();
        thread::sleep(Duration::from_secs_f64(pulse_width - 0.000001 * (PULSE_DURATION_US as f64)));
        //println!("{} {} {}", i, pulse_width, velocity_hz);
    }
    println!("Finished successfully");
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let handle: thread::JoinHandle<Result<()>> = thread::spawn(move || {run(opt)});
    handle.join().unwrap()?;
    println!("Finished successfully");
    Ok(())
}
