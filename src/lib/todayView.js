/** @typedef {import("./types").ActiveSession} ActiveSession */
/** @typedef {import("./types").CreateTaskInput} CreateTaskInput */
/** @typedef {import("./types").RecentThread} RecentThread */
/** @typedef {import("./types").Task} Task */
/** @typedef {import("./types").TaskDetail} TaskDetail */
/** @typedef {import("./types").TaskKind} TaskKind */
/** @typedef {import("./types").TodayPayload} TodayPayload */

export const todayEmptyStates = Object.freeze({
  pickup: "Nothing queued for pickup",
  recentThreads: "No recent sessions yet",
  backlog: "No long-term tasks yet"
});

/**
 * @returns {TodayPayload}
 */
export function createEmptyTodayPayload() {
  return {
    activeSession: null,
    pickup: [],
    backlog: [],
    recentThreads: []
  };
}

/**
 * @param {TaskKind} kind
 * @param {string} title
 * @param {string} [nextAction]
 * @returns {CreateTaskInput}
 */
export function createQuickCaptureInput(kind, title, nextAction = "") {
  if (kind !== "pickup" && kind !== "long_term") {
    throw new Error(`Unsupported task kind: ${kind}`);
  }

  const normalizedTitle = title.trim();
  if (!normalizedTitle) {
    throw new Error("Task title is required.");
  }

  const normalizedNextAction = nextAction.trim();
  return {
    title: normalizedTitle,
    kind,
    ...(normalizedNextAction ? { nextAction: normalizedNextAction } : {})
  };
}

/**
 * @param {Task} task
 * @returns {boolean}
 */
export function canStartTask(task) {
  return task.status === "pickup" || task.status === "backlog";
}

/**
 * @param {TodayPayload} payload
 */
export function createTodayViewModel(payload) {
  return {
    active: payload.activeSession ? createActiveRow(payload.activeSession) : null,
    pickup: {
      label: "Pickup",
      emptyText: todayEmptyStates.pickup,
      rows: payload.pickup.map(createTaskRow)
    },
    recentThreads: {
      label: "Recent Threads",
      emptyText: todayEmptyStates.recentThreads,
      rows: payload.recentThreads.map(createRecentThreadRow)
    },
    backlog: {
      label: "Backlog",
      emptyText: todayEmptyStates.backlog,
      rows: payload.backlog.map(createTaskRow)
    }
  };
}

/**
 * @param {TodayPayload} payload
 */
export function renderTodaySnapshot(payload) {
  const model = createTodayViewModel(payload);

  return {
    active: model.active ? [model.active.title, model.active.metadata] : [],
    pickup: renderRowsOrEmpty(
      model.pickup.rows,
      model.pickup.emptyText,
      (row) => [row.title, row.nextAction, row.metadata].filter(Boolean).join(" | ")
    ),
    recentThreads: renderRowsOrEmpty(
      model.recentThreads.rows,
      model.recentThreads.emptyText,
      (row) => [row.title, row.progressNote, row.nextAction, row.metadata]
        .filter(Boolean)
        .join(" | ")
    ),
    backlog: renderRowsOrEmpty(
      model.backlog.rows,
      model.backlog.emptyText,
      (row) => [row.title, row.nextAction, row.metadata].filter(Boolean).join(" | ")
    )
  };
}

/**
 * @param {TaskDetail} detail
 */
export function createTaskDetailViewModel(detail) {
  const sessions = detail.sessions.map(createSessionHistoryRow);
  const progressNotes = sessions.filter((session) => session.progressNote);

  return {
    task: detail.task,
    title: detail.task.title,
    description: detail.task.description,
    kind: detail.task.kind === "long_term" ? "Long-term" : "Pickup",
    status: formatStatus(detail.task.status),
    nextAction: detail.task.nextAction,
    totalDuration: formatDuration(detail.totalDurationSeconds) ?? "0m",
    sessions,
    progressNotes
  };
}

/**
 * @param {TaskDetail} detail
 */
export function renderTaskDetailSnapshot(detail) {
  const model = createTaskDetailViewModel(detail);

  return [
    model.title,
    model.description,
    model.kind,
    model.status,
    model.nextAction,
    model.totalDuration,
    ...model.sessions.map((session) =>
      [
        session.when,
        session.status,
        session.duration,
        session.progressNote,
        session.nextAction
      ]
        .filter(Boolean)
        .join(" | ")
    )
  ].filter(Boolean);
}

/**
 * @param {Task} task
 */
function createTaskRow(task) {
  return {
    id: task.id,
    task,
    title: task.title,
    nextAction: task.nextAction,
    metadata: formatTaskMetadata(task),
    canStart: canStartTask(task)
  };
}

/**
 * @param {ActiveSession} activeSession
 */
function createActiveRow(activeSession) {
  return {
    id: activeSession.task.id,
    title: activeSession.task.title,
    nextAction: activeSession.task.nextAction,
    metadata: `Active since ${formatShortDateTime(activeSession.session.startedAt)}`
  };
}

/**
 * @param {RecentThread} thread
 */
function createRecentThreadRow(thread) {
  return {
    id: thread.session.id,
    task: thread.task,
    title: thread.task.title,
    progressNote: thread.progressNote,
    nextAction: thread.nextAction,
    metadata: formatRecentThreadMetadata(thread),
    canStart: canStartTask(thread.task)
  };
}

/**
 * @param {import("./types").Session} session
 */
function createSessionHistoryRow(session) {
  return {
    id: session.id,
    when: session.endedAt
      ? `Ended ${formatShortDateTime(session.endedAt)}`
      : `Started ${formatShortDateTime(session.startedAt)}`,
    status: formatEndReason(session.endReason),
    duration:
      session.endReason === "discarded"
        ? "Discarded / excluded from total"
        : formatDuration(session.durationSeconds),
    progressNote: session.progressNote,
    nextAction: session.nextAction
  };
}

/**
 * @param {Task} task
 */
function formatTaskMetadata(task) {
  const parts = [task.kind === "long_term" ? "Long-term" : "Pickup"];

  if (task.pickupDate) {
    parts.push(`Due ${formatShortDate(task.pickupDate)}`);
  }

  if (task.priority > 0) {
    parts.push(`Priority ${task.priority}`);
  }

  parts.push(`Updated ${formatShortDateTime(task.updatedAt)}`);
  return parts.join(" / ");
}

/**
 * @param {string} status
 */
function formatStatus(status) {
  return status
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

/**
 * @param {string | null} endReason
 */
function formatEndReason(endReason) {
  if (!endReason) {
    return "Active";
  }

  return formatStatus(endReason);
}

/**
 * @param {RecentThread} thread
 */
function formatRecentThreadMetadata(thread) {
  const parts = [`Worked ${formatRelativeDateTime(thread.lastWorkedAt)}`];
  const duration = formatDuration(thread.durationSeconds);

  if (duration) {
    parts.push(duration);
  }

  return parts.join(" / ");
}

/**
 * @template T
 * @param {T[]} rows
 * @param {string} emptyText
 * @param {(row: T) => string} renderRow
 */
function renderRowsOrEmpty(rows, emptyText, renderRow) {
  return rows.length > 0 ? rows.map(renderRow) : [emptyText];
}

/**
 * @param {string} value
 */
function formatRelativeDateTime(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  const now = new Date();
  const localDay = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const dayDelta = Math.round((today - localDay) / 86400000);

  if (dayDelta === 0) {
    return "today";
  }

  if (dayDelta === 1) {
    return "yesterday";
  }

  if (dayDelta > 1 && dayDelta <= 30) {
    return `${dayDelta}d ago`;
  }

  return formatShortDateTime(value);
}

/**
 * @param {string} value
 */
function formatShortDate(value) {
  const date = new Date(`${value}T00:00:00`);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric"
  }).format(date);
}

/**
 * @param {string} value
 */
function formatShortDateTime(value) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }

  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric"
  }).format(date);
}

/**
 * @param {number | null} seconds
 */
function formatDuration(seconds) {
  if (!seconds || seconds <= 0) {
    return null;
  }

  if (seconds < 3600) {
    return `${Math.max(1, Math.round(seconds / 60))}m`;
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.round((seconds % 3600) / 60);
  return minutes > 0 ? `${hours}h ${minutes}m` : `${hours}h`;
}
