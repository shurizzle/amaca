# Multiplexers

Amaca drives a terminal multiplexer: it splits the current pane and runs the
ticket banner in the new one. Two kinds of backend are supported:

| Kind | Backend | Pane id |
| --- | --- | --- |
| Standalone multiplexer | tmux ([tmux.md](tmux.md)) | `TMUX_PANE` |
| Terminal with a built-in mux | WezTerm ([wezterm.md](wezterm.md)) | `WEZTERM_PANE` |

The backend code lives in `src/mux/`: `mod.rs` detects and dispatches, and one
module per backend holds the backend-specific calls.

## Detection

Before any other work, Amaca detects the backend by calling each backend's
`detect() -> Option<Mux>` in priority order, and exits early when none
returns a backend. tmux takes precedence over WezTerm: a terminal with a
built-in multiplexer can also run tmux inside it, and then tmux is the mux
that owns the panes.

1. `tmux::detect()`: `TMUX_PANE` is set and `tmux list-panes` succeeds.
2. `wezterm::detect()`: `WEZTERM_PANE` is set and `wezterm cli list`
   succeeds.
3. Otherwise: error, no ticket is fetched and no worktree is created.

A set environment variable is not enough: the server must answer too. When
several candidates are present, a backend whose probe fails falls through to
the next one.

After changing directory into the ticket worktree, Amaca emits OSC 7
(`\x1b]7;file://<path>\x07`) so the terminal tracks the worktree as the current
directory.

## Adding a backend

1. Add a module under `src/mux/` with `detect() -> Option<Mux>` and `layout()`.
2. Add a variant to `Mux`, an arm to `layout()`, and the call to `detect()` in
   the priority chain in `Mux::detect()`.
3. Add the detection rule, giving priority to the backend that owns the panes
   when another one runs inside it.

Keep every backend-specific call in its module. The input parser, ticket
resolution and launchers must not care which multiplexer is running.
