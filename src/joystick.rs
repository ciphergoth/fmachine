use std::sync::{atomic::Ordering, Arc};

use anyhow::{anyhow, Result};
use evdev_rs::{
    enums::{EventCode, EV_ABS},
    InputEvent, TimeVal,
};

use crate::{device, timeval, Opt};

#[derive(Debug)]
struct AxisSpec {
    abs: EV_ABS,
    min: f64,
    max: f64,
    time_to_max_s: f64,
}

#[derive(Debug)]
struct Axis {
    spec: AxisSpec,
    event_code: EventCode,
    per: f64,
    flat: i32,
    driven: f64,
    drive: bool,
    last_time: evdev_rs::TimeVal,
    last_value: i32,
}

impl Axis {
    fn new(
        spec: AxisSpec,
        driven: f64,
        ev_device: &evdev_rs::Device,
        now: evdev_rs::TimeVal,
    ) -> Result<Axis> {
        let event_code = EventCode::EV_ABS(spec.abs);
        let abs_info = ev_device
            .abs_info(&event_code)
            .ok_or_else(|| anyhow!("wtf"))?;
        let per = spec.max / (abs_info.maximum as f64 * spec.time_to_max_s);
        let flat = abs_info.flat * 11 / 10;
        Ok(Axis {
            spec,
            event_code,
            per,
            flat,
            driven,
            drive: false,
            last_time: now,
            last_value: 0,
        })
    }

    fn handle_tick(&mut self, now: TimeVal) {
        if self.drive {
            self.driven +=
                self.last_value as f64 * self.per * timeval::diff_as_f64(&now, &self.last_time);
            self.driven = self.driven.max(self.spec.min).min(self.spec.max);
        }
        self.last_time = now;
    }

    fn handle_event(&mut self, event: &evdev_rs::InputEvent) {
        if event.event_code != EventCode::EV_ABS(self.spec.abs) {
            return;
        }
        self.handle_tick(event.time);
        self.last_value = if event.value <= self.flat && event.value >= -self.flat {
            0
        } else {
            event.value
        };
    }
}

const TRIGGER_CODE: EventCode = EventCode::EV_ABS(EV_ABS::ABS_RZ);
const TRIGGER_FACTOR_LN: f64 = 3.0;

#[derive(Debug)]
pub struct JoyState {
    opt: Opt,
    ctrl: Arc<device::Control>,
    pos: Axis,
    stroke_len: Axis,
    asymmetry: Axis,
    speed: Axis,
    trigger_max: i32,
    trigger_ln: f64,
    drive: bool,
}

impl JoyState {
    pub fn new(
        opt: Opt,
        ctrl: Arc<device::Control>,
        ev_device: &evdev_rs::Device,
        now: TimeVal,
    ) -> Result<JoyState> {
        Ok(JoyState {
            opt,
            ctrl,
            pos: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_X,
                    min: -(opt.max_pos - opt.min_stroke) as f64,
                    max: (opt.max_pos - opt.min_stroke) as f64,
                    time_to_max_s: 5.0,
                },
                0.0,
                &ev_device,
                now,
            )?,
            stroke_len: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_Y,
                    min: opt.min_stroke as f64,
                    max: opt.max_pos as f64,
                    time_to_max_s: -5.0,
                },
                opt.min_stroke as f64,
                &ev_device,
                now,
            )?,
            asymmetry: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_RX,
                    min: -0.5,
                    max: 0.5,
                    time_to_max_s: 5.0,
                },
                0.0,
                &ev_device,
                now,
            )?,
            speed: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_RY,
                    min: opt.min_speed.ln(),
                    max: opt.max_speed.ln(),
                    time_to_max_s: -5.0,
                },
                opt.init_speed.ln(),
                &ev_device,
                now,
            )?,
            trigger_max: ev_device
                .abs_info(&TRIGGER_CODE)
                .ok_or_else(|| anyhow!("wtf"))?
                .maximum,
            trigger_ln: 0.0,
            drive: false,
        })
    }

    pub fn handle_tick(&mut self, now: TimeVal) {
        for ax in [
            &mut self.pos,
            &mut self.stroke_len,
            &mut self.asymmetry,
            &mut self.speed,
        ]
        .iter_mut()
        {
            ax.handle_tick(now);
        }
        // Triangular clamp on stroke length
        self.pos.driven = self
            .pos
            .driven
            .max(self.pos.spec.min + self.stroke_len.driven)
            .min(self.pos.spec.max - self.stroke_len.driven);
        if self.drive {
            let ends = [
                ((self.pos.driven - self.stroke_len.driven) as i64).max(-self.opt.max_pos),
                ((self.pos.driven + self.stroke_len.driven) as i64).min(self.opt.max_pos),
            ];
            let v = (self.speed.driven + self.trigger_ln).exp();
            let target_speed0 =
                (v * (1.0 + self.asymmetry.driven).min(1.0)).min(self.opt.max_speed);
            let target_speed1 =
                (v * (1.0 - self.asymmetry.driven).min(1.0)).min(self.opt.max_speed);
            //println!("{:?} {}", ends, target_speed);
            self.ctrl.ends[0].store(ends[0], Ordering::Relaxed);
            self.ctrl.ends[1].store(ends[1], Ordering::Relaxed);
            self.ctrl.target_speed[0].store(
                (target_speed0 / device::CONTROL_FACTOR) as i64,
                Ordering::Relaxed,
            );
            self.ctrl.target_speed[1].store(
                (target_speed1 / device::CONTROL_FACTOR) as i64,
                Ordering::Relaxed,
            );
        }
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        if event.event_code == TRIGGER_CODE {
            if event.value > 0 {
                self.trigger_ln =
                    (((event.value as f64) / (self.trigger_max as f64)) - 1.0) * TRIGGER_FACTOR_LN;
                self.drive = true;
            } else {
                self.drive = false;
                self.ctrl.target_speed[0].store(0, Ordering::Relaxed);
                self.ctrl.target_speed[1].store(0, Ordering::Relaxed);
            }
            for ax in [
                &mut self.pos,
                &mut self.stroke_len,
                &mut self.asymmetry,
                &mut self.speed,
            ]
            .iter_mut()
            {
                ax.handle_tick(event.time);
                ax.drive = self.drive;
            }
        } else {
            for ax in [
                &mut self.pos,
                &mut self.stroke_len,
                &mut self.asymmetry,
                &mut self.speed,
            ]
            .iter_mut()
            {
                ax.handle_event(&event);
            }
        }
        //println!("{:?}", event);
    }

    pub fn report(&self) {
        println!(
            "Joystick state: {:8.2} {:8.2} {:8.2} {:8.2} {}",
            self.pos.driven,
            self.stroke_len.driven,
            self.asymmetry.driven,
            self.speed.driven.exp(),
            self.drive
        );
    }
}
