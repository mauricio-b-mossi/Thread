# Thread

Thread is a Windows-first local desktop app for resuming work context. It opens to a compact Today view with Pickup, Recent Threads, Backlog, and Quick Capture, then keeps one active task in a small always-on-top floating window.

The floating window is intentionally quiet: it shows the task title, optional next action, and a donut that completes one revolution every 60 seconds by default. It does not show an active numeric timer.

## Product Scope

- Create short-term pickup tasks and durable long-term tasks.
- Start exactly one active session at a time.
- Stop pickup tasks back to Pickup by default, with optional notes and next action.
- Stop long-term tasks only after adding both a progress note and next action.
- Complete pickup tasks from the active session.
- Recover unfinished sessions after an app close.
- Review recent stopped or completed threads and task details.
- Configure startup launch, Today-on-startup, donut lap duration, and floating window position.
- Export a database backup and open the local data folder.

Thread is local-first and offline. It has no cloud sync, accounts, telemetry, or normal-use network dependency.

## Local Storage

Thread stores data in SQLite through the Tauri backend. The database file is named `thread.sqlite3` and is created in the standard app data directory for the installed app. The schema includes `tasks`, `sessions`, `task_events`, and `settings`.

Database export creates a copy under an `exports` folder in the app data directory. It does not delete or replace the active database.

## Development

Prerequisites:

- Node.js and npm
- Rust and Cargo
- Visual Studio 2022 Build Tools with the C++ workload for Windows Tauri builds

Commands:

```powershell
npm install
npm run dev
npm run check
npm run typecheck
npm test
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-rust.ps1
npm run tauri:dev
npm run tauri:build
```

`npm test` runs the Svelte check and frontend behavior tests. `scripts\check-rust.ps1` runs Rust tests through the Visual Studio developer environment.

## Current Limitations

- The app is currently Windows-focused.
- Startup registration depends on Windows desktop integration and can report an error if the host blocks registration.
- System tray behavior depends on the desktop environment and installed Tauri runtime support.
- There is no cloud sync, account system, sharing, reminders, or mobile companion app.
