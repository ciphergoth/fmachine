# fmachine - the fuck machine you can make

Plans and code for an open-source software-controlled fuck machine designed
for easy construction.

## DANGER OF DEATH

* A fuck machine applies significant force to delicate areas
* If that force goes wrong, it can probably harm someone a lot or worse
* This one is software controlled; software is evil and unreliable
* Proceed at your own risk.

## Warning: very early stage project

I have built a fuck machine based on the plans and software in this repo; it
works very well and people love it.

That said, as of the time of writing, this is a PRE-RELEASE, ALPHA-QUALITY
PROJECT.

* The only one of these machines that has ever been built is my own.
* These instructions are therefore completely untested.
* They are also pretty incomplete.
* There are many places where I made a decision which was fairly idiosyncratic
  and contingent on what I happened to have to hand. These plans are a mixture
  of what I did and what I didn't do but I think you should do.
* I also don't have a super clear idea what it costs - I think it's only a few
  hundred dollars but I'm not sure. Hopefully a lot cheaper for you than for me
  since you won't take my wrong turns.
* I measured and drilled the baseboard I use by hand. My plan was that you
  should just be able to laser-cut yours. However, the laser cutter I have
  access to is currently broken.
* Until I can test this for you, you're committing to an iterative process: try
  the current [`gen-svg`](plans/gen-svg.py) to build laser cutting plans, find
  out what's wrong with the resulting cut, fix it and try again.
* If you get it working, please submit a pull request with a working plan.

## Introduction

Nearly all the fuck machines you can buy are based on the same principle - a
motor turns a wheel in a smooth motion, and a cam converts the rotary motion
into a back and forth motion. This works well, but it has significant
limitations: changing eg the stroke length or the depth of thrust requires
adjusting the mechanism.

Recently a new kind of fuck machine has appeared: the software controlled fuck
machine. This design generally has a much simpler mechanism which converts
motion of a stepper motor to motion of a shaft, with the software generating the
back-and-forth pattern required. This means the pattern can be whatever you can
program, and it can be adjusted on the fly without stopping. In particular, this
means that asymmetric thrust, in which the inward thrust is faster than the
pulling back, can be programmed; it turns out that asymmetry leads to a much
more pleasant fuck.

However the only such machine I can find that can be bought off the shelf costs
over $2500, and the software for this machine is not open source, meaning that
what it can do is limited to what the creators programmed.

After a lover asked to try a fuck machine, and during the COVID lockdown, I
decided I would build one myself. The only problem was: I'm not very good at
making things, and I knew this would be more ambitious than anything I'd already
done. So I gave a lot of thought to how to design it such that it was within my
abilities to actually build it. This design is the result - and hopefully it'll
be even easier for you to build now I've written the software and paved the way.
I've tried this on five different lovers and myself, and people like it!

## Instructions

* [Very incomplete instructions on how to build it](plans/instructions.md)
* [How to use it](plans/using.md)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT
license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in fmachine by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Obviously this project is nothing to do with my employer.
