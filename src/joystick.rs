use std::{
    fs::OpenOptions,
    io::ErrorKind,
    os::unix::fs::OpenOptionsExt,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use anyhow::{anyhow, Result};
use evdev_rs::{enums::EV_ABS, InputEvent, TimeVal};
use tokio::{
    io::{unix::AsyncFd, Interest},
    time,
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
    event_code: evdev_rs::enums::EventCode,
    per: f64,
    flat: i32,
    driven: f64,
    drive: bool,
    last_time: evdev_rs::TimeVal,
    last_value: i32,
}

impl Axis {
    fn new(spec: AxisSpec, ev_device: &evdev_rs::Device, now: evdev_rs::TimeVal) -> Result<Axis> {
        let event_code = evdev_rs::enums::EventCode::EV_ABS(spec.abs);
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
            driven: 0.0,
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
        if event.event_code != evdev_rs::enums::EventCode::EV_ABS(self.spec.abs) {
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

#[derive(Debug)]
struct JoyState {
    opt: Opt,
    ctrl: Arc<device::Control>,
    pos: Axis,
    stroke_len: Axis,
    asymmetry: Axis,
    speed: Axis,
    drive: bool,
}

impl JoyState {
    fn new(
        opt: Opt,
        ctrl: Arc<device::Control>,
        ev_device: &evdev_rs::Device,
        now: TimeVal,
    ) -> Result<JoyState> {
        let mut res = JoyState {
            opt,
            ctrl,
            pos: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_X,
                    min: -opt.max_pos as f64,
                    max: opt.max_pos as f64,
                    time_to_max_s: 5.0,
                },
                &ev_device,
                now,
            )?,
            stroke_len: Axis::new(
                AxisSpec {
                    abs: EV_ABS::ABS_Y,
                    min: 0.0,
                    max: opt.max_pos as f64,
                    time_to_max_s: -5.0,
                },
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
                &ev_device,
                now,
            )?,
            drive: false,
        };
        res.speed.driven = opt.init_speed.ln();
        Ok(res)
    }

    fn handle_tick(&mut self, now: TimeVal) {
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
            let v = self.speed.driven.exp();
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

    fn handle_event(&mut self, event: InputEvent) {
        if event.event_code == evdev_rs::enums::EventCode::EV_KEY(evdev_rs::enums::EV_KEY::BTN_TR) {
            if event.value == 1 {
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
    }

    fn report(&self) {
        println!(
            "{:5} {:5} {:5} {:5} {}",
            self.pos.driven,
            self.stroke_len.driven,
            self.asymmetry.driven,
            self.speed.driven.exp(),
            self.drive
        );
    }
}

pub async fn main_loop(opt: Opt, ctrl: Arc<device::Control>) -> Result<()> {
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let mut joystate = JoyState::new(opt, ctrl.clone(), &ev_device, timeval::now()?)?;
    println!("{:?}", joystate);
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)?;
    let mut interval = time::interval(Duration::from_millis(50));
    let mut report = time::interval(Duration::from_secs(1));
    while ctrl.run.load(Ordering::Relaxed) {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;
                let a = afd.get_ref().next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        joystate.handle_event(k.1);
                        //println!("Event: {:?}", k.1);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        //println!("would block");
                        guard.clear_ready();
                    }
                    not_ok => {
                        println!("boom");
                        not_ok?;
                    }
                }
            }
            _ = interval.tick() => {
                joystate.handle_tick(timeval::now()?);
            }
            _ = report.tick() => {
                joystate.report();
            }
        }
    }
    println!("Finished joystick loop");
    Ok(())
}
