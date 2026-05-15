/**
 * @param {unknown} target
 */
export function isEditableTarget(target) {
  if (!target || typeof target !== "object") {
    return false;
  }

  const element = /** @type {{ tagName?: string, isContentEditable?: boolean }} */ (target);
  const tagName = element.tagName?.toLowerCase();
  return Boolean(
    element.isContentEditable ||
      tagName === "input" ||
      tagName === "select" ||
      tagName === "textarea"
  );
}

/**
 * @param {unknown} target
 * @param {string} selector
 */
function targetMatchesOrContains(target, selector) {
  if (!target || typeof target !== "object") {
    return false;
  }

  const element = /** @type {{ matches?: (selector: string) => boolean, closest?: (selector: string) => unknown }} */ (target);
  return Boolean(element.matches?.(selector) || element.closest?.(selector));
}

/**
 * @param {unknown} target
 */
export function isInteractiveTarget(target) {
  if (target && typeof target === "object") {
    const tagName = /** @type {{ tagName?: string }} */ (target).tagName?.toLowerCase();
    if (tagName === "button" || tagName === "a" || tagName === "input" || tagName === "select" || tagName === "textarea" || tagName === "summary") {
      return true;
    }
  }

  return targetMatchesOrContains(
    target,
    'button, a[href], input, select, textarea, summary, [contenteditable="true"], [role="button"], [role="link"], [role="menuitem"], [role="checkbox"], [role="radio"], [aria-disabled="true"], [disabled]'
  );
}

/**
 * @param {unknown} target
 */
export function isStopFormTarget(target) {
  return targetMatchesOrContains(target, "[data-stop-form]");
}

/**
 * @param {{ key: string, ctrlKey?: boolean, metaKey?: boolean, altKey?: boolean, target?: unknown }} event
 */
export function getTodayKeyboardCommand(event) {
  const key = event.key.toLowerCase();
  const editable = isEditableTarget(event.target);
  const commandModifier = Boolean(event.ctrlKey || event.metaKey);

  if (key === "escape") {
    return "escape";
  }

  if (commandModifier && key === ",") {
    return "settings";
  }

  if (commandModifier && key === "enter" && isStopFormTarget(event.target)) {
    return "submit-stop";
  }

  if (editable || event.altKey || commandModifier) {
    return null;
  }

  if (key === "n") {
    return "new-task";
  }

  if (key === "/") {
    return "filter";
  }

  if (key === "enter" && !isInteractiveTarget(event.target)) {
    return "start-selected";
  }

  return null;
}
