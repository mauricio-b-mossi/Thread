/** @typedef {"today" | "floating"} EntryRoute */
/** @typedef {"settings" | "stop-current-task" | null} EntryIntent */

/**
 * @param {{ hash?: string, pathname?: string }} locationLike
 * @returns {{ route: EntryRoute, intent: EntryIntent }}
 */
export function parseEntryLocation(locationLike) {
  const rawRoute =
    (locationLike.hash || "").replace(/^#\/?/, "") ||
    (locationLike.pathname || "").replace(/^\//, "");

  if (rawRoute === "floating") {
    return { route: "floating", intent: null };
  }

  if (rawRoute === "settings") {
    return { route: "today", intent: "settings" };
  }

  if (rawRoute === "stop-current-task") {
    return { route: "today", intent: "stop-current-task" };
  }

  return { route: "today", intent: null };
}
