#!/usr/bin/env python3

from __future__ import annotations
import argparse
import math
from typing import Iterator

import svg


def mounting_holes(
    x: float, y: float, w: float, h: float, d: float
) -> Iterator[svg.Element]:
    for xo in [-1, 1] if w != 0 else [0]:
        for yo in [-1, 1] if h != 0 else [0]:
            yield svg.Circle(
                cx=x + w * xo / 2,
                cy=y + h * yo / 2,
                r=d / 2,
            )


def shaft_axis() -> Iterator[svg.Element]:
    # Brushes SCS-12UU
    # L = 39
    for p in [20, 460]:
        yield from mounting_holes(0, p, 30.5, 26, 5)
    # Pretty sure the metric specs are the authoritative ones
    # https://keesafety.co.uk/resources/downloads
    # https://keesafety.co.uk/media/3b3apds4/safety_components_catalogue_v10_0723_web.pdf
    # Flange 61-6, measurement G - what I used
    spacing = 64 / math.sqrt(2)
    # Flange 61-5, measurement G - what you should use
    # spacing = 57 / math.sqrt(2)
    yield from mounting_holes(0, 330, spacing, spacing, 6.5)


def belt_axis() -> Iterator[svg.Element]:
    # Idler pulley
    # Edge of top brush + 2mm clearance + radius of wheel
    # + distance from topmost hole to second holes
    # + 1/2 distance between holes
    top = 39.5 + 2 + 11 + 32 + 10
    yield from mounting_holes(0, top, 20, 20, 5.1)
    # Stepper motor 23HS30-3004S
    # What does 4-Ø5.2 mean?
    steppery = 410
    yield from mounting_holes(0, steppery, 47.14, 47.14, 5.2)
    yield from mounting_holes(0, steppery, 0, 0, 19)


def cut_elements() -> Iterator[svg.Element]:
    yield svg.G(
        transform=[
            svg.Translate(51 + 10, 0),
        ],
        elements=list(shaft_axis()),
    )
    yield svg.G(
        transform=[
            svg.Translate(51 - 10, 0),
        ],
        elements=list(belt_axis()),
    )
    # Stepper driver
    yield from mounting_holes(20, 514, 0, 112, 4.5)
    # Power cord
    yield from mounting_holes(20, 590, 0, 0, 10.8)
    # Pibow
    yield from mounting_holes(65, 550, 57.75, 90.75, 3)

    # Whole board
    yield svg.Rect(
        x=0,
        y=0,
        width=102,
        height=610,
        rx=5,
    )


def draw_elements(w: float, h: float) -> Iterator[svg.Element]:
    for x in range(100, int(w*10 + 1), 254):
        yield svg.Path(d=[svg.M(x/10, 5), svg.V(h)])
    for y in range(100, int(h*10 + 1), 254):
        yield svg.Path(d=[svg.M(5, y/10), svg.H(w)])


def elements(w: float, h: float) -> Iterator[svg.Element]:
    yield svg.G(
        style="stroke:red; stroke-width: 0.1;",
        elements=list(draw_elements(w, h))
    )
    yield svg.G(
        style="stroke:black; stroke-width: 0.3;",
        transform=[
            svg.Translate(10, 10),
        ],
        elements=list(cut_elements()),
    )


def draw() -> svg.SVG:
    w = int(0.5 + 20 + 25.4*4)
    h = int(0.5 + 20 + 25.4*24)
    return svg.SVG(
        width=svg.Length(w, "mm"),
        height=svg.Length(h, "mm"),
        viewBox=svg.ViewBoxSpec(0, 0, w, h),
        style="fill:transparent;",
        elements=list(elements(w, h)),
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("outfile")
    return parser.parse_args()


def main():
    args = parse_args()
    with open(args.outfile, "w") as f:
        f.write(str(draw()))


if __name__ == "__main__":
    main()
