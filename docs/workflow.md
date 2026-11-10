# Workflow

The flow Amaca automates:

```
Jira ticket  ->  terminal multiplexer  ->  Claude
```

1. User runs Amaca inside zellij, tmux, screen, kitty, iTerm2 or WezTerm.
   Amaca detects the backend first, probing zellij, then tmux, then screen,
   then kitty, then iTerm2, then WezTerm (`ZELLIJ_PANE_ID`, `TMUX_PANE`,
   `STY`, `KITTY_WINDOW_ID`, `ITERM_SESSION_ID`, `WEZTERM_PANE` set and their
   socket or server answering). A failing probe falls through to the next
   backend; with no reachable backend Amaca exits early
   ([docs/multiplexer.md](multiplexer.md)).
2. User gives a ticket (key, URL, branch, or free text).
3. Amaca asks Claude to resolve the ticket key and fetch the ticket, returning
   `id`, `title`, `summary` and `body` as JSON ([docs/jira.md](docs/jira.md)).
   The call runs in the run's Claude session (`--session-id <uuid>`), so the
   ticket content stays in that session's context. Later steps resume the
   session (`--resume <uuid>`) and do not fetch the ticket again.
4. Amaca resolves the ticket worktree: existing branch, existing worktree, or a
   new branch from the dev branch ([docs/worktree.md](docs/worktree.md)).
5. When the worktree was just created, Amaca runs the `command.created` setup
   command from the config inside it ([docs/config.md](docs/config.md)).
6. Amaca opens a banner pane in the current zellij, tmux, screen, kitty,
   iTerm2 or WezTerm window: a small pane runs the hidden `ꙮ` command with the
   ticket id, title and summary, and the focus goes back to the original pane
   ([docs/multiplexer.md](multiplexer.md)).
   The banner sits above the original pane, except with screen, which cannot
   move a region and keeps it below ([docs/screen.md](screen.md)).
7. Amaca resumes the session interactively in the original pane, named after
   the ticket id (`--name <TICKET_ID>`), asking Claude to solve the ticket. The
   ticket is already in the session's context. The prompt comes from the
   config ([docs/config.md](docs/config.md)): `prompt.created` when the
   worktree was just created, `prompt.existing` otherwise.

## Input

```sh
amaca solve <link|key>   # Jira URL or ticket key
amaca solve              # current git branch name
amaca <link|key>         # same, solve is the default subcommand
amaca                    # same, current git branch name
```

The CLI is clap-based; `solve` holds the whole workflow and is the only
subcommand for now. Without a subcommand, `solve` runs.

- With an argument, the input is a Jira link or a ticket key.
- Without arguments, Amaca takes the current git branch name. Outside a git
  repository it fails: the workflow requires git.
- A plain key (`PROJECT-123`), a Jira URL, a branch name or free text all go to
  Claude, which returns the ticket as JSON (`claude --print`). No ticket found
  is an explicit error.
- Every run creates a new Claude session with a fresh id. The session is stored
  by Claude Code and resumed by id; Amaca writes no state of its own.

## Open questions

- Window layout: one tab with a split, or a dedicated workspace per ticket. Not decided yet.
