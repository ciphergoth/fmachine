use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use rppal::gpio::{Gpio, Level};

#[derive(Debug)]
pub struct Control {
    run: AtomicBool,
    ends: [AtomicI64; 2],
    target_speeds: [AtomicI64; 2],
    accel: AtomicI64,
}

impl Control {
    pub fn new(accel: f64) -> Self {
        Self {
            run: AtomicBool::new(true),
            ends: [AtomicI64::new(0), AtomicI64::new(0)],
            target_speeds: [AtomicI64::new(0), AtomicI64::new(0)],
            accel: AtomicI64::new((accel / CONTROL_FACTOR) as i64),
        }
    }

    pub fn run(&self) -> bool {
        self.run.load(Ordering::Relaxed)
    }

    pub fn accel(&self) -> f64 {
        self.accel.load(Ordering::Relaxed) as f64 * CONTROL_FACTOR
    }

    pub fn end(&self, i: usize) -> i64 {
        self.ends[i].load(Ordering::Relaxed)
    }

    pub fn set_ends(&self, ends: &[i64; 2]) {
        for i in 0..2 {
            self.ends[i].store(ends[i], Ordering::Relaxed);
        }
    }

    pub fn target_speed(&self, i: usize) -> f64 {
        self.target_speeds[i].load(Ordering::Relaxed) as f64 * CONTROL_FACTOR
    }

    pub fn set_target_speeds(&self, target_speeds: &[f64; 2]) {
        for i in 0..2 {
            self.target_speeds[i].store(
                (target_speeds[i] / CONTROL_FACTOR) as i64,
                Ordering::Relaxed,
            );
        }
    }

    pub fn stop(&self) {
        self.run.store(false, Ordering::Relaxed);
        self.set_target_speeds(&[0.0, 0.0]);
    }
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
const MIN_SPEED: f64 = 1.0;
const MIN_T: f64 = 0.0001;

pub fn device(ctrl: Arc<Control>) -> Result<()> {
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();
    let mut pos: i64 = 0;
    let mut dir: usize = 0;

    while ctrl.run() {
        let can_go = (0..2)
            .map(|d| {
                let dir_mul = (d as i64) * 2 - 1;
                ctrl.target_speed(d) > MIN_SPEED && (ctrl.end(d) - pos) * dir_mul > MIN_DISTANCE
            })
            .collect::<Vec<_>>();
        let other_dir = 1 - dir;
        if !can_go[dir] {
            if can_go[other_dir] {
                dir = other_dir;
            } else {
                dir_pin.set_low();
                thread::sleep(POLL_SLEEP);
                continue;
            }
        }
        let dir_mul = (dir as i64) * 2 - 1;
        dir_pin.write(if dir == 0 { Level::Low } else { Level::High });
        thread::sleep(DIR_SLEEP);
        let mut velocity_hz = 0.0;
        let accel = ctrl.accel();
        let mut t = (2.0 / accel).sqrt();
        let start_pos = pos;
        let mut slept = 0.0;
        let start = Instant::now();
        loop {
            let end = ctrl.end(dir);
            let target_speed = ctrl.target_speed(dir);
            let max_delta_v = accel * t;
            let delta_v = (target_speed - velocity_hz)
                .min(max_delta_v)
                .max(-max_delta_v);
            let new_vel = velocity_hz + delta_v;
            let delta_v = if new_vel * new_vel / accel < ((end - pos) * dir_mul * 2) as f64 {
                delta_v
            } else {
                -max_delta_v
            };
            velocity_hz += delta_v;
            if velocity_hz <= MIN_SPEED {
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
        println!(
            "At stroke end: pos {:8.2} velocity_hz {:8.2}",
            pos, velocity_hz
        );
        if slept > 0.3 {
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "elapsed {:8.2} slept {:8.2} diff {:8.2} ratio 1 + {:e}",
                elapsed,
                slept,
                (elapsed - slept),
                (elapsed / slept) - 1.0
            );
            let ticks = (pos - start_pos) * dir_mul;
            println!(
                "ticks {} diff per tick {:8.2}us",
                ticks,
                (elapsed - slept) * 1e6 / (ticks as f64)
            );
        }
    }
    println!("Finished successfully");
    Ok(())
}
