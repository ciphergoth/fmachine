# Parts and instructions

I need your help improving these instructions!

I built and refined this over a period of three years. I don't really know exactly what steps I followed to build it, and you want to follow a straighter path than I did anyway. So here's instruction number 1:

* get in touch with me at fmachine@paul.ciphergoth.org and tell me you're building this thing. 
* talk to me throughout the process and tell me about the problems you encounter

That way I can refine these instructions to smooth the path for the people who come after you.

I live in the USA but I buy components that are used internationally, so this is a terrible mix of metric and imperial measurements.

The instructions here are for building the exact machine I have tested. I don't expect people to build the exact same thing I have. Some parts are now unavailable and will need to be replaced; and eg the switch from 3/4" ND pipe for the frame to 1" ND pipe for the flange on the base board is simply because I didn't have a 3/4" ND flange handy.

## Tools needed

* Laser cutter (I used an Epilog Legend 36EXT, but maybe a service like [Ponoko](https://www.ponoko.com/laser-cutting) can do this?)
* Allen key set
* Heat gun
* Ratchet socket 

## Frame parts

I used Kee Klamps and 3/4" ND pipe for the frame, except for the flange on the baseboard which is 1" ND. Kee Klamps are a kind of Meccano/Erector set for constructions like this; they're expensive but very flexible if you build up a collection of pipes. 

Confusingly, pipes have a "nominal diameter" (ND) which is neither their outer diameter (OD) nor their inner diameter (ID) ([Wikipedia](https://en.wikipedia.org/wiki/Nominal_Pipe_Size)). 3/4" ND = 1.050" OD, and 1" ND = 1.315" OD.

* Steel piping. All Schedule 40. I bought all mine from [Naylor Steel](https://www.naylorsteel.com/)
  * 3 x 3ft x 3/4" ND
  * 5 x 4ft x 3/4" ND
  * 1 x 2ft x 1" ND
* [Plastic caps](https://www.simplifiedbuilding.com/pipe-fittings/kee-klamp/133-plastic-plug)
  * 16 x 133-5
  * 2 x 133-6
* [Crossovers](https://www.simplifiedbuilding.com/pipe-fittings/kee-klamp/45-crossover)
  * 10 x 45-5
  * 1 x 45-65
  * Lowes sell a much cheaper version of the 45-5 made by SteelTek.
* [Flange](https://www.simplifiedbuilding.com/pipe-fittings/kee-klamp/61-flange)
  * 1 x 61-6

## Machine parts

* Plywood - try 3/8 in
* Power supply https://www.amazon.com/COOLM-Switching-100-240V-5-5x2-5mm-Injector/dp/B083R7H6FD
* Power socket https://www.amazon.com/dp/B08271YLXZ/
* DC-DC converter https://www.amazon.com/GERI-Converter-Module-8-50V-Output/dp/B00W52N8XW/
* Stepper driver DM542 https://www.amazon.com/dp/B07YWZRXGR/
* Stepper motor 23HS30-3004S https://www.amazon.com/dp/B00PNEPW4C/
* Pulley 3GT (GT2-3M) https://openbuildspartstore.com/3gt-gt2-3m-timing-pulley-20-tooth/
* Timing belt 3GT (GT2-3M) https://openbuildspartstore.com/3gt-gt2-3m-timing-belt-by-the-foot/
* Idler pulley https://openbuildspartstore.com/smooth-idler-pulley-kit/
* Idler mount https://openbuildspartstore.com/idler-pulley-plate/
* Brushes: uxcell SCS12UU 12mm https://www.amazon.com/dp/B07GQX9CL1/
* Shaft: ReliaBot 12mm x 800mm Linear Motion Shaft	https://www.amazon.com/dp/B08F7KST49/
* Collars 12mm https://www.amazon.com/uxcell-Collar-Wrench-Carbon-Woodworking/dp/B07KWWGTR6/
* Vac-U-Lock attachment - ask for one secured with set screws https://www.etsy.com/listing/733855115/vac-u-lock-adapter-for-use-with-sex
* Dildo - any Vac-U-Lock compatible one is good https://www.amazon.com/Doc-Johnson-Vac-U-Lock-ULTRASKYN-Compatible/dp/B00OMSZUTW
* Joystick https://www.amazon.com/Wireless-Controller-Gamepad-Computer-Windows/dp/B07PQ62D7V/ref=zg_bs_402045011_20?_encoding=UTF8&psc=1&refRID=6YZEXNECC47Q62SYVA0A
* Wires https://www.amazon.com/gp/product/B01IB7UOFE/
* Collars 15mm https://www.mcmaster.com/catalog/129/1433/6056N23
* M5 screws 18mm x100 https://www.mcmaster.com/catalog/129/3494/91290A238
* M5 screws 25mm x50 https://www.mcmaster.com/catalog/129/3494/91290A252
* M5 nuts x100 https://www.mcmaster.com/catalog/129/3584/90592A095
* M5 washers x100 https://www.mcmaster.com/catalog/129/3628/93475A240
* x100 Phillips Flat Head Screws for Wood, Zinc-Plated Steel, Number 6 Size, 3/8" Long https://www.mcmaster.com/catalog/129/3376/90031A146
 

The Etsy listing for the Vac-U-Lock attachment includes various fastening options, but none of them seemed good for this project, so I asked them to make one that fastened with two set screws, it seems to work very well.

## FIXME

* Pi 4B
* Pi case
* Heat shrink tubing
* other collars (used to fasten belt to shaft)
* bolts
* washers
* nuts
* FIXME how does voltage converter connect to Pi etc? What tools are needed?

## Instructions

FIXME test this with a fresh SD card - some steps probably missing.

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
* Put the Pi in its case
* Connect Pi to a power source
* Log into the Pi over SSH
* [Install Rust](https://rustup.rs/)
* Run the following:

```
sudo apt-get install git
git clone https://github.com/ciphergoth/fmachine.git
cd fmachine
sudo ./apt-get-prerequisites.sh
cargo build --release
sudo ./install.sh
sudo shutdown -h now
```

* Disconnect power
* Construct the frame as per the photograph
* Attach the pulley to the stepper motor
* FIXME set the DIP switches on the stepper controller
* Screw the various components to the baseboard as shown in the picture
  * don't forget to use washers
  * leave the flange till second last
  * leave the stepper motor until last
  * The face of the stepper motor is not even, you'll need to place washers between the motor and the board to get it to lie flat
* Connect the DC-DC converter to the Pi
  * Cut the USB connector off the DC-DC converter
  * Strip the wires
  * Solder them to GPIO pin wires
  * Cover the connections with heatsink wrap
  * Connect to the GPIO power pins
  * This is what I did, but this bypasses all the surge protection etc so it's dangerous for the Pi. Maybe just using the USB connector and the USB input to the Pi is a better idea?
* Cut, strip, and connect the wiring
  * Stepper motor controller to GPIO pins
  * Power socket to stepper motor controller, and DC-DC converter to stepper motor controller
  * FIXME more detail here
* Try it out
  * Plug in the joystick to a USB port
  * Plug the power in
  * Wait a bit for the Pi to boot
  * Squeeze the right trigger
  * Watch the wheel turn back and forth
* Thread the rod through the first brush, then the two 15mm collars, then the second brush
* Put the 12mm collar on the stepper motor end