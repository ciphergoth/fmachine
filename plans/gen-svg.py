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
    # Brushes
    for p in [0, 300]:
        yield from mounting_holes(0, p, 30.5, 26, 5)
    # Flange 61-6, measurement G
    spacing = 64 / math.sqrt(2)
    # spacing = 20
    yield from mounting_holes(0, 150, spacing, spacing, 6)


def belt_axis() -> Iterator[svg.Element]:
    # Stepper motor 23HS30-3004S
    # What does 4-Ø5.2 mean?
    steppery = 200
    yield from mounting_holes(0, steppery, 47.14, 47.14, 5.2)
    yield from mounting_holes(0, steppery, 0, 0, 19)
    # Idler pulley - FIXME these are all invented
    yield from mounting_holes(0, 250, 30, 40, 5)


def elements() -> Iterator[svg.Element]:
    yield svg.Style(text="* {stroke:black; fill:transparent; stroke-width: 0.3}")
    yield svg.G(
        transform=[
            svg.Translate(25, 23),
        ],
        elements=list(shaft_axis()),
    )
    yield svg.G(
        transform=[
            svg.Translate(50, 23),
        ],
        elements=list(belt_axis()),
    )
    yield svg.Rect(
        x=0,
        y=0,
        width=102,
        height=610,
        stroke="black",
        fill="transparent",
        rx=5,
    )


def draw() -> svg.SVG:
    w = 122
    h = 630
    return svg.SVG(
        width=svg.Length(w, "mm"),
        height=svg.Length(h, "mm"),
        viewBox=svg.ViewBoxSpec(0, 0, w, h),
        elements=[
            svg.G(
                transform=[
                    svg.Translate(10, 10),
                ],
                elements=list(elements()),
            ),
        ],
    )


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("outfile")
    return parser.parse_args()


def main():
    args = parse_args()
    with open(args.outfile, "w") as f:
        f.write(str(draw()))


if __name__ == "__main__":
    main()
