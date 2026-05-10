<script lang="ts">
  import { onMount } from "svelte";
  import { createTask, listToday, startSession } from "./lib/commands";
  import type { CreateTaskInput, Task, TaskKind, TodayPayload } from "./lib/types";
  import {
    canStartTask,
    createEmptyTodayPayload,
    createQuickCaptureInput,
    createTodayViewModel
  } from "./lib/todayView.js";

  type EntryRoute = "today" | "floating";

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
  let feedbackMessage = $state("");
  let errorMessage = $state("");

  const todayView = $derived(createTodayViewModel(todayPayload));
  const hasActiveTask = $derived(Boolean(todayPayload.activeSession));

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
      todayPayload = await listToday();
    } catch (error) {
      errorMessage = getErrorMessage(error, "Today could not load.");
    } finally {
      loadingToday = false;
    }
  };

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
    if (!canStartTask(task) || hasActiveTask) {
      return;
    }

    feedbackMessage = "";
    errorMessage = "";
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

  const canUseStartButton = (canStart: boolean, taskId: string) =>
    canStart && !hasActiveTask && startingTaskId !== taskId;

  const startButtonLabel = (canStart: boolean, taskId: string) => {
    if (!canStart) {
      return "Closed";
    }

    if (startingTaskId === taskId) {
      return "Starting";
    }

    return hasActiveTask ? "Stop first" : "Start";
  };

  onMount(() => {
    void refreshToday();
  });
</script>

<svelte:window onhashchange={updateRoute} onpopstate={updateRoute} />

{#if route === "floating"}
  <main class="floating-shell" aria-label="Thread floating window">
    <div class="floating-bar">
      <span class="status-dot"></span>
      <span>Thread</span>
    </div>
    <p>Floating capture placeholder</p>
  </main>
{:else}
  <main class="today-shell" aria-label="Thread Today">
    <header class="today-header">
      <div>
        <p class="app-name">Thread</p>
        <h1>Today</h1>
      </div>
      <div class="header-actions">
        <time datetime={todayIso}>{todayLabel}</time>
        <button class="quiet-button" type="button" aria-label="Settings">Settings</button>
      </div>
    </header>

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
        <div>
          <p class="section-kicker">Active now</p>
          <h2>{todayView.active.title}</h2>
          {#if todayView.active.nextAction}
            <p class="next-action">{todayView.active.nextAction}</p>
          {/if}
          <p class="metadata">{todayView.active.metadata}</p>
        </div>
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
