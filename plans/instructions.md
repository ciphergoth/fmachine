# Instructions

I need your help improving these instructions!

I built and refined this over a period of three years. I don't really know
exactly what steps I followed to build it, and you want to follow a straighter
path than I did anyway. So here's instruction number 1:

* get in touch with me at fmachine@paul.ciphergoth.org and tell me you're
  building this thing
* talk to me throughout the process and tell me about the problems you encounter.

That way I can refine these instructions to smooth the path for the people who come after you.

## Caveats

These plans are a mixture of what I did and what I didn't do but I think you
should do; this means that as of the time of writing parts of it have never been
tested. Hopefully I'll get feedback from other builders and update these with
what works.

I describe how to build this using a laser cutter. I have never tried this, I'm
waiting for our work one to be repaired. When it's working, I'll use an Epilog
Legend 36EXT to test it.

The parts are all specified in metric, but I live in the USA, so this is a
terrible mix of metric and imperial measurements.

This will be nearly impossible without referring to photographs of the finished
device. If there are no such photos, don't attempt to build it but hassle me to
upload photos instead.

## Overview

* Set up the Raspberry Pi
* Use the Pi to generate the laser cutter instructions
* Cut the board
* Screw the parts to the board
* Attach the timing belt
* Bolt the frame together
* Enjoy!

## Tools needed

* Allen key set
* Ratchet socket with 1/2in hex bit
* Adjustable spanner
* Slip joint pliers (to tension the belt)
* Diagonal pliers (to cut the belt and zip ties)
* Laser cutter (I plan to try an Epilog Legend 36EXT)

## Instructions

* Buy the [parts](parts.md)
* Use the [Raspberry Pi Imager](https://www.raspberrypi.com/software/) to set up the SD card.
  * Raspberry Pi OS
  * Lite
  * 64-bit
  * Advanced options:
    * Hostname `fmachine`
    * enable SSH with password access
    * set username and password
    * configure wifi
    * configure locale settings
* Insert the SD card into the Pi
* Connect Pi to a power source
* Log into the Pi over SSH
* Run the following:

```
sudo apt install git
git clone https://github.com/ciphergoth/fmachine.git
cd fmachine
./install-prerequisites.sh
# FIXME: pip install -r requirements.txt
./plans/gen-svg.py /tmp/fmachine.svg
cargo build --release
sudo ./install.sh
```

* Use scp to make a local copy /tmp/fmachine.svg
* Run `sudo shutdown -h now`
* Disconnect power
* Construct the frame as per the photograph
  * FIXME take a photo of the frame and add here
* Attach the pulley to the stepper motor
* Set the [DIP switches on the stepper controller](dip-switches.md)
* Screw the various components to the baseboard as shown in the picture
  * don't forget to use washers
  * leave the flange till second last
  * leave the stepper motor until last
  * The face of the stepper motor is not even, you'll need to place washers
    between the motor and the board to get it to lie flat
* Connect the DC-DC converter to the Pi
  * Just use a 6" USB A-C cable, I see no advantage to the more complex thing I
    did.
* Cut, strip, and connect the wiring
  * Stepper motor controller to GPIO pins
  * Power socket to stepper motor controller, and DC-DC converter to stepper
    motor controller
  * FIXME more detail here
* Try it out
  * Plug in the joystick to a USB port
  * Plug the power in
  * Wait a bit for the Pi to boot
  * Squeeze the right trigger
  * Watch the wheel turn back and forth
* Attach the collars and belt to the rod
  * I really need to make a video on how to do this, it's hard to describe.
  * Put the machine in front of you with the stepper motor at the left end and
    the free pulley on the right.
  * The pulleys and the toothed wheel should now be above the shaft.
  * Slide the rod through one pillow, then two 15mm collars, then the other
    pillow.
  * Put a 12mm collar at the very leftmost end of the rod. Tighten.
  * Thread the end of the timing belt through the leftmost 15mm collar
    * in the right-to-left direction
    * with teeth facing upwards towards the pulleys
  * Pull a decent amount of slack through the collar
    * you'll adjust the exact amount later, but it needs to go around both
      pulleys
  * Now thread the same end of the timing belt through the rightmost 15mm collar
    * in the right-to-left direction
    * with teeth facing upwards towards the pulleys
    * until about 50mm is sticking out
  * Secure the belt to the rightmost 15mm collar
    * Fold the belt onto itself so that the teeth mesh together
    * Use a zip tie to attach the belt to itself as close as possible to the
      collar.
    * Put a second zip tie around 10mm from the first.
    * Cut the spare plastic from the zip ties with the diagonal cutters
  * Slide the rod all the way in until the 12mm collar touches the pillow.
  * Move the rightmost 15mm collar with belt attached until it is close to
    hitting the screws holding the pulley in place. Tighten the collar into
    place.
  * Move the leftmost 15mm collar until it is about 10-15mm away from the
    rightmost one.
  * Loop the belt around the pulley and the toothed wheel, and pull tight
  * Tighten the leftmost collar into place temporarily
  * Secure the belt to the leftmost 15mm collar
    * Fold the belt onto itself so that the teeth mesh together
    * Use a zip tie to attach the belt to itself as close as possible to the
      collar.
    * Put a second zip tie around 10mm from the first.
    * Cut the spare plastic from the zip ties with the diagonal cutters
  * Tension the belt
    * Loosen the leftmost 15mm collar again
    * Grab both collars with the slip joint pliers and squeeze.
      * If you can't get both collars in the teeth of the pliers, you have too
        little slack on the belt.
      * If they touch, you have too much slack on the belt.
      * In either case, redo securing the belt on the leftmost collar after
        adjusting the slack.
    * While squeezing, tighten the leftmost 15mm collar again.
  * With the belt tensioned, you have done the hardest part!
* Slide the rod until the zip tie on the leftmost collar is close to the
  toothed wheel
* Put the 12mm collar on the rightmost end of the rod, slide until it touches
  the pillow, and tighten.
* Slide the Vac-U-Lock adapter onto the rightmost end and tighten the grub screws
* Build the frame (uh instructions to follow)
* Put the fuck machine on the frame
* [Enjoy!](using.md)
