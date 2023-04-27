use std::{sync::Arc, time::SystemTime};

use anyhow::{anyhow, Result};
use evdev_rs::{enums, enums::EventCode, DeviceWrapper, InputEvent};
use log::{debug, info};

use crate::{device, Config};

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
    per: f64,
    flat: i32,
    driven: f64,
    last_time: SystemTime,
    last_value: i32,
}

impl Axis {
    fn new(
        spec: AxisSpec,
        driven: f64,
        ev_device: &evdev_rs::Device,
        now: SystemTime,
    ) -> Result<Axis> {
        let event_code = EventCode::EV_ABS(spec.abs);
        let abs_info = ev_device
            .abs_info(&event_code)
            .ok_or_else(|| anyhow!("abs_info failed"))?;
        let per = spec.max / (abs_info.maximum as f64 * spec.time_to_max_s);
        let flat = abs_info.flat * 11 / 10;
        Ok(Axis {
            spec,
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

    fn handle_tick(&mut self, drive: bool, now: SystemTime) {
        if drive {
            if let Ok(t) = now.duration_since(self.last_time) {
                self.driven += self.speed() * t.as_secs_f64();
                self.driven = self.driven.max(self.spec.min).min(self.spec.max);
            }
        }
        self.last_time = now;
    }

    fn handle_event(&mut self, drive: bool, event: &evdev_rs::InputEvent) {
        if event.event_code != EventCode::EV_ABS(self.spec.abs) {
            return;
        }
        self.handle_tick(drive, event.time.try_into().unwrap());
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

#[derive(Debug, PartialEq, Eq)]
enum TriggerLockState {
    Unlocked,
    LockedTriggerNonzero,
    LockedTriggerZero,
}

#[derive(Debug)]
pub struct JoyState {
    config: Config,
    ctrl: Arc<device::Control>,
    pos: Axis,
    stroke_len: Axis,
    asymmetry: Axis,
    speed: Axis,
    trigger_max: i32,
    trigger_ln: f64,
    trigger_lock: TriggerLockState,
    drive: bool,
    last_stop: i64,
    pos_offset: i64,
}

impl JoyState {
    pub fn new(
        config: Config,
        ctrl: Arc<device::Control>,
        ev_device: &evdev_rs::Device,
        now: SystemTime,
    ) -> Result<JoyState> {
        Ok(JoyState {
            config,
            ctrl,
            pos: Axis::new(
                AxisSpec {
                    abs: enums::EV_ABS::ABS_X,
                    min: 0.0,
                    max: config.max_pos as f64,
                    time_to_max_s: config.time_to_max_s,
                },
                0.0,
                ev_device,
                now,
            )?,
            stroke_len: Axis::new(
                AxisSpec {
                    abs: enums::EV_ABS::ABS_Y,
                    min: config.min_stroke as f64,
                    max: (config.max_pos as f64) / 2.0,
                    time_to_max_s: -config.time_to_max_s,
                },
                config.min_stroke as f64,
                ev_device,
                now,
            )?,
            asymmetry: Axis::new(
                AxisSpec {
                    abs: enums::EV_ABS::ABS_RX,
                    min: -0.8,
                    max: 0.8,
                    time_to_max_s: config.time_to_max_s,
                },
                0.0,
                ev_device,
                now,
            )?,
            speed: Axis::new(
                AxisSpec {
                    abs: enums::EV_ABS::ABS_RY,
                    min: config.min_speed.ln(),
                    max: config.max_speed.ln(),
                    time_to_max_s: -config.time_to_max_s,
                },
                config.init_speed.ln(),
                ev_device,
                now,
            )?,
            trigger_max: ev_device
                .abs_info(&TRIGGER_CODE)
                .ok_or_else(|| anyhow!("abs_info failed on trigger_max"))?
                .maximum,
            trigger_ln: 0.0,
            trigger_lock: TriggerLockState::Unlocked,
            drive: false,
            last_stop: 0,
            pos_offset: 0,
        })
    }

    pub fn handle_tick(&mut self, now: SystemTime) {
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
            //debug!("{:?} {}", ends, target_speed);
            self.ctrl.set_ends(&[
                self.pos_offset + ((self.pos.driven - self.stroke_len.driven) as i64).max(0),
                self.pos_offset
                    + ((self.pos.driven + self.stroke_len.driven) as i64).min(self.config.max_pos),
            ]);
            self.ctrl.set_target_speeds(&[
                (v / (1.0 + self.asymmetry.driven) - self.pos.speed()).min(self.config.max_speed),
                (v / (1.0 - self.asymmetry.driven) + self.pos.speed()).min(self.config.max_speed),
            ]);
        } else {
            self.stroke_len
                .clamp(0.0, self.pos.driven - self.pos.spec.min);
            self.stroke_len
                .clamp(0.0, self.pos.spec.max - self.pos.driven);
            self.ctrl
                .set_ends(&[self.pos_offset, self.pos_offset + self.config.max_pos]);
            self.ctrl
                .set_target_speeds(&[-self.pos.speed(), self.pos.speed()]);
        }
    }

    pub fn handle_event(&mut self, event: InputEvent) {
        if self.config.report_events {
            info!("Event: {:?}", event);
        }
        self.pos.handle_event(true, &event);
        self.stroke_len.handle_event(self.drive, &event);
        self.asymmetry.handle_event(self.drive, &event);
        self.speed.handle_event(self.drive, &event);
        match event.event_code {
            TRIGGER_CODE => {
                if event.value > 0 {
                    if self.trigger_lock != TriggerLockState::LockedTriggerNonzero {
                        self.trigger_lock = TriggerLockState::Unlocked;
                        self.trigger_ln = (((event.value as f64) / (self.trigger_max as f64))
                            - 1.0)
                            * TRIGGER_FACTOR_LN;
                        self.drive = true;
                    }
                } else if self.trigger_lock == TriggerLockState::Unlocked {
                    self.trigger_ln = -1.0;
                    self.drive = false;
                    self.ctrl.set_target_speeds(&[0.0, 0.0]);
                } else {
                    self.trigger_lock = TriggerLockState::LockedTriggerZero;
                }
            }
            ASYMMETRY_RESET_CODE => {
                if event.value != 0 {
                    self.asymmetry.driven = 0.0;
                }
            }
            EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_HAT0X) => match event.value {
                1 => {
                    let dp = self.last_stop - self.pos_offset;
                    self.pos_offset += dp;
                    self.pos.driven -= dp as f64;
                }
                -1 => {
                    let dp = self.config.max_pos + self.pos_offset - self.last_stop;
                    self.pos_offset -= dp;
                    self.pos.driven += dp as f64;
                }
                _ => (),
            },
            EventCode::EV_KEY(evdev_rs::enums::EV_KEY::BTN_TR) => {
                if event.value == 1 && self.trigger_ln != -1.0 {
                    self.trigger_lock = TriggerLockState::LockedTriggerNonzero;
                }
            }
            _ => (),
        }
    }

    pub fn handle_status(&mut self, status: device::StatusMessage) {
        debug!("{status:?}");
        self.last_stop = status.0;
    }

    pub fn report(&self) {
        debug!(
            "Joystick state: pos = {:8.2} stroke_len = {:8.2} asymmetry = {:8.2} speed = {:8.2} drive = {}",
            self.pos.driven,
            self.stroke_len.driven,
            self.asymmetry.driven,
            self.speed.driven.exp(),
            self.drive
        );
    }
}
