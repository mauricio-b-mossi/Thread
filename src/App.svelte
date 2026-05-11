<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import {
    completeSession,
    createTask,
    getPendingSessionRecovery,
    getSettings,
    listToday,
    openTodayWindow,
    resolveSessionRecovery,
    saveFloatingWindowPosition,
    startSession,
    stopSession,
    switchTask,
    updateSettings
  } from "./lib/commands";
  import {
    calculateDonutLap,
    clampDonutLapDurationSeconds,
    DONUT_LAP_DURATION_PRESETS_SECONDS,
    DONUT_MAX_LAP_DURATION_SECONDS,
    DONUT_MIN_LAP_DURATION_SECONDS,
    isFloatingWindowDrag,
    shouldOpenFloatingWindowMenuOnPointerUp
  } from "./lib/floatingWindow.js";
  import type {
    ActiveSession,
    CreateTaskInput,
    RecoveryAction,
    Settings,
    Task,
    TaskKind,
    TodayPayload
  } from "./lib/types";
  import {
    canStartTask,
    createEmptyTodayPayload,
    createQuickCaptureInput,
    createTodayViewModel
  } from "./lib/todayView.js";

  type EntryRoute = "today" | "floating";
  type LifecycleAction = "complete" | "stop" | "switch";
  type SessionDestination = "pickup" | "backlog";

  const getRoute = (): EntryRoute => {
    if (typeof window === "undefined") {
      return "today";
    }

    const route =
      window.location.hash.replace(/^#\/?/, "") || window.location.pathname.replace(/^\//, "");
    return route === "floating" ? "floating" : "today";
  };

  const today = new Date();
  const todayIso = [
    today.getFullYear(),
    String(today.getMonth() + 1).padStart(2, "0"),
    String(today.getDate()).padStart(2, "0")
  ].join("-");
  const todayLabel = new Intl.DateTimeFormat(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric"
  }).format(today);

  let route = $state<EntryRoute>(getRoute());
  let todayPayload = $state<TodayPayload>(createEmptyTodayPayload());
  let loadingToday = $state(false);
  let captureKind = $state<TaskKind>("pickup");
  let captureTitle = $state("");
  let captureNextAction = $state("");
  let creatingTask = $state(false);
  let startingTaskId = $state<string | null>(null);
  let endingAction = $state<LifecycleAction | null>(null);
  let switchingTaskId = $state<string | null>(null);
  let activeLifecycleTaskId = $state<string | null>(null);
  let sessionProgressNote = $state("");
  let sessionNextAction = $state("");
  let sessionDestination = $state<SessionDestination>("pickup");
  let confirmLongTermCompletion = $state(false);
  let feedbackMessage = $state("");
  let errorMessage = $state("");
  let settings = $state<Settings | null>(null);
  let settingsOpen = $state(false);
  let savingSettings = $state(false);
  let customLapDuration = $state("60");
  let floatingMenuOpen = $state(false);
  let floatingPointerStart = $state<{ x: number; y: number } | null>(null);
  let floatingDragStarted = $state(false);
  let donutNowMs = $state(Date.now());
  let recoverySession = $state<ActiveSession | null>(null);
  let recoveryProgressNote = $state("");
  let recoveryNextAction = $state("");
  let resolvingRecoveryAction = $state<RecoveryAction | null>(null);

  const todayView = $derived(createTodayViewModel(todayPayload));
  const activeSession = $derived(todayPayload.activeSession);
  const hasActiveTask = $derived(Boolean(activeSession));
  const activeTaskIsLongTerm = $derived(activeSession?.task.kind === "long_term");
  const selectedLapDuration = $derived(settings?.donutLapDurationSeconds ?? 60);
  const donutLap = $derived(
    activeSession
      ? calculateDonutLap(
          activeSession.session.startedAt,
          donutNowMs,
          activeSession.session.lapDurationSeconds || selectedLapDuration
        )
      : null
  );
  const donutRadius = 18;

  const updateRoute = () => {
    route = getRoute();
  };

  const getErrorMessage = (error: unknown, fallback: string) => {
    if (typeof error === "string") {
      return error;
    }

    if (error && typeof error === "object" && "message" in error) {
      return String((error as { message: unknown }).message);
    }

    return fallback;
  };

  const refreshToday = async () => {
    loadingToday = true;
    errorMessage = "";

    try {
      const payload = await listToday();
      todayPayload = payload;
      return payload;
    } catch (error) {
      errorMessage = getErrorMessage(error, "Today could not load.");
      return null;
    } finally {
      loadingToday = false;
    }
  };

  const initializeApp = async () => {
    await refreshToday();
    void refreshSettings();

    if (route === "today") {
      try {
        const pendingRecovery = await getPendingSessionRecovery();
        recoverySession = pendingRecovery.activeSession;
      } catch (error) {
        errorMessage = getErrorMessage(error, "Session recovery could not load.");
      }
    }
  };

  const refreshSettings = async () => {
    try {
      settings = await getSettings();
      customLapDuration = String(settings.donutLapDurationSeconds);
    } catch (error) {
      errorMessage = getErrorMessage(error, "Settings could not load.");
    }
  };

  const saveLapDuration = async (value: number) => {
    feedbackMessage = "";
    errorMessage = "";
    savingSettings = true;

    try {
      const nextDuration = clampDonutLapDurationSeconds(value);
      settings = await updateSettings({ donutLapDurationSeconds: nextDuration });
      customLapDuration = String(settings.donutLapDurationSeconds);
      feedbackMessage = "Settings updated.";
    } catch (error) {
      errorMessage = getErrorMessage(error, "Settings could not be updated.");
    } finally {
      savingSettings = false;
    }
  };

  const saveCustomLapDuration = () => {
    void saveLapDuration(Number(customLapDuration));
  };

  const defaultSessionDestination = (task?: Task): SessionDestination =>
    task?.kind === "long_term" ? "backlog" : "pickup";

  const resetLifecycleInputs = (task?: Task) => {
    sessionProgressNote = "";
    sessionNextAction = "";
    sessionDestination = defaultSessionDestination(task);
    confirmLongTermCompletion = false;
  };

  $effect(() => {
    const activeTask = activeSession?.task;
    const activeTaskId = activeTask?.id ?? null;

    if (activeTaskId !== activeLifecycleTaskId) {
      activeLifecycleTaskId = activeTaskId;
      resetLifecycleInputs(activeTask);
    }
  });

  const handleCaptureSubmit = (event: SubmitEvent) => {
    event.preventDefault();
    void addCapturedTask();
  };

  const addCapturedTask = async () => {
    feedbackMessage = "";
    errorMessage = "";

    let input: CreateTaskInput;
    try {
      input = createQuickCaptureInput(captureKind, captureTitle, captureNextAction);
    } catch (error) {
      errorMessage = getErrorMessage(error, "Add a task title.");
      return;
    }

    creatingTask = true;
    try {
      await createTask(input);
      captureTitle = "";
      captureNextAction = "";
      feedbackMessage = captureKind === "pickup" ? "Pickup captured." : "Long-term task captured.";
      await refreshToday();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Task could not be created.");
    } finally {
      creatingTask = false;
    }
  };

  const beginTask = async (task: Task) => {
    if (!canStartTask(task)) {
      return;
    }

    feedbackMessage = "";
    errorMessage = "";

    if (activeSession) {
      if (activeSession.task.id === task.id) {
        return;
      }

      if (!validateLifecycleInput("switch")) {
        return;
      }

      switchingTaskId = task.id;

      try {
        await switchTask({
          taskId: task.id,
          progressNote: sessionProgressNote,
          nextAction: sessionNextAction,
          destinationStatus: sessionDestination
        });
        feedbackMessage = `Switched to ${task.title}.`;
        resetLifecycleInputs();
        await refreshToday();
      } catch (error) {
        errorMessage = getErrorMessage(error, "Task could not be switched.");
      } finally {
        switchingTaskId = null;
      }
      return;
    }

    startingTaskId = task.id;

    try {
      await startSession({ taskId: task.id });
      feedbackMessage = `${task.title} started.`;
      await refreshToday();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Task could not be started.");
    } finally {
      startingTaskId = null;
    }
  };

  const validateLifecycleInput = (action: LifecycleAction) => {
    if (!activeSession) {
      errorMessage = "No active session.";
      return false;
    }

    if (activeTaskIsLongTerm && (action === "stop" || action === "switch")) {
      if (!sessionProgressNote.trim()) {
        errorMessage = "Add a progress note before stopping or switching a long-term task.";
        return false;
      }

      if (!sessionNextAction.trim()) {
        errorMessage = "Add a next action before stopping or switching a long-term task.";
        return false;
      }
    }

    if (activeTaskIsLongTerm && action === "complete" && !confirmLongTermCompletion) {
      errorMessage = "Confirm completion before completing a long-term task.";
      return false;
    }

    return true;
  };

  const endActiveSession = async (action: Exclude<LifecycleAction, "switch">) => {
    feedbackMessage = "";
    errorMessage = "";

    if (!validateLifecycleInput(action)) {
      return;
    }

    endingAction = action;

    try {
      if (action === "complete") {
        await completeSession({
          sessionId: activeSession?.session.id,
          progressNote: sessionProgressNote,
          nextAction: sessionNextAction,
          confirmLongTermCompletion
        });
        feedbackMessage = "Session completed.";
      } else {
        await stopSession({
          sessionId: activeSession?.session.id,
          progressNote: sessionProgressNote,
          nextAction: sessionNextAction,
          destinationStatus: sessionDestination
        });
        feedbackMessage = "Session stopped.";
      }

      resetLifecycleInputs();
      await refreshToday();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Session could not be updated.");
    } finally {
      endingAction = null;
    }
  };

  const canUseStartButton = (canStart: boolean, taskId: string) =>
    canStart &&
    activeSession?.task.id !== taskId &&
    startingTaskId !== taskId &&
    switchingTaskId !== taskId &&
    endingAction === null;

  const startButtonLabel = (canStart: boolean, taskId: string) => {
    if (!canStart) {
      return "Closed";
    }

    if (switchingTaskId === taskId) {
      return "Switching";
    }

    if (startingTaskId === taskId) {
      return "Starting";
    }

    if (activeSession?.task.id === taskId) {
      return "Active";
    }

    return hasActiveTask ? "Switch" : "Start";
  };

  const showTodayWindow = async () => {
    floatingMenuOpen = false;
    errorMessage = "";

    try {
      await openTodayWindow();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Today could not be opened.");
    }
  };

  const completeFromFloatingMenu = async () => {
    floatingMenuOpen = false;

    if (activeTaskIsLongTerm) {
      await showTodayWindow();
      return;
    }

    await endActiveSession("complete");
  };

  const resolveRecovery = async (action: RecoveryAction) => {
    if (!recoverySession) {
      return;
    }

    feedbackMessage = "";
    errorMessage = "";

    if (action === "stop" && !recoveryProgressNote.trim()) {
      errorMessage = "Add a recovery note before stopping the previous session.";
      return;
    }

    resolvingRecoveryAction = action;

    try {
      await resolveSessionRecovery({
        action,
        sessionId: recoverySession.session.id,
        progressNote: recoveryProgressNote,
        nextAction: recoveryNextAction
      });
      recoverySession = null;
      recoveryProgressNote = "";
      recoveryNextAction = "";
      feedbackMessage =
        action === "resume"
          ? "Session resumed."
          : action === "discard"
            ? "Session discarded."
            : "Session stopped.";
      await refreshToday();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Session recovery could not be resolved.");
    } finally {
      resolvingRecoveryAction = null;
    }
  };

  const handleFloatingPointerDown = (event: PointerEvent) => {
    if (event.button !== 0) {
      return;
    }

    floatingPointerStart = { x: event.clientX, y: event.clientY };
    floatingDragStarted = false;
  };

  const handleFloatingPointerMove = (event: PointerEvent) => {
    if (!floatingPointerStart || floatingDragStarted) {
      return;
    }

    if (!isFloatingWindowDrag(floatingPointerStart, { x: event.clientX, y: event.clientY })) {
      return;
    }

    floatingDragStarted = true;
    floatingMenuOpen = false;
    void getCurrentWindow().startDragging();
  };

  const handleFloatingPointerUp = (event: PointerEvent) => {
    if (!floatingPointerStart) {
      return;
    }

    const pointerEnd = { x: event.clientX, y: event.clientY };
    const wasDragging = floatingDragStarted;
    const shouldOpenMenu = shouldOpenFloatingWindowMenuOnPointerUp(
      floatingPointerStart,
      pointerEnd,
      wasDragging
    );
    floatingPointerStart = null;

    if (wasDragging) {
      floatingDragStarted = false;
      return;
    }

    if (shouldOpenMenu) {
      floatingMenuOpen = !floatingMenuOpen;
    }

    floatingDragStarted = false;
  };

  onMount(() => {
    void initializeApp();

    if (route !== "floating") {
      let removeSessionListener: (() => void) | null = null;

      void listen("session-changed", () => {
        void refreshToday();
      }).then((unlisten) => {
        removeSessionListener = unlisten;
      });

      return () => {
        removeSessionListener?.();
      };
    }

    const appWindow = getCurrentWindow();
    let animationFrame = 0;
    let savePositionTimer: ReturnType<typeof setTimeout> | null = null;
    let removeMoveListener: (() => void) | null = null;
    let removeSessionListener: (() => void) | null = null;

    const tickDonut = () => {
      donutNowMs = Date.now();
      animationFrame = requestAnimationFrame(tickDonut);
    };
    animationFrame = requestAnimationFrame(tickDonut);

    void appWindow.onMoved(({ payload }) => {
      if (savePositionTimer) {
        clearTimeout(savePositionTimer);
      }

      savePositionTimer = setTimeout(() => {
        void saveFloatingWindowPosition({ position: { x: payload.x, y: payload.y } });
      }, 200);
    }).then((unlisten) => {
      removeMoveListener = unlisten;
    });

    void listen("session-changed", () => {
      void refreshToday();
    }).then((unlisten) => {
      removeSessionListener = unlisten;
    });

    return () => {
      if (savePositionTimer) {
        clearTimeout(savePositionTimer);
      }

      cancelAnimationFrame(animationFrame);
      removeMoveListener?.();
      removeSessionListener?.();
    };
  });
</script>

<svelte:window onhashchange={updateRoute} onpopstate={updateRoute} />

{#if route === "floating"}
  <main
    class="floating-shell"
    aria-label="Thread floating window"
    onpointerdown={handleFloatingPointerDown}
    onpointermove={handleFloatingPointerMove}
    onpointerup={handleFloatingPointerUp}
    onpointercancel={() => {
      floatingPointerStart = null;
      floatingDragStarted = false;
    }}
  >
    <div class="floating-bar">
      <span class="status-dot"></span>
      <span>Thread</span>
    </div>
    {#if todayView.active}
      <section class="floating-task" aria-label="Active task">
        <div class="floating-copy">
          <h1>{todayView.active.title}</h1>
          {#if todayView.active.nextAction}
            <p>{todayView.active.nextAction}</p>
          {/if}
        </div>
        <div class="donut-area" aria-label="Focus donut">
          {#if donutLap}
            <svg class="donut-svg" viewBox="0 0 48 48" aria-hidden="true">
              <circle class="donut-track" cx="24" cy="24" r={donutRadius}></circle>
              <circle
                class="donut-progress"
                cx="24"
                cy="24"
                r={donutRadius}
                pathLength="1"
                stroke={donutLap.color}
                stroke-dasharray={`${donutLap.progress} ${1 - donutLap.progress}`}
              ></circle>
            </svg>
          {/if}
        </div>
      </section>
    {:else}
      <p class="floating-empty">No active task</p>
    {/if}
    {#if floatingMenuOpen}
      <div class="floating-menu" role="menu" aria-label="Floating task actions">
        <button type="button" role="menuitem" onclick={() => (floatingMenuOpen = false)}>
          Continue
        </button>
        <button
          type="button"
          role="menuitem"
          disabled={!activeSession || endingAction !== null}
          onclick={() => void completeFromFloatingMenu()}
        >
          Complete
        </button>
        <button
          type="button"
          role="menuitem"
          disabled={!activeSession}
          onclick={() => void showTodayWindow()}
        >
          Stop/Pause
        </button>
        <button
          type="button"
          role="menuitem"
          disabled={!activeSession}
          onclick={() => void showTodayWindow()}
        >
          Switch Task
        </button>
        <button type="button" role="menuitem" onclick={() => void showTodayWindow()}>
          Open Today
        </button>
      </div>
    {/if}
  </main>
{:else}
  <main class="today-shell" aria-label="Thread Today">
    {#if recoverySession}
      <div class="recovery-panel" role="dialog" aria-modal="true" aria-label="Recover session">
        <div class="recovery-card">
          <div class="active-summary">
            <p class="section-kicker">Recovery</p>
            <h2>{recoverySession.task.title}</h2>
            <p class="metadata">Unfinished session from {recoverySession.session.startedAt}</p>
          </div>
          <label>
            <span>Recovery note</span>
            <textarea bind:value={recoveryProgressNote} rows="3"></textarea>
          </label>
          <label>
            <span>Next action</span>
            <input bind:value={recoveryNextAction} autocomplete="off" />
          </label>
          <div class="session-actions">
            <button
              type="button"
              disabled={resolvingRecoveryAction !== null}
              onclick={() => void resolveRecovery("resume")}
            >
              {resolvingRecoveryAction === "resume" ? "Resuming" : "Resume"}
            </button>
            <button
              class="quiet-button"
              type="button"
              disabled={resolvingRecoveryAction !== null}
              onclick={() => void resolveRecovery("stop")}
            >
              {resolvingRecoveryAction === "stop" ? "Stopping" : "Stop and write note"}
            </button>
            <button
              class="quiet-button"
              type="button"
              disabled={resolvingRecoveryAction !== null}
              onclick={() => void resolveRecovery("discard")}
            >
              {resolvingRecoveryAction === "discard" ? "Discarding" : "Discard"}
            </button>
          </div>
        </div>
      </div>
    {/if}

    <header class="today-header">
      <div>
        <p class="app-name">Thread</p>
        <h1>Today</h1>
      </div>
      <div class="header-actions">
        <time datetime={todayIso}>{todayLabel}</time>
        <button
          class="quiet-button"
          type="button"
          aria-label="Settings"
          aria-expanded={settingsOpen}
          onclick={() => (settingsOpen = !settingsOpen)}
        >
          Settings
        </button>
      </div>
    </header>

    {#if settingsOpen}
      <section class="settings-panel" aria-label="Settings">
        <div>
          <p class="section-kicker">Donut lap</p>
          <div class="lap-presets" role="group" aria-label="Preset lap durations">
            {#each DONUT_LAP_DURATION_PRESETS_SECONDS as duration}
              <button
                class:active={selectedLapDuration === duration}
                type="button"
                disabled={savingSettings}
                aria-pressed={selectedLapDuration === duration}
                onclick={() => void saveLapDuration(duration)}
              >
                {duration}s
              </button>
            {/each}
          </div>
        </div>
        <label>
          <span>Custom seconds</span>
          <input
            bind:value={customLapDuration}
            type="number"
            min={DONUT_MIN_LAP_DURATION_SECONDS}
            max={DONUT_MAX_LAP_DURATION_SECONDS}
            step="1"
            disabled={savingSettings}
            onblur={saveCustomLapDuration}
            onchange={saveCustomLapDuration}
          />
        </label>
      </section>
    {/if}

    {#if loadingToday}
      <p class="status-line" aria-live="polite">Loading Today</p>
    {/if}

    {#if errorMessage}
      <p class="status-line error" role="alert">{errorMessage}</p>
    {:else if feedbackMessage}
      <p class="status-line" aria-live="polite">{feedbackMessage}</p>
    {/if}

    {#if todayView.active}
      <section class="active-thread" aria-label="Active work">
        <div class="active-summary">
          <p class="section-kicker">Active now</p>
          <h2>{todayView.active.title}</h2>
          {#if todayView.active.nextAction}
            <p class="next-action">{todayView.active.nextAction}</p>
          {/if}
          <p class="metadata">{todayView.active.metadata}</p>
        </div>
        <form class="session-controls" onsubmit={(event) => event.preventDefault()}>
          <label>
            <span>Progress note{activeTaskIsLongTerm ? " required to stop or switch" : ""}</span>
            <textarea bind:value={sessionProgressNote} name="progress-note" rows="2"></textarea>
          </label>
          <label>
            <span>Next action{activeTaskIsLongTerm ? " required to stop or switch" : ""}</span>
            <input bind:value={sessionNextAction} name="session-next-action" autocomplete="off" />
          </label>
          <label>
            <span>Return to</span>
            <select bind:value={sessionDestination} name="destination-status">
              <option value="pickup">Pickup</option>
              <option value="backlog">Backlog</option>
            </select>
          </label>
          {#if activeTaskIsLongTerm}
            <label class="checkbox-label">
              <input bind:checked={confirmLongTermCompletion} type="checkbox" />
              <span>Confirm long-term task is complete</span>
            </label>
          {/if}
          <div class="session-actions">
            <button
              class="quiet-button"
              type="button"
              disabled={endingAction !== null || switchingTaskId !== null}
              onclick={() => void endActiveSession("stop")}
            >
              {endingAction === "stop" ? "Stopping" : "Stop"}
            </button>
            <button
              type="button"
              disabled={endingAction !== null || switchingTaskId !== null}
              onclick={() => void endActiveSession("complete")}
            >
              {endingAction === "complete" ? "Completing" : "Complete"}
            </button>
          </div>
        </form>
      </section>
    {/if}

    <section class="today-section" aria-labelledby="pickup-heading">
      <div class="section-heading">
        <h2 id="pickup-heading">Pickup</h2>
        <span>{todayView.pickup.rows.length}</span>
      </div>

      {#if todayView.pickup.rows.length === 0}
        <p class="empty-state">{todayView.pickup.emptyText}</p>
      {:else}
        <ul class="task-list">
          {#each todayView.pickup.rows as row (row.id)}
            <li class="task-row">
              <div class="task-copy">
                <h3>{row.title}</h3>
                {#if row.nextAction}
                  <p class="next-action">{row.nextAction}</p>
                {/if}
                <p class="metadata">{row.metadata}</p>
              </div>
              <button
                class="row-action"
                type="button"
                disabled={!canUseStartButton(row.canStart, row.id)}
                onclick={() => void beginTask(row.task)}
              >
                {startButtonLabel(row.canStart, row.id)}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="today-section" aria-labelledby="recent-heading">
      <div class="section-heading">
        <h2 id="recent-heading">Recent Threads</h2>
        <span>{todayView.recentThreads.rows.length}</span>
      </div>

      {#if todayView.recentThreads.rows.length === 0}
        <p class="empty-state">{todayView.recentThreads.emptyText}</p>
      {:else}
        <ul class="task-list">
          {#each todayView.recentThreads.rows as row (row.id)}
            <li class="task-row">
              <div class="task-copy">
                <h3>{row.title}</h3>
                {#if row.progressNote}
                  <p class="progress-note">{row.progressNote}</p>
                {/if}
                {#if row.nextAction}
                  <p class="next-action">{row.nextAction}</p>
                {/if}
                <p class="metadata">{row.metadata}</p>
              </div>
              <button
                class="row-action"
                type="button"
                disabled={!canUseStartButton(row.canStart, row.task.id)}
                onclick={() => void beginTask(row.task)}
              >
                {startButtonLabel(row.canStart, row.task.id)}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="today-section" aria-labelledby="backlog-heading">
      <div class="section-heading">
        <h2 id="backlog-heading">Backlog</h2>
        <span>{todayView.backlog.rows.length}</span>
      </div>

      {#if todayView.backlog.rows.length === 0}
        <p class="empty-state">{todayView.backlog.emptyText}</p>
      {:else}
        <ul class="task-list">
          {#each todayView.backlog.rows as row (row.id)}
            <li class="task-row">
              <div class="task-copy">
                <h3>{row.title}</h3>
                {#if row.nextAction}
                  <p class="next-action">{row.nextAction}</p>
                {/if}
                <p class="metadata">{row.metadata}</p>
              </div>
              <button
                class="row-action"
                type="button"
                disabled={!canUseStartButton(row.canStart, row.id)}
                onclick={() => void beginTask(row.task)}
              >
                {startButtonLabel(row.canStart, row.id)}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="quick-capture-section" aria-labelledby="quick-capture-heading">
      <div class="section-heading">
        <h2 id="quick-capture-heading">Quick Capture</h2>
      </div>
      <form class="quick-capture" onsubmit={handleCaptureSubmit}>
        <div class="kind-toggle" role="group" aria-label="Task kind">
          <button
            type="button"
            class:active={captureKind === "pickup"}
            aria-pressed={captureKind === "pickup"}
            onclick={() => (captureKind = "pickup")}
          >
            Pickup
          </button>
          <button
            type="button"
            class:active={captureKind === "long_term"}
            aria-pressed={captureKind === "long_term"}
            onclick={() => (captureKind = "long_term")}
          >
            Long-term
          </button>
        </div>
        <label>
          <span>Title</span>
          <input bind:value={captureTitle} name="title" autocomplete="off" />
        </label>
        <label>
          <span>Next action</span>
          <input bind:value={captureNextAction} name="next-action" autocomplete="off" />
        </label>
        <button class="capture-submit" type="submit" disabled={creatingTask}>
          {creatingTask ? "Adding" : "Add"}
        </button>
      </form>
    </section>
  </main>
{/if}
