# Amaca

Automates a personal workflow: **Jira -> terminal multiplexer -> Claude**.
Takes a ticket as input and sets up the terminal window to work on it.
Supported multiplexers: tmux and WezTerm (tmux wins when both are present).

## Status

Input handling works: a Jira link or key as argument, or the current git
branch when no argument is given. Claude resolves the ticket and returns
`id`, `title`, `summary` and `body` as JSON, inside the run's Claude session,
so the content stays in its context for the next steps. Amaca then finds or
creates the ticket worktree, enters it, splits the current tmux or WezTerm pane
with a 2-row ticket banner on top, gives the focus back, resumes the session
named after the ticket id and asks Claude to solve the ticket.

## Usage

```sh
amaca solve https://example.atlassian.net/browse/PROJ-123
amaca solve PROJ-123
amaca solve            # uses the current git branch name
amaca PROJ-123         # solve is the default subcommand
```

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — design, components, decisions, open questions
- [AGENTS.md](AGENTS.md) — rules for agents and code conventions
- [docs/](docs/) — knowledge base: workflow, integrations, configuration
- [schema.json](schema.json) — JSON Schema for `.amaca.jsonc`

## Build

```sh
cargo build
```
