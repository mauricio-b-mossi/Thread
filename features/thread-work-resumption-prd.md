# Thread PRD: Local Work Resumption Desktop App

## 1. Product Overview

Thread is a Windows-first, local-first desktop app that helps the user resume work where they left off. It launches on PC startup, shows a fast Today view, lets the user select one task to resume or pursue, then hides while a small always-on-top floating task window remains visible on the desktop.

Thread is not a conventional todo app. It is an external working memory for preserving context across tired evenings, overnight breaks, interruptions, and task switching.

Core loop:

```text
Start PC
-> see Today's pickup context
-> choose one pickup or backlog task
-> Today window hides
-> floating task window appears on top of everything
-> donut slowly loops as a proxy for active pursuit
-> user stops or completes
-> app captures progress and next action
-> tomorrow's context is preserved
```

The v1 product must include:

- startup Today window
- short-term Pickup queue
- long-term Backlog
- Recent Threads section
- active session tracking
- floating always-on-top task window
- donut proxy for active time
- no explicit active timer while working
- stop and completion flows
- progress notes
- next actions
- local SQLite persistence
- startup registration
- crash and unfinished-session recovery
- minimal Notion-inspired visual design

## 2. Goals

### 2.1 Primary Goal

Make it extremely easy for the user to remember and resume the most relevant work thread immediately after boot or after returning to the PC.

### 2.2 Product Goals

- Replace slow cloud todo startup with a fast local app.
- Distinguish short-term resumption work from durable long-term backlog work.
- Keep one active task visible without requiring the main app to stay open.
- Preserve the exact context needed to restart a task.
- Avoid clutter accumulation from unchecked or stale todos.
- Track active work sessions locally for later review.
- Provide a quiet visual sense of ongoing pursuit through a donut, not a numeric timer.

### 2.3 Non-Goals For v1

- No account system.
- No cloud sync.
- No external todo integrations.
- No collaborative features.
- No mobile app.
- No AI categorization or LLM summaries.
- No gamified productivity score.
- No streaks.
- No notification-heavy reminder system.
- No calendar integration.
- No multi-user support.

## 3. Target User

The initial target user is a single Windows desktop user who:

- works on learning projects, coding projects, reading, lectures, and long-running goals
- often stops work while tired
- forgets the exact previous context the next day
- dislikes slow cloud-based todo startup apps
- dislikes manually maintaining large todo lists
- wants an always-visible but calm active-task reminder
- wants to know what they did and what to do next

## 4. Product Principles

### 4.1 Local First

Thread must function without a network connection. SQLite is the source of truth. There are no required calls to remote services during startup or normal app use.

### 4.2 Fast First Interaction

The Today window should be usable as quickly as possible after app process start. Avoid heavy initialization before rendering the first usable view.

Target:

- first usable Today window in under 1 second after process start on a typical development Windows machine
- no network-dependent startup path
- no splash screen unless technically required

### 4.3 Resumption Over Todo Management

The primary unit is not "task checked off"; it is "work context preserved." Completion matters, but the app should be optimized around:

- what was I doing?
- where did I stop?
- what is the next action?
- should this be in tomorrow's pickup queue?

### 4.4 One Active Thread

Only one task may be actively pursued at a time. The app should enforce exactly zero or one active session.

### 4.5 Donut, Not Timer

While a task is active, the floating task window must not show a numeric elapsed timer. The donut is the visible active-time proxy.

The app may show historical durations after a session ends, in Recent Threads, task detail, or history views.

### 4.6 Calm Desktop Presence

The floating task window must stay above normal windows, but it must not be loud, urgent, animated aggressively, or visually distracting.

### 4.7 Minimal Notion-Inspired Design

The app should use a simple neutral palette, dense but breathable layouts, subtle borders, modest shadows, and muted color accents. It should feel local, quiet, and fast.

## 5. Recommended Technology

Build the app as a greenfield Tauri desktop app.

### 5.1 Stack

- Tauri 2 for desktop shell and native windows
- Svelte + TypeScript for frontend UI
- Rust for backend commands and persistence
- SQLite for local storage
- CSS custom properties for theme tokens
- Tauri multi-window APIs for Today and floating task windows
- Tauri or native Windows startup registration

### 5.2 Rationale

Tauri is preferred because:

- it produces smaller desktop apps than Electron
- it supports native desktop windows
- it can create always-on-top floating windows
- it can bundle a polished web UI
- it supports Rust-side SQLite access
- it is suitable for local-first desktop tools

Svelte is preferred because:

- simple stateful UI is concise
- animations and conditional rendering are ergonomic
- build output is lightweight
- it is a good fit for a compact desktop app

## 6. Core Concepts

### 6.1 Pickup Task

A short-term task intended to be resumed soon. Pickup tasks are concrete and completion-oriented.

Examples:

- Watch Stat 110 Lecture 2
- Finish notes on conditional probability
- Review yesterday's Codex plan
- Do 5 probability practice problems

Pickup tasks usually live in the Pickup queue and are visible in the Today view.

### 6.2 Long-Term Backlog Task

A durable project, learning track, course, or goal that can receive repeated work sessions.

Examples:

- Stat 110
- Build local productivity tracker
- Read ISL
- Clean up Codex skills repo

Long-term tasks remain in the Backlog after a session unless explicitly archived or completed.

### 6.3 Active Session

A time-bounded period where the user is currently pursuing one task. Active sessions:

- begin when a user starts a Pickup or Backlog task
- end when the user completes, stops, switches, discards, or resolves recovery
- are persisted in SQLite
- are represented visually by the floating task window and donut

### 6.4 Floating Task Window

The compact always-on-top desktop window shown while a task is active.

It displays:

- task title
- next action, if available
- donut proxy
- subtle interaction affordance

It must not display a numeric active timer.

### 6.5 Donut Proxy

The donut is a circular lap simulator. It is similar to a treadmill lap display, but applied to the currently active task.

Default:

- one full revolution every 60 seconds
- ring fills clockwise from empty to full
- after one revolution, the ring starts over from empty
- each revolution uses a new muted color

The donut communicates "this task is underway" and gives gentle time texture without turning the session into a stopwatch.

### 6.6 Progress Note

A concise note captured when a session stops. It answers:

```text
What did I get done?
```

Examples:

- Watched the first 35 minutes and reached the birthday problem.
- Implemented the database schema and need to wire UI commands next.
- Read section 2.1 and got stuck on bias-variance examples.

### 6.7 Next Action

A concrete restart instruction for the next session. It answers:

```text
What should I do next?
```

Examples:

- Resume Lecture 2 at 34:20 and do the birthday problem example.
- Add the Tauri command for starting a session.
- Re-read the paragraph on conditional expectation and make one example.

The next action is one of the most important pieces of stored context.

## 7. Information Architecture

Thread has four main interface areas:

1. Today window
2. Floating task window
3. Stop/completion flow
4. Settings/history/task detail views

### 7.1 Today Window

The Today window is the startup surface and main command center. It should be compact and fast.

Required sections:

- Pickup
- Recent Threads
- Backlog
- Quick Capture

The Today view should not look like a landing page. It should immediately show usable task context.

### 7.2 Floating Task Window

The floating task window is shown only when a session is active. It stays always-on-top and provides the calm active-task reminder.

### 7.3 Stop Flow

The stop flow appears when the user stops, completes, switches tasks, or resolves an unfinished active session.

### 7.4 History And Task Detail

History and task detail are secondary views. They show completed sessions, notes, next actions, total historical time, and archived/completed tasks.

## 8. User Flows

### 8.1 First Launch Flow

When Thread launches for the first time:

1. Create/open local SQLite database.
2. Run migrations.
3. Insert default settings.
4. Show Today window.
5. Show empty states in Pickup, Recent Threads, and Backlog.

Do not show marketing onboarding. Do not require account creation.

Empty states:

- Pickup: "Nothing queued for pickup."
- Recent Threads: "No recent sessions yet."
- Backlog: "No long-term tasks yet."

Quick actions:

- Add Pickup Task
- Add Backlog Task
- Open Settings

### 8.2 Startup Launch Flow

When Windows starts:

1. Thread launches if `launch_on_startup` is enabled.
2. App opens SQLite locally.
3. App checks for unfinished active session.
4. If unfinished active session exists, show recovery flow.
5. Otherwise show Today window.

Startup must not block on network access.

### 8.3 Add Pickup Task Flow

User enters a task from Today:

Required:

- title

Optional:

- description
- next action
- pickup date
- priority

Default values:

- kind: `pickup`
- status: `pickup`
- pickup date: today
- priority: 0

After create:

- task appears in Pickup
- input clears
- focus remains useful for rapid entry

### 8.4 Add Long-Term Backlog Task Flow

User enters a long-term task from Today:

Required:

- title

Optional:

- description
- next action
- priority

Default values:

- kind: `long_term`
- status: `backlog`
- priority: 0

After create:

- task appears in Backlog
- task is available to start

### 8.5 Start Pickup Task Flow

When a user starts a Pickup task:

1. App verifies there is no active session.
2. If another active session exists, show switch-task flow.
3. App creates a session with `started_at`.
4. App sets task status to `active`.
5. App hides Today window.
6. App opens floating task window.
7. Donut starts animating from empty.

No explicit timer is shown.

### 8.6 Start Long-Term Task Flow

When a user starts a long-term Backlog task:

1. App verifies there is no active session.
2. If another active session exists, show switch-task flow.
3. App creates a session with `started_at`.
4. App marks the task active for the current session.
5. App hides Today window.
6. App opens floating task window.
7. Donut starts animating.

Important: starting a long-term task is "sudo pop" behavior. It temporarily pulls the task into active focus, but the task remains a durable backlog item. It returns to Backlog when the session stops unless the user explicitly completes or archives it.

### 8.7 Floating Window Click Flow

The floating window supports click and drag.

Click detection:

- pointer movement under 6px from mouse down to mouse up counts as click
- pointer movement 6px or more counts as drag

On click:

- open compact action menu

Action menu options:

- Continue
- Complete
- Stop / Pause
- Switch Task
- Open Today

The action menu must not display a numeric live timer.

### 8.8 Floating Window Drag Flow

When dragged:

1. Floating window moves with pointer.
2. Window remains always-on-top.
3. Final position is saved to settings.
4. Position is reused for future active sessions.

### 8.9 Complete Pickup Task Flow

When completing a Pickup task:

1. End active session.
2. Calculate duration internally.
3. Store `ended_at`, `duration_seconds`, and `end_reason = completed`.
4. Optionally ask for progress note.
5. Mark task `completed`.
6. Set `completed_at`.
7. Remove task from Pickup.
8. Show Today window or return to tray/background based on setting.

Historical duration may be visible in Recent Threads after completion.

### 8.10 Stop Pickup Task Flow

When stopping a Pickup task:

Prompt fields:

- progress note, optional
- next action, optional but visible
- destination, default `Pickup`

Destination options:

- keep in Pickup
- move to Backlog
- archive
- complete

Default:

- keep in Pickup

After stop:

- end session
- store duration internally
- save progress note if present
- update task next action if provided
- apply destination
- show Today window

### 8.11 Stop Long-Term Task Flow

When stopping a long-term task:

Prompt fields:

- progress note, required
- next action, required
- add to Pickup, optional checkbox

Defaults:

- return to Backlog
- do not complete whole long-term task
- do not archive

After stop:

1. End session.
2. Store duration internally.
3. Save progress note.
4. Save next action on the task.
5. Restore task status to `backlog`.
6. If "add to Pickup" is selected, make it visible in Pickup as well.

The exact implementation may represent a long-term task added to Pickup either by `status = pickup` or by a separate pickup marker. If using a single status field, prefer `status = pickup` and keep `kind = long_term` so the task remains semantically long-term.

### 8.12 Complete Long-Term Task Flow

When user chooses Complete on a long-term task:

1. Show confirmation:
   `Complete this entire long-term task?`
2. Default action is cancel/stop session, not completion.
3. If confirmed:
   - end active session
   - store duration
   - optionally collect final note
   - mark task `completed`
   - set `completed_at`

### 8.13 Switch Task Flow

If the user attempts to start another task while one is active:

1. Show current task stop form.
2. Require long-term note and next action if current task is long-term.
3. End current session.
4. Start selected task.
5. Replace floating window content.

No two sessions may be active at once.

### 8.14 Recovery Flow

If an unfinished session exists on launch:

Display recovery before normal Today view:

```text
You had an active session:
[Task title]

What happened?
[Resume]
[Stop and write note]
[Discard]
```

Resume:

- keep session active
- reopen floating window
- donut uses elapsed time from original start time

Stop and write note:

- open stop flow
- use `end_reason = app_closed`
- require long-term note and next action

Discard:

- set `end_reason = discarded`
- set `ended_at`
- do not count session in historical totals
- restore task to appropriate non-active status

## 9. Donut Proxy Specification

### 9.1 Active Timer Text Is Forbidden

While a task is underway, the floating task window must not show:

- `00:12`
- `12 min`
- `1h 20m`
- countdown text
- elapsed-time text
- lap count text
- words like "12 minutes elapsed"

The app records elapsed time internally, but the active visible representation is the donut only.

Historical durations may be shown after the session ends.

### 9.2 Default Donut Behavior

Default lap duration:

```text
60 seconds
```

Behavior:

- donut begins empty when session starts
- donut fills clockwise over 60 seconds
- after 60 seconds, donut resets to empty
- each new lap uses the next muted color
- animation remains smooth and calm
- no flashing
- no urgent pulse
- no alarm effect
- no sound

### 9.3 Donut Settings

Settings should allow:

- 30 seconds
- 60 seconds
- 90 seconds
- 120 seconds
- custom value from 10 seconds to 600 seconds

Default selected value:

- 60 seconds

### 9.4 Donut Calculation

Use wall-clock active session time, unless recovery has asked the user to adjust/discard a session.

```text
elapsed_seconds = now - session.started_at - paused_duration
lap_progress = (elapsed_seconds % lap_duration_seconds) / lap_duration_seconds
lap_index = floor(elapsed_seconds / lap_duration_seconds)
color = donut_palette[lap_index % donut_palette.length]
```

For v1, there is no separate pause-without-stop state. Stop ends the session.

### 9.5 Donut Rendering

Preferred implementation:

- SVG circular progress ring

Rendering details:

- track stroke: muted border gray
- progress stroke: current lap color
- stroke cap: round
- stroke width: moderate
- inner area empty or subtly transparent
- no text inside the donut
- no numeric label outside the donut

The donut can animate using requestAnimationFrame or CSS/Svelte reactive state. It must remain smooth without excessive CPU usage.

## 10. Visual Design Specification

### 10.1 Design Direction

Thread should feel minimal, local, and Notion-inspired.

It should look like a quiet utility, not a gamified habit app, marketing page, or colorful dashboard.

### 10.2 Palette

Use a light neutral palette.

```text
App background:      #F7F6F3
Surface:             #FFFFFF
Surface subtle:      #FAFAF8
Primary text:        #37352F
Secondary text:      #787774
Muted text:          #9B9A97
Border:              #E6E4DE
Border strong:       #D8D5CC
Hover:               #EFEDE8
Selected:            #E9E7E1
Shadow:              rgba(15, 15, 15, 0.08)
Danger text:         #9F4A44
Focus ring:          #9A8F7A
```

Donut palette:

```text
Sage:                #7C9885
Muted blue:          #6E8FA3
Dusty rose:          #B58A8A
Warm ochre:          #B69B63
Soft violet:         #8E83A8
Clay:                #A98274
Charcoal green:      #65756A
```

Avoid:

- saturated neon colors
- strong gradients
- purple-blue gradient-heavy UI
- gamified streak colors
- oversized hero sections
- decorative blobs
- bokeh/orb backgrounds
- loud animations

### 10.3 Typography

Use this stack:

```css
font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
```

If Inter is not bundled, system UI fonts are acceptable.

Suggested scale:

- app title: 18-20px
- section headings: 12-13px
- task title: 14-16px
- body text: 13-14px
- metadata: 12px
- buttons: 13px

Rules:

- no negative letter spacing
- no viewport-scaling font sizes
- no huge hero text
- text must fit in buttons and rows at minimum window size

### 10.4 Today Layout

Default window:

- width: about 760px
- height: about 620px
- min width: about 520px
- min height: about 440px
- resizable

Layout:

- top bar with app name and settings icon
- compact sections
- thin dividers
- no nested cards
- task rows are simple and scannable
- whitespace is moderate, not spacious marketing-page spacing

### 10.5 Floating Window Layout

Default:

- width: 280px
- height: 120-160px
- position: upper-right with margin if no saved position
- always-on-top
- draggable
- remembers position

Visual:

- surface: white or subtle off-white
- border: 1px solid neutral border
- radius: 8px or less
- shadow: soft and shallow
- title line
- next action line if present
- donut aligned to one side or centered depending on compactness
- no timer text

### 10.6 Buttons And Controls

Use familiar controls:

- icon button for settings
- segmented control or select for task type
- checkboxes/toggles for binary settings
- text buttons only for clear commands

If icons are used, prefer an established icon library such as lucide.

## 11. Data Model

Use SQLite with migrations. Store timestamps as ISO 8601 strings in UTC.

### 11.1 tasks

```sql
CREATE TABLE tasks (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  description TEXT,
  kind TEXT NOT NULL CHECK (kind IN ('pickup', 'long_term')),
  status TEXT NOT NULL CHECK (status IN ('pickup', 'backlog', 'active', 'completed', 'archived')),
  priority INTEGER NOT NULL DEFAULT 0,
  pickup_date TEXT,
  next_action TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  completed_at TEXT,
  archived_at TEXT
);
```

Rules:

- `kind = pickup` represents a short-term task.
- `kind = long_term` represents a durable backlog task.
- `status = active` is temporary and only valid while there is an active session.
- completed tasks are hidden from Today by default.
- archived tasks are hidden from Today by default.

### 11.2 sessions

```sql
CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  duration_seconds INTEGER,
  end_reason TEXT CHECK (end_reason IN ('completed', 'paused', 'stopped', 'switched', 'app_closed', 'discarded')),
  progress_note TEXT,
  next_action TEXT,
  lap_duration_seconds INTEGER NOT NULL DEFAULT 60,
  recovered_from_crash INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id)
);
```

Rules:

- `ended_at IS NULL` means the session is active.
- there must be at most one active session.
- duration is stored after session ends.
- discarded sessions should not count in history totals.
- stopped long-term sessions require `progress_note` and `next_action`.

### 11.3 task_events

```sql
CREATE TABLE task_events (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  session_id TEXT,
  event_type TEXT NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id),
  FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

Event examples:

- created
- started
- stopped
- completed
- moved_to_pickup
- moved_to_backlog
- archived
- next_action_updated
- recovery_resolved

### 11.4 settings

```sql
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

Default settings:

```text
launch_on_startup=true
show_today_on_startup=true
donut_lap_duration_seconds=60
theme=notion_light
floating_window_always_on_top=true
floating_window_x=null
floating_window_y=null
require_long_term_progress_note=true
require_long_term_next_action=true
return_to_today_after_stop=true
```

## 12. Backend Command Interface

Expose typed Tauri commands from Rust to the frontend.

Core commands:

```ts
createTask(input: CreateTaskInput): Promise<Task>
updateTask(input: UpdateTaskInput): Promise<Task>
archiveTask(input: ArchiveTaskInput): Promise<Task>
listToday(): Promise<TodayPayload>
listBacklog(): Promise<Task[]>
listRecentThreads(input: RecentThreadsInput): Promise<RecentThread[]>
startSession(input: StartSessionInput): Promise<ActiveSession>
getActiveSession(): Promise<ActiveSession | null>
completeSession(input: CompleteSessionInput): Promise<SessionResult>
stopSession(input: StopSessionInput): Promise<SessionResult>
switchTask(input: SwitchTaskInput): Promise<ActiveSession>
resolveSessionRecovery(input: RecoveryDecisionInput): Promise<RecoveryResult>
getSettings(): Promise<Settings>
updateSettings(input: UpdateSettingsInput): Promise<Settings>
saveFloatingWindowPosition(input: WindowPositionInput): Promise<void>
exportDatabase(): Promise<ExportResult>
openDataFolder(): Promise<void>
```

Core types:

```ts
type TaskKind = 'pickup' | 'long_term';
type TaskStatus = 'pickup' | 'backlog' | 'active' | 'completed' | 'archived';

type EndReason =
  | 'completed'
  | 'paused'
  | 'stopped'
  | 'switched'
  | 'app_closed'
  | 'discarded';
```

## 13. State Rules

### 13.1 Single Active Session

The app must enforce at most one active session.

Implementation must prevent:

- two active rows in `sessions`
- two tasks with `status = active`
- floating window showing one task while backend active session is another

### 13.2 Status Restoration

When a session ends:

- pickup task completed: task becomes `completed`
- pickup task stopped and kept: task becomes `pickup`
- pickup task moved to backlog: task becomes `backlog`
- long-term task stopped: task becomes `backlog` unless added to Pickup
- long-term task completed: task becomes `completed`
- discarded recovery session: task returns to best previous non-active state

### 13.3 Window Consistency

If active session exists:

- floating window should be available
- Today may be hidden
- tray menu should allow opening Today or floating window

If no active session exists:

- floating window should be closed/hidden
- Today may be shown depending on user flow

### 13.4 App Close Behavior

If app exits with active session:

- do not silently complete
- leave session unfinished
- recover on next launch

## 14. Today Payload Rules

`listToday()` should return:

- pickup tasks
- recent threads
- backlog preview
- active session if one exists
- settings needed for initial render

Pickup ordering:

1. active task first if present
2. tasks with `pickup_date <= today`
3. tasks stopped recently and kept in Pickup
4. higher priority
5. most recently updated

Recent Threads ordering:

1. today sessions
2. yesterday sessions
3. most recent first
4. default limit 8

Backlog preview ordering:

1. high priority long-term tasks
2. tasks with recent sessions
3. older untouched backlog tasks
4. default limit 10

## 15. Screens

### 15.1 Today Window

Required:

- title: Thread
- settings icon button
- active session banner if active session exists
- recovery prompt if unfinished session exists
- Pickup section
- Recent Threads section
- Backlog section
- Quick Capture

Task row contents:

- title
- next action if present
- subtle metadata after session ended, such as "yesterday" or "last worked"
- no checkbox-first primary interaction

### 15.2 Floating Task Window

Required:

- title
- next action if present
- donut proxy
- action menu on click
- drag support
- always-on-top
- saved position

Forbidden:

- active elapsed time text
- countdown text
- current lap count text
- blinking urgent states

### 15.3 Stop Flow

Required:

- task title
- progress note field
- next action field
- destination control
- submit button
- cancel/continue option

For pickup tasks:

- note optional
- next action optional
- default destination: Pickup

For long-term tasks:

- note required
- next action required
- default destination: Backlog
- optional add to Pickup

### 15.4 Task Detail View

Required:

- task title
- description
- kind
- status
- current next action
- total historical active time
- session history
- progress notes
- complete/archive controls

Historical durations can be visible here.

### 15.5 Settings View

Required:

- launch on startup
- show Today on startup
- donut lap duration
- theme
- reset floating window position
- export database
- open data folder

## 16. Keyboard Behavior

Today:

```text
N      add new task
/      search or filter
Enter  start selected task
Esc    hide Today window
Ctrl+, open settings
```

Floating window:

```text
Enter  open action menu
Esc    close action menu
```

Stop form:

```text
Ctrl+Enter submit
Esc        cancel and return to action menu
```

Keyboard support is additive. The app must remain usable by mouse.

## 17. Tray Behavior

Add a system tray menu if supported cleanly.

Tray menu:

- Open Today
- Show Floating Task
- Stop Current Task
- Settings
- Quit

Quitting while a session is active should preserve unfinished session for recovery.

## 18. Error Handling

Database open failure:

- show plain error window
- explain that local data could not be opened
- offer open data folder if possible
- do not silently create replacement data over existing broken data

Migration failure:

- stop launch
- preserve original database
- show recoverable error

Startup registration failure:

- app still works
- settings page shows failure state

Floating window creation failure:

- Today remains available
- active session controls appear in Today

Invalid stop form:

- inline validation
- keep user text intact
- no disruptive alert popups

## 19. Privacy And Data

Requirements:

- all data local
- no telemetry
- no account
- no cloud sync
- no required network calls
- no analytics

Database location:

- standard Tauri app data directory

Export:

- create a user-readable backup copy of SQLite database
- never delete original data during export

## 20. Acceptance Criteria

The implementation is accepted when all criteria are true.

### 20.1 App And Startup

- app launches locally on Windows
- first-run Today view appears without account or network
- startup setting can be enabled and disabled
- Today appears on startup when enabled
- all task/session data persists across restarts

### 20.2 Task Creation

- user can create Pickup task
- user can create long-term Backlog task
- created tasks persist in SQLite
- Pickup and Backlog appear in correct sections

### 20.3 Session Lifecycle

- starting Pickup task creates active session
- starting long-term task creates active session
- Today hides when session starts
- exactly one active session is allowed
- completing Pickup task logs session and removes task from Pickup
- stopping Pickup task can keep it in Pickup with next action
- starting long-term task does not permanently remove it from Backlog
- stopping long-term task requires progress note and next action
- completing long-term task requires explicit confirmation

### 20.4 Floating Window

- floating window appears when session starts
- floating window stays above normal desktop windows
- floating window can be dragged
- floating window position persists
- clicking floating window opens action menu
- floating window shows task title
- floating window shows next action if present
- floating window does not show an explicit timer

### 20.5 Donut

- donut is visible during active session
- donut completes one revolution per 60 seconds by default
- donut changes color on each revolution
- donut resets smoothly after each lap
- donut uses muted Notion-compatible colors
- no active numeric timer is visible in floating window

### 20.6 History And Recovery

- Recent Threads show latest stopped/completed sessions
- historical durations are visible outside the active floating state
- unfinished active session is detected on restart
- user can resume, stop with note, or discard unfinished session
- discarded sessions do not count toward history totals

### 20.7 Visual Design

- UI uses neutral Notion-inspired palette
- no loud gradient-heavy design
- no gamified streak UI
- no decorative orbs/blobs
- text fits at minimum window size
- task rows are compact and scannable

## 21. Automated Test Plan

Backend tests:

- create pickup task
- create long-term task
- update task
- archive task
- list Today payload
- start session
- reject second active session
- stop pickup session
- stop long-term session without note fails
- stop long-term session without next action fails
- complete pickup task
- complete long-term task
- recover unfinished session
- discard unfinished session
- persist settings
- calculate session duration

Frontend/unit tests:

- donut progress calculation
- donut color cycling
- active timer text is absent from floating window
- click-vs-drag threshold
- stop form validation
- Today list ordering
- settings form updates values

Manual tests:

- cold launch
- first-run empty state
- GitHub-free offline normal app use
- create tasks
- start tasks
- floating always-on-top behavior
- donut visual calmness
- stop flows
- restart recovery
- startup registration
- settings persistence
- export database

## 22. Suggested Ralph Task Decomposition

Ralph should decompose this PRD into implementation tasks roughly in this order.

### 22.1 Project Foundation

- initialize Tauri 2 + Svelte + TypeScript app
- configure Rust workspace
- add baseline package scripts
- create app shell
- add neutral theme tokens
- verify app launches

### 22.2 SQLite Persistence

- add SQLite dependency
- create migration system
- implement tasks, sessions, task_events, settings
- add default settings
- add typed Rust data access layer
- add backend tests

### 22.3 Tauri Command Layer

- expose task commands
- expose session commands
- expose settings commands
- expose recovery commands
- add typed frontend wrappers

### 22.4 Today Window

- build Today layout
- implement Pickup section
- implement Recent Threads section
- implement Backlog section
- implement Quick Capture
- implement empty states
- apply Notion-inspired styling

### 22.5 Session Lifecycle

- implement start session
- implement complete session
- implement stop session
- implement switch task
- enforce one active session
- implement task status restoration rules

### 22.6 Floating Task Window

- create separate floating Tauri window
- make it always-on-top
- hide Today on session start
- show title and next action
- implement click action menu
- implement click-vs-drag threshold
- persist window position

### 22.7 Donut Proxy

- implement donut component
- default lap duration 60 seconds
- cycle muted colors per lap
- add settings support for lap duration
- ensure active numeric timer is not rendered
- add unit tests for lap calculation

### 22.8 Stop And Recovery Flows

- build stop form
- require progress and next action for long-term stops
- implement pickup destination choices
- implement unfinished-session recovery
- handle discard/resume/stop recovery decisions

### 22.9 Settings, Tray, Startup

- implement settings view
- implement startup registration
- implement tray menu
- implement reset floating position
- implement export database
- implement open data folder

### 22.10 History And Polish

- build task detail view
- show session history
- show historical durations after sessions end
- refine keyboard behavior
- run manual visual QA
- package Windows build if feasible

## 23. Explicit Defaults

- Product name: Thread
- Platform: Windows first
- Storage: local SQLite
- Cloud sync: none
- Account system: none
- Default frontend: Svelte + TypeScript
- Desktop shell: Tauri 2
- Active timer display: forbidden while task is underway
- Donut default lap duration: 60 seconds
- Floating window: always-on-top by default
- Theme: light Notion-inspired neutral palette
- One active session at a time
- Long-term stop requires progress note and next action
- Pickup stop keeps task in Pickup by default
- Long-term stop returns task to Backlog by default
- Historical durations may be shown after sessions end

## 24. Implementation Notes For Ralph Agents

- Do not implement cloud sync or external integrations.
- Do not show active elapsed time in the floating window.
- Do not treat the donut as optional.
- Keep UI compact and Notion-like.
- Prefer deterministic behavior over AI features.
- Keep changes focused to the assigned task.
- Use discovered tasks for useful follow-up work that is outside the current assignment.
- Do not manually edit `.ralph/ralph.db`.
