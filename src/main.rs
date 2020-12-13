// Copyright (c) 2017-2019 Rene van der Meer
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

// gpio_blinkled_signals.rs - Blinks an LED connected to a GPIO pin in a loop,
// while handling any incoming SIGINT (Ctrl-C) and SIGTERM signals so the
// pin's state can be reset before the application exits.
//
// Remember to add a resistor of an appropriate value in series, to prevent
// exceeding the maximum current rating of the GPIO pin and the LED.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use rppal::gpio::{Gpio, Level};

// The simple-signal crate is used to handle incoming signals.
use simple_signal::{self, Signal};

use structopt::StructOpt;


#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "800")]
    steps: i64,

    #[structopt(long, default_value = "3")]
    period: f64,
}

// Gpio uses BCM pin numbering.
const GPIO_PUL: u8 = 13;
const GPIO_DIR: u8 = 16;

fn gen_steps(opt: Opt) -> Vec<f64> {
    let period = opt.period / std::f64::consts::TAU;
    (0..opt.steps)
        .map(|i| period * (((opt.steps - (i * 2 + 1)) as f64) / (opt.steps as f64)).acos())
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    println!("{:?}", opt);
    let steps = gen_steps(opt);
    // Retrieve the GPIO pin and configure it as an output.
    let gpio = Gpio::new()?;
    let mut pul_pin = gpio.get(GPIO_PUL)?.into_output();
    let mut dir_pin = gpio.get(GPIO_DIR)?.into_output();

    let running = Arc::new(AtomicBool::new(true));

    // When a SIGINT (Ctrl-C) or SIGTERM signal is caught, atomically set running to false.
    simple_signal::set_handler(&[Signal::Int, Signal::Term], {
        let running = running.clone();
        move |_| {
            running.store(false, Ordering::SeqCst);
        }
    });

    // Blink pulse
    'outer: loop {
        for &level in &[Level::Low, Level::High] {
            dir_pin.write(level);
            thread::sleep(Duration::from_millis(10));
            if !running.load(Ordering::SeqCst) {
                break 'outer;
            }
            let min = Duration::from_micros(30);
            let skip = Duration::from_micros(35);
            let mut start = Instant::now();
            let mut skips = 0;
            for &nx in &steps {
                let target = Duration::from_secs_f64(nx);
                let sofar = Instant::now() - start;
                if target >= sofar + min {
                    thread::sleep(target - sofar);
                } else {
                    thread::sleep(skip);
                    start += sofar + skip - target;
                    skips += 1;
                }
                pul_pin.set_high();
                thread::sleep(Duration::from_micros(1));
                pul_pin.set_low();
            }
            println!("skips: {}", skips);
        }
    }

    pul_pin.set_low();
    dir_pin.set_low();

    Ok(())

    // When the pin variable goes out of scope, the GPIO pin mode is automatically reset
    // to its original value, provided reset_on_drop is set to true (default).
}
