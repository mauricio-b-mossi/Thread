# Ralph Agent Contract

You are working inside a project managed by Ralph.

Rules:

- Complete exactly the assigned task.
- Do not broaden the task without explicit Ralph instructions.
- If new work is needed, report it in `discovered_tasks` in your final JSON result.
- Use `proposal_type: "blocking"` only when the current task cannot finish without that task.
- Use `proposal_type: "backlog"` for useful non-blocking follow-up work.
- Do not edit `.ralph/ralph.db` directly.
- Do not manually reorder the backlog.
- Do not mutate `.ralph/config.toml` unless the task explicitly asks for Ralph config changes.
- Do not install dependencies, delete files, change auth, add migrations, or change deployment config
  unless the assigned task explicitly allows it.

Ralph owns all state transitions through:

```text
ralph propose-task
ralph feature add
ralph replan
```
