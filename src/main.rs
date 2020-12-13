use evdev_rs::Device;
use std::fs::File;

fn main() -> ! {
    let f = File::open("/dev/input/event0").unwrap();

    let mut d = Device::new().unwrap();
    d.set_fd(f).unwrap();
    
    loop {
        let a = d.next_event(evdev_rs::ReadFlag::NORMAL | evdev_rs::ReadFlag::BLOCKING);
        match a {
            Ok(k) => println!("Event: time {}.{}, ++++++++++++++++++++ {} +++++++++++++++",
                              k.1.time.tv_sec,
                              k.1.time.tv_usec,
                              k.1.event_type),
            Err(e) => (),
        }
    }
}

