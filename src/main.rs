use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

use anyhow::Result;
use tokio::io::unix::AsyncFd;
use tokio::io::Interest;

#[tokio::main]
pub async fn main() -> Result<()> {
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_fd(fd)?;
    let afd = AsyncFd::with_interest(ev_device.fd().unwrap(), Interest::READABLE)?;
    loop {
        let mut guard = afd.readable().await?;

        let a = ev_device.next_event(evdev_rs::ReadFlag::NORMAL);
        match a {
            Ok(k) => {
                println!("Event: {:?}", k.1);
                guard.retain_ready();
            }
            Err(e) => {
                println!("{:?}", e);
                guard.clear_ready();
            }
        }
    }
}
