export const FLOATING_WINDOW_DRAG_THRESHOLD_PX = 6;

/**
 * @param {{ x: number, y: number }} start
 * @param {{ x: number, y: number }} current
 */
export function pointerTravelDistance(start, current) {
  const deltaX = current.x - start.x;
  const deltaY = current.y - start.y;
  return Math.hypot(deltaX, deltaY);
}

/**
 * @param {{ x: number, y: number }} start
 * @param {{ x: number, y: number }} current
 * @param {number} [threshold]
 */
export function isFloatingWindowDrag(start, current, threshold = FLOATING_WINDOW_DRAG_THRESHOLD_PX) {
  return pointerTravelDistance(start, current) > threshold;
}

/**
 * @param {{ x: number, y: number }} start
 * @param {{ x: number, y: number }} current
 * @param {number} [threshold]
 * @returns {"click" | "drag"}
 */
export function classifyFloatingWindowPointerGesture(
  start,
  current,
  threshold = FLOATING_WINDOW_DRAG_THRESHOLD_PX
) {
  return isFloatingWindowDrag(start, current, threshold) ? "drag" : "click";
}

/**
 * @param {{ x: number, y: number }} start
 * @param {{ x: number, y: number }} current
 * @param {boolean} dragStarted
 * @param {number} [threshold]
 */
export function shouldOpenFloatingWindowMenuOnPointerUp(
  start,
  current,
  dragStarted,
  threshold = FLOATING_WINDOW_DRAG_THRESHOLD_PX
) {
  return (
    !dragStarted &&
    classifyFloatingWindowPointerGesture(start, current, threshold) === "click"
  );
}
