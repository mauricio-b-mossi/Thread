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
import {
  canStartTask,
  createEmptyTodayPayload,
  createQuickCaptureInput,
  createTodayViewModel,
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

const rendered = renderTodaySnapshot(payload);
assert.match(rendered.pickup[0], /Draft launch note/);
assert.match(rendered.pickup[0], /Write the first paragraph/);
assert.match(rendered.recentThreads[0], /Outlined the launch sections/);
assert.match(rendered.backlog[0], /Rework onboarding/);

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

console.log("Today window behavior tests passed.");
