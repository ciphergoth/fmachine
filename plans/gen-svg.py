#!/usr/bin/env python3

from __future__ import annotations
import math
from typing import Iterator

import svg

def mounting_holes(x: float, y: float, w: float, h: float, d: float)  -> Iterator[svg.Element]:
    for xo in [-1, 1]:
        for yo in [-1, 1]:
            yield svg.Circle(
                cx=x + w * xo / 2,
                cy=y + h * yo / 2,
                r=d/2,
            )


def shaft_axis() -> Iterator[svg.Element]:
    for p in [0, 300]:
        yield from mounting_holes(0, p, 30.5, 26, 5)
    spacing = 64/math.sqrt(2)
    #spacing = 20
    yield from mounting_holes(0, 150, spacing, spacing, 6)


def elements() -> Iterator[svg.Element]:
    yield svg.Style(text="* {stroke:black; fill:transparent; stroke-width: 0.3}")
    yield svg.G(
        transform=[
            svg.Translate(25, 23),
        ],
        elements=list(shaft_axis()),
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
    scale = 3
    return svg.SVG(
        width=122 * scale,
        height=630 * scale,
        viewBox=svg.ViewBoxSpec(0, 0, 122 * scale, 630 * scale),
        elements=[
            svg.G(
                transform=[
                    svg.Scale(scale),
                    svg.Translate(10, 10),
                ],
                elements=list(elements()),
            ),
        ],
    )


if __name__ == "__main__":
    with open("/tmp/plan.svg", "w") as f:
        f.write(str(draw()))
