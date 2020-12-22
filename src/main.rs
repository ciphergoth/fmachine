use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

mod timeval;

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut interval = time::interval(Duration::from_millis(50));
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let abs_x = evdev_rs::enums::EventCode::EV_ABS(evdev_rs::enums::EV_ABS::ABS_X);
    let ai = ev_device.abs_info(&abs_x).ok_or_else(|| anyhow!("wtf"))?;
    println!(
        " min {} max {} fuzz {} flat {} res {}",
        ai.minimum, ai.maximum, ai.fuzz, ai.flat, ai.resolution
    );
    let afd = AsyncFd::with_interest(
        ev_device,
        Interest::READABLE,
    )?;
    let mut driven = 0.0;
    let mut last_read = None;
    loop {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;

                let a = afd.get_ref().next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        //println!("Event: {:?}", k.1);
                        if k.1.event_code == abs_x {
                            if let Some((t, v)) = last_read {
                                driven += v as f64 * timeval::diff_as_f64(&k.1.time, &t);
                            }
                            let new_v = if k.1.value <= ai.flat && k.1.value >= -ai.flat {
                                0
                            } else {
                                k.1.value
                            };
                            last_read = Some((k.1.time, new_v));
                            println!("driven: {} stick {} new_v {}", driven, k.1.value, new_v);
                        }
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
                if let Some((t, v)) = last_read {
                    driven += v as f64 * timeval::diff_as_f64(&now, &t);
                    println!("driven: {}", driven);
                    last_read = Some((now, v));
                }
            }
        }
    }
}
