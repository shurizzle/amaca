# Multiplexers

Amaca drives a terminal multiplexer: it splits the current pane and runs the
ticket banner in the new one. Two kinds of backend are supported:

| Kind | Backend | Pane id |
| --- | --- | --- |
| Standalone multiplexer | zellij ([zellij.md](zellij.md)) | `ZELLIJ_PANE_ID` |
| Standalone multiplexer | tmux ([tmux.md](tmux.md)) | `TMUX_PANE` |
| Standalone multiplexer | screen ([screen.md](screen.md)) | session from `STY` |
| Terminal with a built-in mux | WezTerm ([wezterm.md](wezterm.md)) | `WEZTERM_PANE` |

The backend code lives in `src/mux/`: `mod.rs` detects and dispatches, and one
module per backend holds the backend-specific calls.

## Detection

Before any other work, Amaca detects the backend by calling each backend's
`detect() -> Option<Mux>` in priority order, and exits early when none
returns a backend. Standalone multiplexers take precedence over WezTerm, a
terminal with a built-in multiplexer that can also run them inside it: they
own the panes in that case. zellij comes before tmux because it commonly runs
inside WezTerm, and there it owns the panes; tmux comes before screen.

1. `zellij::detect()`: `ZELLIJ_PANE_ID` is set and
   `zellij action list-panes` succeeds.
2. `tmux::detect()`: `TMUX_PANE` is set and `tmux list-panes` succeeds.
3. `screen::detect()`: `STY` is set and `screen -Q number` succeeds.
4. `wezterm::detect()`: `WEZTERM_PANE` is set and `wezterm cli list`
   succeeds.
5. Otherwise: error, no ticket is fetched and no worktree is created.

A set environment variable is not enough: the server must answer too. When
several candidates are present, a backend whose probe fails falls through to
the next one. Nesting one standalone multiplexer inside another is not
supported: detection picks zellij before tmux and screen, and tmux before
screen, even when the order is reversed.

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
