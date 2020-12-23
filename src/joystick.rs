use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use evdev_rs::enums::EV_ABS;
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

use crate::device;
use crate::timeval;
use crate::Opt;

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
    let mut axes = vec![
        AxisSpec {
            abs: EV_ABS::ABS_X,
            min: -opt.max_pos as f64,
            max: opt.max_pos as f64,
            time_to_max_s: 5.0,
        },
        AxisSpec {
            abs: EV_ABS::ABS_Y,
            min: 0.0,
            max: opt.max_pos as f64,
            time_to_max_s: -5.0,
        },
        AxisSpec {
            abs: EV_ABS::ABS_RX,
            min: -0.5,
            max: 0.5,
            time_to_max_s: 5.0,
        },
        AxisSpec {
            abs: EV_ABS::ABS_RY,
            min: opt.min_velocity,
            max: opt.max_velocity,
            time_to_max_s: -5.0,
        },
    ]
    .into_iter()
    .map(|spec| Axis::new(spec, &ev_device, now))
    .collect::<Result<Vec<_>, _>>()?;
    axes[3].driven = opt.max_velocity;
    println!("{:?}", axes);
    let mut drive = false;
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)?;
    let mut interval = time::interval(Duration::from_millis(50));
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
                                ctrl.target_velocity.store(0, Ordering::Relaxed);
                            }
                            for ax in &mut axes {
                                ax.handle_tick(k.1.time);
                                ax.drive = drive;
                            }
                        } else {
                            for ax in &mut axes {
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
                for ax in &mut axes {
                    ax.handle_tick(now);
                }
                // Triangular clamp on stroke length
                axes[0].driven = axes[0].driven.max(axes[0].spec.min + axes[1].driven).min(axes[0].spec.max - axes[1].driven);
                println!("{:5} {:5} {:5} {:5}", axes[0].driven, axes[1].driven, axes[2].driven, axes[3].driven);
                if drive {
                    let ends = [((axes[0].driven - axes[1].driven) as i64).max(-opt.max_pos),
                    ((axes[0].driven + axes[1].driven) as i64).min(opt.max_pos),];
                    let target_velocity = axes[3].driven.min(opt.max_velocity);
                    println!("{:?} {}", ends, target_velocity);
                    ctrl.ends[0].store(ends[0], Ordering::Relaxed);
                    ctrl.ends[1].store(ends[1], Ordering::Relaxed);
                    ctrl.target_velocity.store((target_velocity / device::CONTROL_FACTOR) as i64, Ordering::Relaxed);
                }
            }
        }
    }
    println!("Finished joystick loop");
    Ok(())
}
