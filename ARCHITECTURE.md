# Architecture

Amaca automates a personal workflow: Jira -> terminal multiplexer -> Claude.
Given a ticket, it prepares the terminal window to work on it.

Topic details live in the knowledge base: [docs/workflow.md](docs/workflow.md),
[docs/jira.md](docs/jira.md), [docs/worktree.md](docs/worktree.md),
[docs/multiplexer.md](docs/multiplexer.md), [docs/tmux.md](docs/tmux.md),
[docs/wezterm.md](docs/wezterm.md), [docs/config.md](docs/config.md).

## Flow

```
ticket input
     |
     v
 input parser  ->  ticket (id, title, summary, body)
     |
     v
 worktree resolution  ->  ticket working directory
     |
     v
 multiplexer setup (tmux or wezterm cli)
     |
     +--> top pane (2 rows): current exe with the hidden ꙮ command,
     |    given id, title and summary; focus returns to the pane below
     |
     +--> current pane: Claude (resumes the ticket session, named after
          the ticket id)
```

## Components

| Component | Responsibility | Status |
| --- | --- | --- |
| CLI | clap subcommands; `solve` runs the whole workflow and is the default when no subcommand is given | Done |
| Input parser | Resolve the input (argument or git branch) into a ticket via Claude: `id`, `title`, `summary`, `body` | Done |
| Session | New Claude session id per run, created by the ticket fetch and reused by the launchers | Done |
| Config | `.amaca.jsonc` lookup from the current directory up to the root, closest file wins: prompts and the worktree creation command ([docs/config.md](docs/config.md)) | Done |
| Worktree resolver | Find or create the ticket worktree from the ticket id ([docs/worktree.md](docs/worktree.md)) | Done |
| Terminal orchestrator | Detect the multiplexer, split the current pane, run the `ꙮ` banner on top and give the focus back; tmux and WezTerm backends | Done |
| Claude launcher | Resume the session named after the ticket id and ask Claude to solve the ticket | Done |

## Decisions

See the topic docs for the rationale.

- Jira data is fetched by Claude, not by Amaca ([docs/jira.md](docs/jira.md)).
- Claude returns the ticket as JSON (`id`, `title`, `summary`, `body`), fetched
  through the Atlassian MCP tools ([docs/jira.md](docs/jira.md)). Amaca keeps
  only `id`, `title` and `summary`; `body` stays in the session's context.
- Every run creates a new Claude session; its id lives in memory only, no
  state on disk ([docs/workflow.md](docs/workflow.md)).
- The ticket branch name is the ticket id; the worktree holds the working
  directory ([docs/worktree.md](docs/worktree.md)).
- The resumed Claude session is named after the ticket id (`--name`)
  ([docs/workflow.md](docs/workflow.md)).
- The resumed session gets one prompt: ask Claude to solve the ticket. The
  prompt comes from the config, with a separate one for a just-created worktree
  ([docs/config.md](docs/config.md), [docs/workflow.md](docs/workflow.md)).
- A just-created worktree can run one setup command from the config, inside the
  new worktree, before the layout and Claude: a shell line through
  `system(3)`, or an argv through `Command` ([docs/config.md](docs/config.md)).
- The backend is detected by probing in priority order: tmux when `TMUX_PANE`
  is set and `tmux list-panes` succeeds; otherwise WezTerm when
  `WEZTERM_PANE` is set and `wezterm cli list` succeeds. A failing tmux falls
  through to WezTerm ([docs/multiplexer.md](docs/multiplexer.md)).
- Each backend is driven through its own CLI: `tmux` ([docs/tmux.md](docs/tmux.md))
  or `wezterm cli` ([docs/wezterm.md](docs/wezterm.md)).
- The layout is a split of the current pane: a 2-row pane on top runs the
  hidden `ꙮ` command with the ticket id, title and summary, then the focus goes
  back to the pane below, where Claude launches
  ([docs/multiplexer.md](docs/multiplexer.md)).

## Non-goals

- Launching Neovim. Editing is part of the user's own workflow, outside Amaca.
- Other multiplexers and terminals with a built-in mux beyond tmux and WezTerm.
  Add one module under `src/mux/` per backend
  ([docs/multiplexer.md](docs/multiplexer.md)).

## Open questions

- Reusing an existing top pane instead of splitting again.
