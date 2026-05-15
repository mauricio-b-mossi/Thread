import assert from "node:assert/strict";
import {
  calculateDonutLap,
  classifyFloatingWindowPointerGesture,
  DONUT_COLORS,
  DONUT_LAP_DURATION_PRESETS_SECONDS,
  FLOATING_WINDOW_DRAG_THRESHOLD_PX,
  isFloatingWindowDrag,
  pointerTravelDistance,
  renderFloatingWindowSnapshot,
  shouldOpenFloatingWindowMenuOnPointerUp
} from "../src/lib/floatingWindow.js";
import { parseEntryLocation } from "../src/lib/entryRoute.js";
import {
  getTodayKeyboardCommand,
  isEditableTarget,
  isInteractiveTarget,
  isStopFormTarget
} from "../src/lib/keyboard.js";
import {
  canStartTask,
  createEmptyTodayPayload,
  createQuickCaptureInput,
  createTaskDetailViewModel,
  createTodayViewModel,
  filterTodayRows,
  renderTaskDetailSnapshot,
  renderTodaySnapshot,
  todayEmptyStates
} from "../src/lib/todayView.js";

const baseTask = {
  id: "task-1",
  title: "Draft launch note",
  description: "",
  kind: "pickup",
  status: "pickup",
  priority: 2,
  pickupDate: "2026-05-09",
  nextAction: "Write the first paragraph",
  sortOrder: 10,
  createdAt: "2026-05-09T10:00:00.000Z",
  updatedAt: "2026-05-09T11:00:00.000Z",
  completedAt: null,
  archivedAt: null
};

const longTermTask = {
  ...baseTask,
  id: "task-2",
  title: "Rework onboarding",
  kind: "long_term",
  status: "backlog",
  pickupDate: null,
  nextAction: "Map the first-run path"
};

const emptySnapshot = renderTodaySnapshot(createEmptyTodayPayload());
assert.deepEqual(emptySnapshot.pickup, [todayEmptyStates.pickup]);
assert.deepEqual(emptySnapshot.recentThreads, [todayEmptyStates.recentThreads]);
assert.deepEqual(emptySnapshot.backlog, [todayEmptyStates.backlog]);

const pickupInput = createQuickCaptureInput("pickup", "  Send update  ", "  Add metrics  ");
assert.deepEqual(pickupInput, {
  title: "Send update",
  kind: "pickup",
  nextAction: "Add metrics"
});

const longTermInput = createQuickCaptureInput("long_term", "  Improve search  ");
assert.deepEqual(longTermInput, {
  title: "Improve search",
  kind: "long_term"
});

const payload = {
  activeSession: {
    task: {
      ...baseTask,
      id: "task-active",
      title: "Active migration",
      status: "active"
    },
    session: {
      id: "session-active",
      taskId: "task-active",
      startedAt: "2026-05-09T12:00:00.000Z",
      endedAt: null,
      durationSeconds: null,
      endReason: null,
      progressNote: null,
      nextAction: null,
      lapDurationSeconds: 60,
      recoveredFromCrash: false,
      createdAt: "2026-05-09T12:00:00.000Z",
      updatedAt: "2026-05-09T12:00:00.000Z"
    }
  },
  pickup: [baseTask],
  backlog: [longTermTask],
  recentThreads: [
    {
      task: baseTask,
      session: {
        id: "session-1",
        taskId: baseTask.id,
        startedAt: "2026-05-09T09:00:00.000Z",
        endedAt: "2026-05-09T09:30:00.000Z",
        durationSeconds: 1800,
        endReason: "stopped",
        progressNote: "Outlined the launch sections",
        nextAction: "Write the first paragraph",
        lapDurationSeconds: 60,
        recoveredFromCrash: false,
        createdAt: "2026-05-09T09:00:00.000Z",
        updatedAt: "2026-05-09T09:30:00.000Z"
      },
      lastWorkedAt: "2026-05-09T09:30:00.000Z",
      progressNote: "Outlined the launch sections",
      nextAction: "Write the first paragraph",
      durationSeconds: 1800
    }
  ]
};

const model = createTodayViewModel(payload);
assert.equal(model.active.title, "Active migration");
assert.equal(model.pickup.rows[0].title, "Draft launch note");
assert.equal(model.pickup.rows[0].nextAction, "Write the first paragraph");
assert.equal(model.recentThreads.rows[0].progressNote, "Outlined the launch sections");
assert.equal(model.backlog.rows[0].title, "Rework onboarding");
assert.equal(canStartTask(baseTask), true);
assert.equal(canStartTask(payload.activeSession.task), false);
assert.deepEqual(filterTodayRows(model.pickup.rows, "first paragraph"), model.pickup.rows);
assert.deepEqual(filterTodayRows(model.pickup.rows, "missing"), []);

const rendered = renderTodaySnapshot(payload);
assert.match(rendered.pickup[0], /Draft launch note/);
assert.match(rendered.pickup[0], /Write the first paragraph/);
assert.match(rendered.recentThreads[0], /Outlined the launch sections/);
assert.match(rendered.backlog[0], /Rework onboarding/);

const detailModel = createTaskDetailViewModel({
  task: {
    ...longTermTask,
    description: "Make the first-run path easier to resume"
  },
  totalDurationSeconds: 5400,
  sessions: [
    {
      id: "session-detail-2",
      taskId: longTermTask.id,
      startedAt: "2026-05-10T10:00:00.000Z",
      endedAt: "2026-05-10T11:00:00.000Z",
      durationSeconds: 3600,
      endReason: "stopped",
      progressNote: "Mapped the handoff flow",
      nextAction: "Sketch the recovery panel",
      lapDurationSeconds: 60,
      recoveredFromCrash: false,
      createdAt: "2026-05-10T10:00:00.000Z",
      updatedAt: "2026-05-10T11:00:00.000Z"
    },
    {
      id: "session-discarded",
      taskId: longTermTask.id,
      startedAt: "2026-05-10T12:00:00.000Z",
      endedAt: "2026-05-10T12:05:00.000Z",
      durationSeconds: 300,
      endReason: "discarded",
      progressNote: "Accidental open",
      nextAction: null,
      lapDurationSeconds: 60,
      recoveredFromCrash: true,
      createdAt: "2026-05-10T12:00:00.000Z",
      updatedAt: "2026-05-10T12:05:00.000Z"
    }
  ]
});
assert.equal(detailModel.title, "Rework onboarding");
assert.equal(detailModel.kind, "Long-term");
assert.equal(detailModel.status, "Backlog");
assert.equal(detailModel.totalDuration, "1h 30m");
assert.equal(detailModel.sessions[1].duration, "Discarded / excluded from total");
assert.equal(detailModel.progressNotes.length, 2);

const detailSnapshot = renderTaskDetailSnapshot({
  task: {
    ...longTermTask,
    description: "Make the first-run path easier to resume"
  },
  totalDurationSeconds: 5400,
  sessions: detailModel.sessions.map((row, index) => ({
    id: row.id,
    taskId: longTermTask.id,
    startedAt: index === 0 ? "2026-05-10T10:00:00.000Z" : "2026-05-10T12:00:00.000Z",
    endedAt: index === 0 ? "2026-05-10T11:00:00.000Z" : "2026-05-10T12:05:00.000Z",
    durationSeconds: index === 0 ? 3600 : 300,
    endReason: index === 0 ? "stopped" : "discarded",
    progressNote: row.progressNote,
    nextAction: row.nextAction,
    lapDurationSeconds: 60,
    recoveredFromCrash: index !== 0,
    createdAt: "2026-05-10T10:00:00.000Z",
    updatedAt: "2026-05-10T11:00:00.000Z"
  }))
});
assert(detailSnapshot.some((line) => /Sketch the recovery panel/.test(line)));
assert(detailSnapshot.some((line) => /excluded from total/.test(line)));

assert.equal(pointerTravelDistance({ x: 4, y: 8 }, { x: 7, y: 12 }), 5);
assert.equal(
  classifyFloatingWindowPointerGesture(
    { x: 100, y: 100 },
    { x: 100 + FLOATING_WINDOW_DRAG_THRESHOLD_PX, y: 100 }
  ),
  "click"
);
assert.equal(isFloatingWindowDrag({ x: 100, y: 100 }, { x: 107, y: 100 }), true);
assert.equal(
  classifyFloatingWindowPointerGesture({ x: 100, y: 100 }, { x: 107, y: 100 }),
  "drag"
);
assert.equal(
  shouldOpenFloatingWindowMenuOnPointerUp({ x: 100, y: 100 }, { x: 102, y: 100 }, false),
  true
);
assert.equal(
  shouldOpenFloatingWindowMenuOnPointerUp({ x: 100, y: 100 }, { x: 102, y: 100 }, true),
  false
);
assert.equal(
  shouldOpenFloatingWindowMenuOnPointerUp({ x: 100, y: 100 }, { x: 107, y: 100 }, false),
  false
);

assert.deepEqual(DONUT_LAP_DURATION_PRESETS_SECONDS, [30, 60, 90, 120]);

const firstLap = calculateDonutLap(
  "2026-05-09T12:00:00.000Z",
  "2026-05-09T12:00:15.000Z",
  60
);
assert.equal(firstLap.lapIndex, 0);
assert.equal(firstLap.progress, 0.25);
assert.equal(firstLap.color, DONUT_COLORS[0]);

const secondLap = calculateDonutLap(
  "2026-05-09T12:00:00.000Z",
  "2026-05-09T12:01:15.000Z",
  60
);
assert.equal(secondLap.lapIndex, 1);
assert.equal(secondLap.progress, 0.25);
assert.equal(secondLap.color, DONUT_COLORS[1]);

const wrappedPaletteLap = calculateDonutLap(
  "2026-05-09T12:00:00.000Z",
  "2026-05-09T12:04:30.000Z",
  60
);
assert.equal(wrappedPaletteLap.lapIndex, 4);
assert.equal(wrappedPaletteLap.progress, 0.5);
assert.equal(wrappedPaletteLap.color, DONUT_COLORS[0]);

const floatingSnapshot = renderFloatingWindowSnapshot(payload.activeSession);
assert.deepEqual(floatingSnapshot, ["Active migration", "Write the first paragraph"]);
assert(!floatingSnapshot.some((line) => /\d+:\d+|\b\d+\s*(s|sec|seconds|min|minutes)\b/i.test(line)));

assert.deepEqual(parseEntryLocation({ hash: "#/floating", pathname: "/" }), {
  route: "floating",
  intent: null
});
assert.deepEqual(parseEntryLocation({ hash: "#/settings", pathname: "/" }), {
  route: "today",
  intent: "settings"
});
assert.deepEqual(parseEntryLocation({ hash: "#/stop-current-task", pathname: "/" }), {
  route: "today",
  intent: "stop-current-task"
});
assert.deepEqual(parseEntryLocation({ hash: "", pathname: "/today" }), {
  route: "today",
  intent: null
});

assert.equal(isEditableTarget({ tagName: "INPUT" }), true);
assert.equal(isInteractiveTarget({ matches: (selector) => selector.includes("button") }), true);
assert.equal(isStopFormTarget({ closest: (selector) => selector === "[data-stop-form]" }), true);
assert.equal(getTodayKeyboardCommand({ key: "n" }), "new-task");
assert.equal(getTodayKeyboardCommand({ key: "/", target: { tagName: "INPUT" } }), null);
assert.equal(getTodayKeyboardCommand({ key: "/", target: { tagName: "DIV" } }), "filter");
assert.equal(getTodayKeyboardCommand({ key: "Enter" }), "start-selected");
assert.equal(getTodayKeyboardCommand({ key: "Enter", target: { tagName: "BUTTON" } }), null);
assert.equal(
  getTodayKeyboardCommand({
    key: "Enter",
    ctrlKey: true,
    target: { tagName: "TEXTAREA", closest: (selector) => selector === "[data-stop-form]" }
  }),
  "submit-stop"
);
assert.equal(getTodayKeyboardCommand({ key: "Enter", ctrlKey: true, target: { tagName: "TEXTAREA" } }), null);
assert.equal(getTodayKeyboardCommand({ key: ",", ctrlKey: true }), "settings");
assert.equal(getTodayKeyboardCommand({ key: "Escape", target: { tagName: "INPUT" } }), "escape");

console.log("Today window behavior tests passed.");
