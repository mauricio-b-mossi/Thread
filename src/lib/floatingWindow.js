export const FLOATING_WINDOW_DRAG_THRESHOLD_PX = 6;
export const DONUT_LAP_DURATION_PRESETS_SECONDS = [30, 60, 90, 120];
export const DONUT_MIN_LAP_DURATION_SECONDS = 10;
export const DONUT_MAX_LAP_DURATION_SECONDS = 600;
export const DONUT_COLORS = ["#65756a", "#6f6a83", "#7b6b5f", "#5f7380"];

/**
 * @param {number} value
 */
export function clampDonutLapDurationSeconds(value) {
  if (!Number.isFinite(value)) {
    return 60;
  }

  return Math.min(
    DONUT_MAX_LAP_DURATION_SECONDS,
    Math.max(DONUT_MIN_LAP_DURATION_SECONDS, Math.round(value))
  );
}

/**
 * @param {string | Date} sessionStartedAt
 * @param {string | Date | number} now
 * @param {number} lapDurationSeconds
 */
export function calculateDonutLap(sessionStartedAt, now, lapDurationSeconds = 60) {
  const startedMs = new Date(sessionStartedAt).getTime();
  const nowMs = typeof now === "number" ? now : new Date(now).getTime();
  const safeLapDuration = clampDonutLapDurationSeconds(lapDurationSeconds);
  const elapsedSeconds = Math.max(0, (nowMs - startedMs) / 1000);
  const lapIndex = Math.floor(elapsedSeconds / safeLapDuration);
  const progress = (elapsedSeconds % safeLapDuration) / safeLapDuration;

  return {
    elapsedSeconds,
    lapDurationSeconds: safeLapDuration,
    lapIndex,
    progress,
    color: DONUT_COLORS[lapIndex % DONUT_COLORS.length]
  };
}

/**
 * @param {{ task: { title: string, nextAction?: string | null } } | null} activeSession
 */
export function renderFloatingWindowSnapshot(activeSession) {
  if (!activeSession) {
    return ["No active task"];
  }

  return [
    activeSession.task.title,
    ...(activeSession.task.nextAction ? [activeSession.task.nextAction] : [])
  ];
}

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
