use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::os::unix::fs::OpenOptionsExt;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::io::Interest;
use tokio::{io::unix::AsyncFd, time};

#[tokio::main]
pub async fn main() -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1));
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_fd(fd)?;
    let afd = AsyncFd::with_interest(
        ev_device.fd().ok_or_else(|| anyhow!("wtf"))?,
        Interest::READABLE,
    )?;
    loop {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;

                let a = ev_device.next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        println!("Event: {:?}", k.1);
                        guard.retain_ready();
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        println!("would block");
                        guard.clear_ready();
                    }
                    not_ok => {
                        println!("boom");
                        not_ok?;
                    }
                }
            }
            _ = interval.tick() => {
                println!("tick");
            }
        }
    }
}
