use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use evdev_rs::TimeVal;
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

fn timeval_now() -> std::io::Result<TimeVal> {
    let mut t = libc::timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let res = unsafe { libc::gettimeofday(&mut t, std::ptr::null_mut()) };
    if res == 0 {
        Ok(TimeVal::from_raw(&t))
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn timeval_diff_as_f64(a: &TimeVal, b: &TimeVal) -> f64 {
    (a.tv_sec as f64) - (b.tv_sec as f64) + 0.000001 * ((a.tv_usec as f64) - (b.tv_usec as f64))
}

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut interval = time::interval(Duration::from_millis(17));
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_fd(fd)?;
    let afd = AsyncFd::with_interest(
        ev_device.fd().ok_or_else(|| anyhow!("wtf"))?,
        Interest::READABLE,
    )?;
    let mut value = 0.0;
    let mut last_read = None;
    loop {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;

                let a = ev_device.next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        //println!("Event: {:?}", k.1);
                        if k.1.event_code == evdev_rs::enums::EventCode::EV_ABS(
                            evdev_rs::enums::EV_ABS::ABS_X
                        ) {
                            if let Some((t, v)) = last_read {
                                value += v as f64 * timeval_diff_as_f64(&k.1.time, &t);
                            }
                            last_read = Some((k.1.time, k.1.value));
                            println!("value: {}", value);
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
                let now = timeval_now()?;
                //println!("tick {:?}", now);
                if let Some((t, v)) = last_read {
                    value += v as f64 * timeval_diff_as_f64(&now, &t);
                    println!("value: {}", value);
                    last_read = Some((now, v));
                }
            }
        }
    }
}
