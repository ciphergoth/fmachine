use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

mod timeval;

#[derive(Debug)]
struct Axis {
    event_code: evdev_rs::enums::EventCode,
    abs_info: evdev_rs::AbsInfo,
    driven: f64,
    last_read: Option<(evdev_rs::TimeVal, i32)>,
}

impl Axis {
    fn new(ev_device: &evdev_rs::Device, event_code: evdev_rs::enums::EventCode) -> Result<Axis> {
        Ok(Axis {
            event_code,
            abs_info: ev_device.abs_info(&event_code).ok_or_else(|| anyhow!("wtf"))?,
            driven: 0.0,
            last_read: None
        })
    }

    fn handle_event(&mut self, event: &evdev_rs::InputEvent) {
        if event.event_code == self.event_code {
            if let Some((t, v)) = self.last_read {
                self.driven += v as f64 * timeval::diff_as_f64(&event.time, &t);
            }
            let new_v = if event.value <= self.abs_info.flat && event.value >= -self.abs_info.flat {
                0
            } else {
                event.value
            };
            self.last_read = Some((event.time, new_v));
            println!("driven: {} stick {} new_v {}", self.driven, event.value, new_v);
        }
    }

    fn handle_tick(&mut self, now: evdev_rs::TimeVal) {
        if let Some((t, v)) = self.last_read {
            self.driven += v as f64 * timeval::diff_as_f64(&now, &t);
            println!("driven: {}", self.driven);
            self.last_read = Some((now, v));
        }
    }
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut interval = time::interval(Duration::from_millis(50));
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let mut xaxis = Axis::new(&ev_device,
        evdev_rs::enums::EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_X))?;
    println!(
        " min {} max {} fuzz {} flat {} res {}",
        xaxis.abs_info.minimum, xaxis.abs_info.maximum, xaxis.abs_info.fuzz, xaxis.abs_info.flat, xaxis.abs_info.resolution
    );
    let afd = AsyncFd::with_interest(
        ev_device,
        Interest::READABLE,
    )?;
    loop {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;

                let a = afd.get_ref().next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        xaxis.handle_event(&k.1);
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
                xaxis.handle_tick(now);
            }
        }
    }
}
