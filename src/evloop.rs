use std::{
    io::ErrorKind,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::Result;
use log::{debug, error, info};
use tokio::{
    io::{unix::AsyncFd, Interest},
    sync::mpsc,
    time,
};

use crate::{device, joystick, Opt};

pub async fn main_loop(
    opt: Opt,
    ctrl: Arc<device::Control>,
    mut status: mpsc::UnboundedReceiver<device::StatusMessage>,
) -> Result<()> {
    let ev_device = evdev_rs::Device::new_from_path("/dev/input/event0")?;
    let mut joystate = joystick::JoyState::new(opt, ctrl.clone(), &ev_device, SystemTime::now())?;
    debug!("{:?}", joystate);
    let afd = AsyncFd::with_interest(ev_device, Interest::READABLE)?;
    let mut interval = time::interval(Duration::from_millis(50));
    let mut report = time::interval(Duration::from_secs(1));
    info!("Entering main loop");
    while !ctrl.stop() {
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
                joystate.handle_tick(SystemTime::now());
            }
            _ = report.tick() => {
                joystate.report();
            }
            val = status.recv() => {
                match val {
                    Some(status) => joystate.handle_status(status),
                    None => error!("Status channel closed unexpectedly")
                }
            }
        }
    }
    debug!("Finished joystick loop");
    Ok(())
}
