import { expect, it } from "vitest";
import { clampOverlayPosition } from "./overlay-drag";

const monitor = {
  workArea: {
    position: { x: 100, y: 40 },
    size: { width: 1200, height: 800 },
  },
};
const windowSize = { width: 360, height: 700 };

it("clamps the overlay before setting its position at every work-area edge", () => {
  expect(
    clampOverlayPosition(
      { x: -500, y: -300 },
      windowSize,
      { x: -400, y: -250 },
      [monitor],
    ),
  ).toEqual({ x: 100, y: 40 });
  expect(
    clampOverlayPosition(
      { x: 1800, y: 1200 },
      windowSize,
      { x: 1900, y: 1250 },
      [monitor],
    ),
  ).toEqual({ x: 940, y: 140 });
});

it("uses the work area under the pointer on a second monitor", () => {
  const second = {
    workArea: {
      position: { x: 1300, y: 0 },
      size: { width: 900, height: 900 },
    },
  };
  expect(
    clampOverlayPosition({ x: 1900, y: 100 }, windowSize, { x: 2000, y: 120 }, [
      monitor,
      second,
    ]),
  ).toEqual({ x: 1840, y: 100 });
});
