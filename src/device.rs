use std::{sync::atomic::{AtomicBool, AtomicI64, Ordering}, time::Instant};
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
const POLL_SLEEP: Duration = Duration::from_micros(50000);
const MIN_DISTANCE: i64 = 2;
const MIN_VELOCITY: f64 = 1.0;
const MIN_T: f64 = 0.0001;

fn read_control(ct: &AtomicI64) -> f64 {
    ct.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR
}

pub fn device(ctrl: Arc<Control>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();
    let mut pos: i64 = 0;
    let mut dir: usize = 0;

    while ctrl.run.load(Ordering::Relaxed) {
        dir = 1 - dir;
        let dir_mul = (dir as i64) * 2 - 1;
        let end = ctrl.ends[dir].load(Ordering::Relaxed);
        let target_velocity = read_control(&ctrl.target_velocity);
        if target_velocity <= MIN_VELOCITY || (end - pos) * dir_mul <= MIN_DISTANCE {
            dir_pin.set_low();
            thread::sleep(POLL_SLEEP);
            continue;
        }
        dir_pin.write(if dir == 0 { Level::Low } else { Level::High });
        thread::sleep(DIR_SLEEP);
        let mut velocity_hz = 0.0;
        let accel = read_control(&ctrl.accel);
        let mut t = (2.0 / accel).sqrt();
        let start_pos = pos;
        let mut slept = 0.0;
        let start = Instant::now();
        loop {
            let end = ctrl.ends[dir].load(Ordering::Relaxed);
            let target_velocity = read_control(&ctrl.target_velocity);
            let accel = read_control(&ctrl.accel);
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
            if velocity_hz <= MIN_VELOCITY {
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
            slept += t;
            pos += dir_mul;
            //println!("{} {} {}", i, pulse_width, velocity_hz);
        }
        if slept > 0.3 {
            let elapsed = start.elapsed().as_secs_f64();
            println!("elapsed {} slept {} diff {} ratio {}",
                elapsed, slept, elapsed - slept, elapsed/slept);
            let ticks = (pos - start_pos) * dir_mul;
            println!("ticks {} diff per tick {}", ticks,
                (elapsed - slept)/(ticks as f64));
        }
    }
    println!("Finished successfully");
    Ok(())
}
