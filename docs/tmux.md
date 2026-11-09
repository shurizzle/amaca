# tmux

tmux is a standalone terminal multiplexer: it runs inside any terminal. When
Amaca runs inside tmux, tmux owns the panes even if the outer terminal has its
own multiplexer, so tmux takes precedence over screen, iTerm2 and WezTerm
([docs/multiplexer.md](multiplexer.md)).

## Check

`TMUX_PANE` must be set and `tmux list-panes` must succeed. When the probe
fails, Amaca falls back to WezTerm if it is available; otherwise it exits
before fetching the ticket or creating the worktree.

## Layout

1. `tmux split-window -v -b -l 2 -c <worktree> -t <TMUX_PANE> <current exe> ꙮ <id> <title> <summary>`
   splits the current pane vertically and places the new 2-row pane above it;
   that pane runs the hidden `ꙮ` command with the ticket id, title and summary.
2. `tmux select-pane -t <TMUX_PANE>` gives the focus back to the original
   pane.
3. Claude launches in the original pane.

`ꙮ` sets the terminal title to the ticket id, prints the ticket data and waits
forever, so the banner stays visible above Claude.

After changing directory into the ticket worktree, Amaca emits OSC 7
(`\x1b]7;file://<path>\x07`) so the terminal tracks the worktree as the current
directory.

## Rules

- Only the `tmux` CLI. No direct socket protocol, no writing tmux config
  files.
- Keep `tmux` calls inside `src/mux/tmux.rs`.
- Running Amaca twice on the same ticket currently adds another top pane; the
  split is not reused.
