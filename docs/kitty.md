# kitty

kitty is a terminal with a built-in multiplexer. Amaca drives it through
`kitty @` when it runs inside kitty and no standalone multiplexer is available:
zellij, tmux and screen take precedence over kitty when more than one is
present. kitty, iTerm2 and WezTerm cannot be the outer terminal at the same
time, so their order in the chain does not matter
([docs/multiplexer.md](multiplexer.md)).

## Setup

Remote control must be enabled in `kitty.conf`:

```conf
allow_remote_control yes
```

Without it the probe fails and amaca prints a warning naming this setting.

## Check

`KITTY_WINDOW_ID` must be set and `kitty @ ls` must succeed. When the probe
fails, Amaca falls through to the next backend; with none reachable it exits
before fetching the ticket or creating the worktree.

## Layout

1. `kitty @ launch --type=window-dock --dock-edge=top --dock-size=2 --dock-skip-focus --no-response --cwd <worktree> -- <current exe> ꙮ <id> <title> <summary>`
   docks a 2-row window to the top edge of the current one. The dock runs the
   hidden `ꙮ` command with the ticket id, title and summary.
2. `--dock-size=2` is a row count, so the banner keeps its size whatever the
   layout is. `--dock-skip-focus` keeps the focus on the original window.
   `--no-response` keeps the new window id off the terminal.
3. A dock is shown while the window it is docked to is shown, and closes with
   it, so the banner lives as long as the pane where Claude runs.
4. Claude launches in the original window.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

The backend is written against kitty 0.50.0. It needs a kitty with the docks
feature (`--type=window-dock`).

## Rules

- Only the `kitty @` CLI. No direct socket protocol, no writing kitty config
  files.
- Keep `kitty @` calls inside `src/mux/kitty.rs`.
- Running Amaca twice on the same ticket currently adds another dock; the dock
  is not reused. Reuse is open.
