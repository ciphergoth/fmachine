use std::cmp;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicI8, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

use rppal::gpio::{Gpio, Level};

// The simple-signal crate is used to handle incoming signals.
use simple_signal::{self, Signal};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "800")]
    steps: i64,

    #[structopt(long, default_value = "3")]
    period: f64,
}

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
const GPIO_DIR: u8 = 16;

#[derive(Debug)]
struct Fooble {
    run: AtomicBool,
    direction: AtomicI8,
    sleep_us: AtomicU64,
    offset_steps: AtomicI64,
}

const PULSE_DURATION_US: u64 = 1;
const SLEEP_MIN_US: u64 = 100;
const SLEEP_MAX_US: u64 = 5000;
const SLEEP_STEP_US: u64 = 3000;
const PULSE_DURATION: Duration = Duration::from_micros(PULSE_DURATION_US);

fn drive_motor(fooble: Arc<Fooble>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();

    let mut dir = 0;
    let mut sofar = 0;
    let mut last_pulse = 0;
    while fooble.run.load(Ordering::Relaxed) {
        let ndir = fooble.direction.load(Ordering::Relaxed);
        if dir != ndir {
            if ndir == -1 {
                dir_pin.set_high();
            } else if ndir == 1 {
                dir_pin.set_low();
            }
            dir = ndir;
        }
        let sleep = last_pulse + fooble.sleep_us.load(Ordering::Relaxed) - sofar;
        if sleep >= SLEEP_MAX_US {
            thread::sleep(Duration::from_micros(SLEEP_STEP_US));
            sofar += SLEEP_STEP_US;
        } else {
            thread::sleep(Duration::from_micros(
                cmp::max(SLEEP_MIN_US, sleep) - PULSE_DURATION_US,
            ));
            if dir == 0 {
                thread::sleep(PULSE_DURATION);
            } else {
                pul_pin.set_high();
                thread::sleep(PULSE_DURATION);
                pul_pin.set_low();
                fooble.offset_steps.fetch_add(dir as i64, Ordering::Relaxed);
            }
            sofar += sleep;
            last_pulse = sofar;
        }
    }
    pul_pin.set_low();
    dir_pin.set_low();
    Ok(())
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let fooble = Arc::new(Fooble {
        run: AtomicBool::new(true),
        direction: AtomicI8::new(1),
        sleep_us: AtomicU64::new(300000),
        offset_steps: AtomicI64::new(0),
    });
    drive_motor(fooble)?;

    Ok(())
}
