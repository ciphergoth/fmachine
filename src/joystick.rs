use std::sync::Arc;

use crate::{device, timeval, Opt};
use anyhow::{anyhow, Result};
use evdev_rs::{enums, enums::EventCode, InputEvent, TimeVal};

#[derive(Debug)]
struct AxisSpec {
    abs: enums::EV_ABS,
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
            last_time: now,
            last_value: 0,
        })
    }

    fn speed(&self) -> f64 {
        self.last_value as f64 * self.per
    }

    fn handle_tick(&mut self, drive: bool, now: TimeVal) {
        if drive {
            self.driven += self.speed() * timeval::diff_as_f64(&now, &self.last_time);
            self.driven = self.driven.max(self.spec.min).min(self.spec.max);
        }
        self.last_time = now;
    }

    fn handle_event(&mut self, drive: bool, event: &evdev_rs::InputEvent) {
        if event.event_code != EventCode::EV_ABS(self.spec.abs) {
            return;
        }
        self.handle_tick(drive, event.time);
        self.last_value = if event.value <= self.flat && event.value >= -self.flat {
            0
        } else {
            event.value
        };
    }

    fn clamp(&mut self, lo: f64, hi: f64) {
        self.driven = self
            .driven
            .max(lo)
            .min(hi)
            .max(self.spec.min)
            .min(self.spec.max);
    }
}

const TRIGGER_CODE: EventCode = EventCode::EV_ABS(enums::EV_ABS::ABS_RZ);
const TRIGGER_FACTOR_LN: f64 = 3.0;
const ASYMMETRY_RESET_CODE: EventCode = EventCode::EV_KEY(evdev_rs::enums::EV_KEY::BTN_THUMBR);

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
                    abs: enums::EV_ABS::ABS_X,
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
                    abs: enums::EV_ABS::ABS_Y,
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
                    abs: enums::EV_ABS::ABS_RX,
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
                    abs: enums::EV_ABS::ABS_RY,
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
        self.pos.handle_tick(true, now);
        self.stroke_len.handle_tick(self.drive, now);
        self.asymmetry.handle_tick(self.drive, now);
        self.speed.handle_tick(self.drive, now);
        if self.drive {
            // Triangular clamp on stroke length
            self.pos.clamp(
                self.pos.spec.min + self.stroke_len.driven,
                self.pos.spec.max - self.stroke_len.driven,
            );
            let v = (self.speed.driven + self.trigger_ln).exp();
            //println!("{:?} {}", ends, target_speed);
            self.ctrl.set_ends(&[
                ((self.pos.driven - self.stroke_len.driven) as i64).max(-self.opt.max_pos),
                ((self.pos.driven + self.stroke_len.driven) as i64).min(self.opt.max_pos),
            ]);
            self.ctrl.set_target_speeds(&[
                (v * (1.0 + self.asymmetry.driven).min(1.0) - self.pos.speed())
                    .min(self.opt.max_speed),
                (v * (1.0 - self.asymmetry.driven).min(1.0) + self.pos.speed())
                    .min(self.opt.max_speed),
            ]);
        } else {
            self.stroke_len
                .clamp(0.0, self.pos.driven - self.pos.spec.min);
            self.stroke_len
                .clamp(0.0, self.pos.spec.max - self.pos.driven);
            self.ctrl.set_ends(&[-self.opt.max_pos, self.opt.max_pos]);
            self.ctrl
                .set_target_speeds(&[-self.pos.speed(), self.pos.speed()]);
        }
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        if self.opt.report_events {
            println!("Event: {:?}", event);
        }
        self.pos.handle_event(true, &event);
        self.stroke_len.handle_event(self.drive, &event);
        self.asymmetry.handle_event(self.drive, &event);
        self.speed.handle_event(self.drive, &event);
        match event.event_code {
            TRIGGER_CODE => {
                if event.value > 0 {
                    self.trigger_ln = (((event.value as f64) / (self.trigger_max as f64)) - 1.0)
                        * TRIGGER_FACTOR_LN;
                    self.drive = true;
                } else {
                    self.drive = false;
                    self.ctrl.set_target_speeds(&[0.0, 0.0]);
                }
            }
            ASYMMETRY_RESET_CODE => {
                if event.value != 0 {
                    self.asymmetry.driven = 0.0;
                }
            }
            EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_HAT0X) => {
                self.ctrl.step_add(event.value as i64);
            }
            _ => (),
        }
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
