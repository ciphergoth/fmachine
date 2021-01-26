use std::{
    fs::OpenOptions, io::ErrorKind, os::unix::fs::OpenOptionsExt, sync::Arc, time::Duration,
};

use anyhow::Result;
use tokio::{
    io::{unix::AsyncFd, Interest},
    time,
};

use crate::{device, joystick, timeval, Opt};

pub async fn main_loop(opt: Opt, ctrl: Arc<device::Control>) -> Result<()> {
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/input/event0")?;
    let ev_device = evdev_rs::Device::new_from_file(fd)?;
    let mut joystate = joystick::JoyState::new(opt, ctrl.clone(), &ev_device, timeval::now()?)?;
    println!("{:?}", joystate);
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)?;
    let mut interval = time::interval(Duration::from_millis(50));
    let mut report = time::interval(Duration::from_secs(1));
    while ctrl.run() {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r?;
                let a = afd.get_ref().next_event(evdev_rs::ReadFlag::NORMAL);
                match a {
                    Ok(k) => {
                        guard.retain_ready();
                        joystate.handle_event(k.1);
                    }
                    Err(e) if e.kind() == ErrorKind::WouldBlock => {
                        guard.clear_ready();
                    }
                    not_ok => {
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
