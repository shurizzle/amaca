# Architecture

Amaca automates a personal workflow: Jira -> WezTerm -> Claude.
Given a ticket, it prepares the terminal window to work on it.

Topic details live in the knowledge base: [docs/workflow.md](docs/workflow.md),
[docs/jira.md](docs/jira.md), [docs/worktree.md](docs/worktree.md),
[docs/wezterm.md](docs/wezterm.md).

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
 WezTerm setup (wezterm cli)
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
| Worktree resolver | Find or create the ticket worktree from the ticket id ([docs/worktree.md](docs/worktree.md)) | Done |
| Terminal orchestrator | Split the current pane, run the `ꙮ` banner on top and give the focus back; WezTerm backend only for now | Done |
| Claude launcher | Resume the session named after the ticket id and ask Claude to solve the ticket | Done |

## Decisions

See the topic docs for the rationale.

- Jira data is fetched by Claude, not by Amaca ([docs/jira.md](docs/jira.md)).
- Claude returns the ticket as JSON (`id`, `title`, `summary`, `body`), fetched
  through the Atlassian MCP tools ([docs/jira.md](docs/jira.md)).
- Every run creates a new Claude session; its id lives in memory only, no
  state on disk ([docs/workflow.md](docs/workflow.md)).
- The ticket branch name is the ticket id; the worktree holds the working
  directory ([docs/worktree.md](docs/worktree.md)).
- The resumed Claude session is named after the ticket id (`--name`)
  ([docs/workflow.md](docs/workflow.md)).
- The resumed session gets one prompt: ask Claude to solve the ticket
  ([docs/workflow.md](docs/workflow.md)).
- WezTerm is driven through `wezterm cli` ([docs/wezterm.md](docs/wezterm.md)).
- The layout is a horizontal split of the current pane: a 2-row pane on top
  runs the hidden `ꙮ` command with the ticket id, title and summary, then the
  focus goes back to the pane below, where Claude launches
  ([docs/wezterm.md](docs/wezterm.md)).

## Non-goals

- Launching Neovim. Editing is part of the user's own workflow, outside Amaca.
- Only WezTerm is supported for now; other multiplexers (tmux) come later. No
  abstraction layer until the second backend exists ([docs/wezterm.md](docs/wezterm.md)).

## Open questions

- Backend selection: config file, flag, or detection. See [docs/wezterm.md](docs/wezterm.md).
