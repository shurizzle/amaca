# AGENTS.md

Rules for agents working on Amaca. See [ARCHITECTURE.md](ARCHITECTURE.md) for design and [docs/](docs/) for project documentation.

## Project

Amaca automates a personal workflow: Jira -> terminal multiplexer -> Claude.
It takes a ticket as input and sets up the terminal window to work on it.

## Language

- Code, comments, identifiers, documentation: English.
- Chat with the user: Italian, unless asked otherwise.

## Where rules go

Every new rule agreed with the user is written to the file that owns it:

| Rule about | File |
| --- | --- |
| Agent behavior, workflow, code conventions, commands | `AGENTS.md` (this file) |
| Design, components, technical decisions | `ARCHITECTURE.md` |
| One topic of the knowledge base (workflow, Jira, WezTerm, config...) | `docs/<topic>.md` |
| Project intro, quickstart, documentation index | `README.md` (root) |

`README.md` lives in the repository root only. `docs/` holds one markdown per
topic: it is the shared knowledge base for the whole project. No index file
in `docs/`; the index is `README.md`.

Do not leave an agreed rule only in chat. Add it in the same session it is agreed.

## Keeping docs in sync

Every new directive from the user updates every markdown that needs it, in the
same turn, before replying. Not only the most obvious file: check all of
`README.md`, `ARCHITECTURE.md`, `AGENTS.md` and `docs/*.md` for what the
directive contradicts or completes. A doc that is now stale or wrong gets
fixed or deleted, not left behind.

## Code conventions

- Rust 2024 edition, standard `cargo` tooling.
- Errors: `anyhow::Result`, with `anyhow!`, `bail!` and `Context` for messages.
  No `Box<dyn Error>` and no hand-rolled `format!` error plumbing.
- Follow the patterns already in the codebase; do not introduce new abstractions without a reason.
- Before considering work done: `cargo fmt`, `cargo clippy`, `cargo test`.
- `schema.json` is generated from the config structs. After changing them, run
  `UPDATE_SCHEMA=1 cargo test config_schema_is_current`.
- Keep multiplexer-specific code inside its backend under `src/mux/`: no
  `tmux` or `wezterm cli` calls outside the tmux and WezTerm modules.

## Git

- Never commit, amend, push, rebase or tag without explicit confirmation.
- Commit messages: Conventional Commits.
- Do not write client names in configs, docs or notes.
