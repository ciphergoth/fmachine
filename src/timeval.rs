use evdev_rs::TimeVal;

pub fn now() -> std::io::Result<TimeVal> {
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

pub fn diff_as_f64(a: &TimeVal, b: &TimeVal) -> f64 {
    (a.secs() as f64) - (b.secs() as f64)
        + 0.000001 * ((a.subsec_micros() as f64) - (b.subsec_micros() as f64))
}
