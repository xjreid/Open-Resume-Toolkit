type Point = { x: number; y: number };
type Size = { width: number; height: number };
type WorkArea = { position: Point; size: Size };

function distanceToArea(point: Point, area: WorkArea): number {
  const left = area.position.x;
  const top = area.position.y;
  const right = left + area.size.width;
  const bottom = top + area.size.height;
  const dx = Math.max(left - point.x, 0, point.x - right);
  const dy = Math.max(top - point.y, 0, point.y - bottom);
  return dx * dx + dy * dy;
}

export function clampOverlayPosition(
  desired: Point,
  windowSize: Size,
  pointer: Point,
  monitors: readonly { workArea: WorkArea }[],
): Point {
  const area = monitors.reduce<WorkArea | null>((closest, monitor) => {
    if (!closest) return monitor.workArea;
    return distanceToArea(pointer, monitor.workArea) <
      distanceToArea(pointer, closest)
      ? monitor.workArea
      : closest;
  }, null);
  if (!area) return { x: Math.round(desired.x), y: Math.round(desired.y) };
  const minX = area.position.x;
  const minY = area.position.y;
  const maxX = Math.max(minX, minX + area.size.width - windowSize.width);
  const maxY = Math.max(minY, minY + area.size.height - windowSize.height);
  return {
    x: Math.round(Math.min(maxX, Math.max(minX, desired.x))),
    y: Math.round(Math.min(maxY, Math.max(minY, desired.y))),
  };
}
