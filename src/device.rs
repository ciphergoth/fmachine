use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use rppal::gpio::{Gpio, Level};

#[derive(Debug)]
pub struct Control {
    pub run: AtomicBool,
    pub ends: [AtomicI64; 2],
    pub target_velocity: AtomicI64,
    pub accel: AtomicI64,
}

pub const CONTROL_FACTOR: f64 = 0.001;

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
const GPIO_DIR: u8 = 16;

const PULSE_DURATION_US: u64 = 1;
const PULSE_DURATION: Duration = Duration::from_micros(PULSE_DURATION_US);
const DIR_SLEEP: Duration = Duration::from_micros(1000);
const MIN_T: f64 = 0.0001;

pub fn device(ctrl: Arc<Control>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();
    let mut pos: i64 = 0;
    let mut dir: usize = 0;

    while ctrl.run.load(Ordering::Relaxed) {
        dir_pin.write(if dir == 0 { Level::Low } else { Level::High });
        let dir_mul = (dir as i64) * -2 + 1;
        thread::sleep(DIR_SLEEP);
        let mut velocity_hz = 0.0;
        let accel = ctrl.accel.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
        let mut t = (2.0 / accel).sqrt();
        loop {
            let end = ctrl.ends[dir].load(Ordering::Relaxed);
            let target_velocity =
                ctrl.target_velocity.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
            let accel = ctrl.accel.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR;
            let max_delta_v = accel * t;
            let delta_v = (target_velocity - velocity_hz)
                .min(max_delta_v)
                .max(-max_delta_v);
            let new_vel = velocity_hz + delta_v;
            let delta_v = if new_vel * new_vel / accel < ((end - pos) * dir_mul * 2) as f64 {
                delta_v
            } else {
                -max_delta_v
            };
            velocity_hz += delta_v;
            if velocity_hz <= 1.0 {
                println!("{} {}", pos, velocity_hz);
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
            pos += dir_mul;
            //println!("{} {} {}", i, pulse_width, velocity_hz);
        }
        dir = 1 - dir;
    }
    println!("Finished successfully");
    Ok(())
}

