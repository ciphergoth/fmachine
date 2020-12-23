use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use evdev_rs::enums::EV_ABS;
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

mod timeval;
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
    last_read: Option<(evdev_rs::TimeVal, i32)>,
}

impl Axis {
    fn new(spec: AxisSpec, ev_device: &evdev_rs::Device) -> Result<Axis> {
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
            last_read: None,
        })
    }

    fn handle_change(&mut self, now: evdev_rs::TimeVal, new_v: Option<i32>) {
        if let Some((t, v)) = self.last_read {
            self.driven += v as f64 * self.per * timeval::diff_as_f64(&now, &t);
            self.driven = self.driven.max(self.spec.min).min(self.spec.max);
            self.last_read = Some((now, new_v.unwrap_or(v)));
        } else if let Some(new_v) = new_v {
            self.last_read = Some((now, new_v));
        }
    }

    fn handle_event(&mut self, event: &evdev_rs::InputEvent) {
        if event.event_code != evdev_rs::enums::EventCode::EV_ABS(self.spec.abs) {
            return;
        }
        let new_v = if event.value <= self.flat && event.value >= -self.flat {
            0
        } else {
            event.value
        };
        self.handle_change(event.time, Some(new_v));
    }

    fn handle_tick(&mut self, now: evdev_rs::TimeVal) {
        self.handle_change(now, None);
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let mut axes = vec![
        AxisSpec {
            abs: EV_ABS::ABS_X,
            min: -400.0,
            max: 400.0,
            time_to_max_s: 5.0,
        },
        AxisSpec {
            abs: EV_ABS::ABS_Y,
            min: 0.0,
            max: 400.0,
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
            min: 0.0,
            max: 400.0,
            time_to_max_s: -5.0,
        },
    ]
    .into_iter()
    .map(|spec| Axis::new(spec, &ev_device))
    .collect::<Result<Vec<_>, _>>()?;
    println!("{:?}", axes);
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)?;
    let mut interval = time::interval(Duration::from_millis(50));
    loop {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;
                let a = afd.get_ref().next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        for ax in &mut axes {
                            ax.handle_event(&k.1);
                        }
                        println!("Event: {:?}", k.1);
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
                println!("{:5} {:5} {:5} {:5}", axes[0].driven, axes[1].driven, axes[2].driven, axes[3].driven)
            }
        }
    }
}
