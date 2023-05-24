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
    for p in [20, 460]:
        yield from mounting_holes(0, p, 30.5, 26, 5)
    # Flange 61-6, measurement G
    spacing = 64 / math.sqrt(2)
    yield from mounting_holes(0, 330, spacing, spacing, 6)


def belt_axis() -> Iterator[svg.Element]:
    # Idler pulley - FIXME these are all invented
    yield from mounting_holes(0, 90, 30, 30, 5)
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
    # FIXME Pibow
    yield from mounting_holes(65, 550, 60, 90, 3)

    # Whole board
    yield svg.Rect(
        x=0,
        y=0,
        width=102,
        height=610,
        stroke="black",
        fill="transparent",
        rx=5,
    )

def draw_elements(w: float, h: float) -> Iterator[svg.Element]:
    for x in range(0, int(w+1), 20):
        yield svg.Path(d=[svg.M(x, 5), svg.V(h)])
    for y in range(0, int(h+1), 20):
        yield svg.Path(d=[svg.M(5, y), svg.H(w)])

def elements(w: float, h: float) -> Iterator[svg.Element]:
    yield svg.G(
        style="stroke:red; fill:transparent; stroke-width: 0.1",
        elements=list(draw_elements(w, h)))
    yield svg.G(
        style="stroke:black; fill:transparent; stroke-width: 0.3",
                transform=[
                    svg.Translate(10, 10),
                ],
                elements=list(cut_elements()),
            )

def draw() -> svg.SVG:
    w = 122
    h = 630
    return svg.SVG(
        width=svg.Length(w, "mm"),
        height=svg.Length(h, "mm"),
        viewBox=svg.ViewBoxSpec(0, 0, w, h),
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
