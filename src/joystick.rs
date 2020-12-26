use std::{
    fs::OpenOptions,
    io::ErrorKind,
    os::unix::fs::OpenOptionsExt,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use anyhow::{anyhow, Result};
use evdev_rs::enums::EV_ABS;
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

    fn handle_tick(&mut self, now: evdev_rs::TimeVal) {
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

#[tokio::main(flavor = "current_thread")]
pub async fn main_loop(opt: Opt, ctrl: Arc<device::Control>) -> Result<()> {
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let now = timeval::now()?;
    let mut pos_axis = Axis::new(
        AxisSpec {
            abs: EV_ABS::ABS_X,
            min: -opt.max_pos as f64,
            max: opt.max_pos as f64,
            time_to_max_s: 5.0,
        },
        &ev_device,
        now,
    )?;
    let mut stroke_len_axis = Axis::new(
        AxisSpec {
            abs: EV_ABS::ABS_Y,
            min: 0.0,
            max: opt.max_pos as f64,
            time_to_max_s: -5.0,
        },
        &ev_device,
        now,
    )?;
    let mut asymmetry_axis = Axis::new(
        AxisSpec {
            abs: EV_ABS::ABS_RX,
            min: -0.5,
            max: 0.5,
            time_to_max_s: 5.0,
        },
        &ev_device,
        now,
    )?;
    let mut speed_axis = Axis::new(
        AxisSpec {
            abs: EV_ABS::ABS_RY,
            min: opt.min_speed.ln(),
            max: opt.max_speed.ln(),
            time_to_max_s: -5.0,
        },
        &ev_device,
        now,
    )?;
    speed_axis.driven = opt.init_speed.ln();
    println!(
        "{:?} {:?} {:?} {:?}",
        pos_axis, stroke_len_axis, asymmetry_axis, speed_axis
    );
    let mut drive = false;
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
                        if k.1.event_code == evdev_rs::enums::EventCode::EV_KEY(evdev_rs::enums::EV_KEY::BTN_TR) {
                            if k.1.value == 1 {
                                drive = true;
                            } else {
                                drive = false;
                                ctrl.target_speed[0].store(0, Ordering::Relaxed);
                                ctrl.target_speed[1].store(0, Ordering::Relaxed);
                            }
                            for ax in [&mut pos_axis, &mut stroke_len_axis, &mut asymmetry_axis, &mut speed_axis].iter_mut() {
                                ax.handle_tick(k.1.time);
                                ax.drive = drive;
                            }
                        } else {
                            for ax in [&mut pos_axis, &mut stroke_len_axis, &mut asymmetry_axis, &mut speed_axis].iter_mut() {
                                ax.handle_event(&k.1);
                            }
                        }
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
                let now = timeval::now()?;
                //println!("tick {:?}", now);
                for ax in [&mut pos_axis, &mut stroke_len_axis, &mut asymmetry_axis, &mut speed_axis].iter_mut() {
                    ax.handle_tick(now);
                }
                // Triangular clamp on stroke length
                pos_axis.driven = pos_axis.driven.max(pos_axis.spec.min + stroke_len_axis.driven).min(pos_axis.spec.max - stroke_len_axis.driven);
                if drive {
                    let ends = [
                        ((pos_axis.driven - stroke_len_axis.driven) as i64).max(-opt.max_pos),
                        ((pos_axis.driven + stroke_len_axis.driven) as i64).min(opt.max_pos),
                    ];
                    let v = speed_axis.driven.exp();
                    let target_speed0 = (v * (1.0 + asymmetry_axis.driven).min(1.0)).min(opt.max_speed);
                    let target_speed1 = (v * (1.0 - asymmetry_axis.driven).min(1.0)).min(opt.max_speed);
                    //println!("{:?} {}", ends, target_speed);
                    ctrl.ends[0].store(ends[0], Ordering::Relaxed);
                    ctrl.ends[1].store(ends[1], Ordering::Relaxed);
                    ctrl.target_speed[0].store((target_speed0 / device::CONTROL_FACTOR) as i64, Ordering::Relaxed);
                    ctrl.target_speed[1].store((target_speed1 / device::CONTROL_FACTOR) as i64, Ordering::Relaxed);
                }
            }
            _ = report.tick() => {
                println!("{:5} {:5} {:5} {:5} {}", pos_axis.driven, stroke_len_axis.driven, asymmetry_axis.driven, speed_axis.driven.exp(), drive);
            }
        }
    }
    println!("Finished joystick loop");
    Ok(())
}
