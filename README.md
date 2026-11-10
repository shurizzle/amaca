# Amaca

Automates a personal workflow: **Jira -> terminal multiplexer -> Claude**.
Takes a ticket as input and sets up the terminal window to work on it.
Supported multiplexers: zellij, tmux, screen, kitty, iTerm2 and WezTerm
(standalone multiplexers win when more than one is present).

## Status

Input handling works: a Jira link or key as argument, or the current git
branch when no argument is given. Claude resolves the ticket and returns
`id`, `title`, `summary` and `body` as JSON, inside the run's Claude session,
so the content stays in its context for the next steps. Amaca then finds or
creates the ticket worktree, enters it, opens a small ticket banner pane in
the current zellij, tmux, screen, kitty, iTerm2 or WezTerm window, gives the
focus back, resumes the session named after the ticket id and asks Claude to
solve the ticket.

The `review` subcommand takes a GitHub pull request, resolves its head branch
with Claude, extracts the Jira ticket key from the branch, loads the ticket
and the whole pull request conversation into one session, checks the branch
out in a throwaway worktree and asks Claude to review it
([docs/github.md](docs/github.md)).

## Usage

```sh
amaca solve https://example.atlassian.net/browse/PROJ-123
amaca solve PROJ-123
amaca solve            # uses the current git branch name
amaca PROJ-123         # solve is the default subcommand
amaca review https://github.com/owner/repo/pull/123
amaca review 123       # pull request number
```

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) — design, components, decisions, open questions
- [AGENTS.md](AGENTS.md) — rules for agents and code conventions
- [docs/](docs/) — knowledge base: workflow, integrations, configuration
- [schema.json](schema.json) — JSON Schema for `.amaca.jsonc`

## License

MIT. See [LICENSE](LICENSE).

## Build

```sh
cargo build
```
