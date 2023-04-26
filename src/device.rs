use std::{
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{ensure, Result};
use log::debug;
use rppal::gpio::{Gpio, Level};
use tokio::sync::mpsc;

pub const CONTROL_FACTOR: f64 = 0.001;

// Written by joystick, read by device
#[derive(Debug)]
pub struct Control {
    // Arc within an Arc, for the signal_handler crate.
    pub stop: Arc<AtomicBool>,
    ends: [AtomicI64; 2],
    target_speeds: [AtomicI64; 2],
    accel: f64,
}

impl Control {
    pub fn new(accel: f64) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            ends: [AtomicI64::new(0), AtomicI64::new(0)],
            target_speeds: [AtomicI64::new(0), AtomicI64::new(0)],
            accel,
        }
    }

    pub fn stop(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn end(&self, i: usize) -> i64 {
        self.ends[i].load(Ordering::Relaxed)
    }

    pub fn set_ends(&self, ends: &[i64; 2]) {
        for (end, &value) in self.ends.iter().zip(ends.iter()) {
            end.store(value, Ordering::Relaxed);
        }
    }

    pub fn target_speed(&self, i: usize) -> f64 {
        self.target_speeds[i].load(Ordering::Relaxed) as f64 * CONTROL_FACTOR
    }

    pub fn set_target_speeds(&self, target_speeds: &[f64; 2]) {
        for (target_speed, &value) in self.target_speeds.iter().zip(target_speeds.iter()) {
            target_speed.store((value / CONTROL_FACTOR) as i64, Ordering::Relaxed);
        }
    }
}

#[derive(Debug)]
pub struct StatusMessage(pub i64);

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
const GPIO_DIR: u8 = 16;

const PULSE_DURATION_US: u64 = 1;
const PULSE_DURATION: Duration = Duration::from_micros(PULSE_DURATION_US);
const DIR_SLEEP: Duration = Duration::from_micros(1000);
const POLL_SLEEP: Duration = Duration::from_micros(50000);
const MIN_DISTANCE: i64 = 2;
const MIN_PULSE: f64 = 0.00005;
const MIN_SPEED: f64 = 1.0;

pub fn device(ctrl: Arc<Control>, status: mpsc::UnboundedSender<StatusMessage>) -> Result<()> {
    let pulse_table: Vec<_> = {
        let a = 2.0 / ctrl.accel;
        std::iter::once(1.0 / MIN_SPEED)
            .chain(
                (1..)
                    .map(|d| (a * (d as f64)).sqrt())
                    .scan(0.0, |state, t| {
                        // let dt = t - *state;
                        // https://stackoverflow.com/questions/32444817/numerically-stable-evaluation-of-sqrtxa-sqrtx
                        let dt = a / (t + *state);
                        *state = t;
                        Some(dt)
                    })
                    .skip_while(|&dt| dt >= 1.0 / MIN_SPEED)
                    .take_while(|&dt| dt >= MIN_PULSE),
            )
            .collect()
    };
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();
    let mut pos: i64 = 0;
    let mut dir: usize = 0;
    let mut time_error = 0.0002;

    while !ctrl.stop() {
        let target_speeds = (0..2)
            .map(|d| {
                let dir_mul = (d as i64) * 2 - 1;
                let target_speed = ctrl.target_speed(d);
                if target_speed >= MIN_SPEED && (ctrl.end(d) - pos) * dir_mul > MIN_DISTANCE {
                    Some(target_speed)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        //dbg!(&ctrl, min_speed, dir, pos, &can_go);
        let other_dir = 1 - dir;
        let mut pulse_len = if let Some(ts) = target_speeds[dir] {
            1.0 / ts
        } else if let Some(ts) = target_speeds[other_dir] {
            dir = other_dir;
            1.0 / ts
        } else {
            thread::sleep(POLL_SLEEP);
            continue;
        };
        let dir_mul = (dir as i64) * 2 - 1;
        dir_pin.write(if dir == 1 { Level::Low } else { Level::High });
        thread::sleep(DIR_SLEEP);

        let start_pos = pos;
        // TODO: use a binary search here
        let max_pulse_ix = pulse_table
            .iter()
            .position(|&dt| dt < time_error)
            .unwrap_or(pulse_table.len() - 1);
        ensure!(
            max_pulse_ix > 0,
            "time_error = {time_error}, pulse_table[0] = {}",
            pulse_table[0]
        );
        let mut pulse_ix: usize = 1;
        let mut slept = 0.0;
        let mut time_clip = false;
        let start = Instant::now();
        while pulse_ix > 0 {
            pulse_len = pulse_len
                .min(pulse_table[pulse_ix - 1])
                .max(pulse_table[pulse_ix]);
            pul_pin.set_high();
            thread::sleep(PULSE_DURATION);
            pul_pin.set_low();
            thread::sleep(Duration::from_secs_f64(pulse_len));
            slept += pulse_len;
            pos += dir_mul;
            let end = ctrl.end(dir);
            pulse_len = 1.0 / ctrl.target_speed(dir).max(0.1 * MIN_SPEED);
            if dir_mul * (end - pos) < pulse_ix.try_into().unwrap()
                || pulse_table[pulse_ix - 1] <= pulse_len
                || ctrl.stop()
            {
                pulse_ix -= 1;
            } else if pulse_table[pulse_ix] > pulse_len {
                if pulse_ix == max_pulse_ix {
                    time_clip = true;
                } else {
                    pulse_ix += 1
                }
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        status.send(StatusMessage(pos))?;
        let ticks = (pos - start_pos) * dir_mul;
        debug!("At stroke end: pos {:8.2} time_clip {}", pos, time_clip);
        if ticks > 50 {
            debug!(
                "elapsed {:8.2} slept {:8.2} diff {:8.2} ratio 1 + {:e}",
                elapsed,
                slept,
                (elapsed - slept),
                (elapsed / slept) - 1.0
            );
            time_error = (elapsed - slept) / (ticks as f64);
            debug!("ticks {} time error {:8.2}us", ticks, time_error * 1e6);
        }
    }
    debug!("Control loop finished successfully");
    Ok(())
}
