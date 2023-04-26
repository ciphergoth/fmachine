use std::{
    io::ErrorKind,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use log::{debug, info};
use tokio::{
    io::{unix::AsyncFd, Interest},
    sync::mpsc,
    time,
};

use crate::{device, joystick, Config};

pub async fn main_loop(
    config: Config,
    ctrl: Arc<device::Control>,
    mut status: mpsc::UnboundedReceiver<device::StatusMessage>,
) -> Result<()> {
    let ev_device =
        evdev_rs::Device::new_from_path("/dev/input/event0").context("open ev device")?;
    let mut joystate = joystick::JoyState::new(config, ctrl.clone(), &ev_device, SystemTime::now())
        .context("create JoyState")?;
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)
        .context("async wrap for ev device")?;
    let mut interval = time::interval(Duration::from_millis(50));
    let mut report = time::interval(Duration::from_secs(1));
    info!("Entering main loop");
    while !ctrl.stop() {
        tokio::select! {
            r = afd.readable() => {
                let mut guard = r.context("read from ev device")?;
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
                        not_ok.context("event from ev device")?;
                    }
                }
            }
            _ = interval.tick() => {
                joystate.handle_tick(SystemTime::now());
            }
            _ = report.tick() => {
                joystate.report();
            }
            val = status.recv() => {
                if let Some(status) = val {
                    joystate.handle_status(status);
                }
            }
        }
    }
    debug!("Finished joystick loop");
    Ok(())
}
